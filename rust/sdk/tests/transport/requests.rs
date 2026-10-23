use std::{error::Error, time::Duration};

use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Body, Method, Response,
};

use crate::{
    http::{HttpClient, HttpError},
    ClientError, HttpConfig,
};

fn client(max_retries: u32) -> Result<HttpClient, ClientError> {
    let mut config = HttpConfig::new("test-only-secret".into());
    config.base_url = Some("https://api.example/prefix/".into());
    config.retry_options.max_retries = max_retries;
    config.retry_options.initial_delay = Duration::ZERO;
    HttpClient::new(config)
}

fn response(status: u16, body: Vec<u8>) -> Result<Response, http::Error> {
    Ok(http::Response::builder().status(status).body(body)?.into())
}

#[tokio::test]
async fn positional_rpc_errors_do_not_override_http_status() -> Result<(), Box<dyn Error>> {
    for body in [
        br#"[14]"#.to_vec(),
        br#"[{"code":14}]"#.to_vec(),
        br#"{"error":[14]}"#.to_vec(),
        br#"{"code":3,"code":14}"#.to_vec(),
        br#"{"error":{"code":3,"code":14}}"#.to_vec(),
    ] {
        let client = client(0)?;
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .push_back(Ok(response(400, body)?));
        assert!(matches!(
            client.get("/wallet/policies").await,
            Err(HttpError::UnexpectedStatus(400))
        ));
        let transport = client.test.lock().map_err(|_| "poisoned transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn malformed_rpc_envelope_preserves_direct_object_fallback() -> Result<(), Box<dyn Error>> {
    let client = client(0)?;
    client
        .test
        .lock()
        .map_err(|_| "poisoned transport")?
        .responses
        .push_back(Ok(response(400, br#"{"error":[14],"code":3}"#.to_vec())?));
    assert!(matches!(
        client.get("/wallet/policies").await,
        Err(HttpError::InvalidArgument)
    ));
    assert_eq!(
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .requests
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn invalid_urls_never_send_or_consume_a_response() -> Result<(), Box<dyn Error>> {
    let client = client(3)?;
    client
        .test
        .lock()
        .map_err(|_| "poisoned transport")?
        .responses
        .push_back(Ok(response(200, b"must remain queued".to_vec())?));

    for path in [
        "https://evil.example",
        "//evil.example",
        "/\\evil.example",
        "/path#fragment",
        "/https://evil.example",
        "/https://[",
        "/https://user:private@api.example",
    ] {
        for result in [
            client.get(path).await,
            client.post(path, b"{}".to_vec()).await,
            client.post_with_retry(path, b"{}".to_vec()).await,
        ] {
            assert!(
                matches!(result, Err(HttpError::InvalidUrl)),
                "{path}: {result:?}"
            );
        }
        let transport = client.test.lock().map_err(|_| "poisoned transport")?;
        assert!(transport.requests.is_empty());
        assert_eq!(transport.responses.len(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn wrappers_preserve_response_bytes_and_build_authenticated_requests(
) -> Result<(), Box<dyn Error>> {
    let client = client(2)?;
    let payloads = [
        b"not JSON\x00\xff".to_vec(),
        br#"{"data":{"id":"policy"},"extra":true}"#.to_vec(),
        Vec::new(),
    ];
    {
        let mut transport = client.test.lock().map_err(|_| "poisoned transport")?;
        for payload in &payloads {
            transport
                .responses
                .push_back(Ok(response(200, payload.clone())?));
        }
    }
    let body = br#"{"amount":"1","idempotencyKey":"same-request"}"#.to_vec();
    assert_eq!(client.get("/wallet/policies?id=a%2Fb").await?, payloads[0]);
    assert_eq!(
        client.post("/wallet/prepare", body.clone()).await?,
        payloads[1]
    );
    assert_eq!(
        client
            .post_with_retry("/paymaster/sponsor", body.clone())
            .await?,
        payloads[2]
    );

    let transport = client.test.lock().map_err(|_| "poisoned transport")?;
    assert_eq!(transport.requests.len(), 3);
    for (request, (method, path, expected_body)) in transport.requests.iter().zip([
        (Method::GET, "/prefix/wallet/policies?id=a%2Fb", None),
        (
            Method::POST,
            "/prefix/wallet/prepare",
            Some(body.as_slice()),
        ),
        (
            Method::POST,
            "/prefix/paymaster/sponsor",
            Some(body.as_slice()),
        ),
    ]) {
        assert_eq!(request.method(), method);
        assert_eq!(request.url().as_str(), format!("https://api.example{path}"));
        let authorization = request
            .headers()
            .get(AUTHORIZATION)
            .ok_or("authorization missing")?;
        assert_eq!(authorization, "Bearer test-only-secret");
        assert!(authorization.is_sensitive());
        assert!(!format!("{:?}", request.headers()).contains("test-only-secret"));
        assert_eq!(request.body().and_then(Body::as_bytes), expected_body);
        if method == Method::POST {
            assert_eq!(
                request.headers().get(CONTENT_TYPE),
                Some(&"application/json".parse()?)
            );
        } else {
            assert!(!request.headers().contains_key(CONTENT_TYPE));
        }
    }
    Ok(())
}

#[tokio::test]
async fn ordinary_post_never_retries_server_or_transport_failures() -> Result<(), Box<dyn Error>> {
    for failure in [
        Ok(response(500, Vec::new())?),
        Err(HttpError::Timeout),
        Err(HttpError::RequestFailed),
    ] {
        let client = client(3)?;
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .extend([failure, Ok(response(200, b"must remain queued".to_vec())?)]);
        assert!(matches!(
            client.post("/wallet/create", b"{}".to_vec()).await,
            Err(HttpError::UnexpectedStatus(500) | HttpError::Timeout | HttpError::RequestFailed)
        ));
        let transport = client.test.lock().map_err(|_| "poisoned transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert_eq!(transport.responses.len(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn get_and_opted_in_post_replay_transport_and_server_failures() -> Result<(), Box<dyn Error>>
{
    for use_post in [false, true] {
        let client = client(3)?;
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .extend([
                Err(HttpError::Timeout),
                Err(HttpError::RequestFailed),
                Ok(response(502, b"bad gateway".to_vec())?),
                Ok(response(200, b"complete".to_vec())?),
            ]);
        let payload = br#"{"transaction":"exact serialized bytes","idempotencyKey":"request-1"}"#;
        let result = if use_post {
            client
                .post_with_retry("/paymaster/sponsor?network=devnet", payload.to_vec())
                .await
        } else {
            client.get("/wallet/policies?limit=1").await
        };
        assert_eq!(result?, b"complete");
        let transport = client.test.lock().map_err(|_| "poisoned transport")?;
        assert_eq!(transport.requests.len(), 4);
        let first = transport.requests.first().ok_or("request missing")?;
        for request in &transport.requests {
            assert_eq!(request.method(), first.method());
            assert_eq!(request.url(), first.url());
            assert_eq!(request.headers(), first.headers());
            assert_eq!(
                request.body().and_then(Body::as_bytes),
                if use_post {
                    Some(payload.as_slice())
                } else {
                    None
                }
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn server_statuses_retry_regardless_of_structured_error_code() -> Result<(), Box<dyn Error>> {
    for use_post in [false, true] {
        for status in [500, 501, 502, 503, 504, 599] {
            for code in [3, 5, 6, 7, 8, 9, 10, 14, 16] {
                for in_header in [false, true] {
                    let client = client(3)?;
                    let mut builder = http::Response::builder().status(status);
                    if in_header {
                        builder = builder.header("grpc-status", code.to_string());
                    }
                    let body = if in_header {
                        br#"{"code":14}"#.to_vec()
                    } else {
                        format!(r#"{{"code":{code}}}"#).into_bytes()
                    };
                    client
                        .test
                        .lock()
                        .map_err(|_| "poisoned transport")?
                        .responses
                        .extend([
                            Ok(builder.body(body)?.into()),
                            Ok(response(200, b"retried".to_vec())?),
                        ]);
                    let result = if use_post {
                        client
                            .post_with_retry("/paymaster/sponsor", b"{}".to_vec())
                            .await
                    } else {
                        client.get("/wallet/policies").await
                    };
                    assert_eq!(result?, b"retried");
                    let transport = client.test.lock().map_err(|_| "poisoned transport")?;
                    assert_eq!(transport.requests.len(), 2);
                    assert!(transport.responses.is_empty());
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn rpc_headers_preserve_safe_errors_after_the_retry_budget() -> Result<(), Box<dyn Error>> {
    for use_post in [false, true] {
        for status in [200, 500, 503] {
            for code in [6, 9] {
                let client = client(1)?;
                let body = Body::wrap_stream(futures_util::stream::iter([
                    Ok(b"private upstream details".to_vec()),
                    Err(std::io::Error::other("private upstream details")),
                ]));
                let failed = http::Response::builder()
                    .status(status)
                    .header("grpc-status", code.to_string())
                    .body(body)?;
                let repeated = http::Response::builder()
                    .status(status)
                    .header("grpc-status", code.to_string())
                    .body(Vec::new())?;
                client
                    .test
                    .lock()
                    .map_err(|_| "poisoned transport")?
                    .responses
                    .extend([
                        Ok(failed.into()),
                        Ok(repeated.into()),
                        Ok(response(200, b"must remain queued".to_vec())?),
                    ]);
                let result = if use_post {
                    client
                        .post_with_retry("/paymaster/sponsor", b"{}".to_vec())
                        .await
                } else {
                    client.get("/wallet/policies").await
                };
                let error = result.err().ok_or("expected a terminal RPC error")?;
                assert!(matches!(
                    (code, &error),
                    (6, HttpError::AlreadyExists) | (9, HttpError::FailedPrecondition)
                ));
                assert!(!format!("{error:?} {error}").contains("private upstream details"));
                let transport = client.test.lock().map_err(|_| "poisoned transport")?;
                let expected_requests = if status == 200 { 1 } else { 2 };
                assert_eq!(transport.requests.len(), expected_requests);
                assert_eq!(transport.responses.len(), 3 - expected_requests);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn retries_stop_at_the_configured_attempt_count() -> Result<(), Box<dyn Error>> {
    for max_retries in [0, 2] {
        for use_post in [false, true] {
            let client = client(max_retries)?;
            {
                let mut transport = client.test.lock().map_err(|_| "poisoned transport")?;
                for _ in 0..max_retries + 2 {
                    transport
                        .responses
                        .push_back(Ok(response(503, Vec::new())?));
                }
            }
            let result = if use_post {
                client
                    .post_with_retry("/paymaster/sponsor", b"{}".to_vec())
                    .await
            } else {
                client.get("/wallet/policies").await
            };
            assert!(matches!(result, Err(HttpError::Unavailable)));
            let transport = client.test.lock().map_err(|_| "poisoned transport")?;
            assert_eq!(transport.requests.len(), (max_retries + 1) as usize);
            assert_eq!(transport.responses.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn redirects_and_client_errors_do_not_retry_even_with_retryable_rpc_codes(
) -> Result<(), Box<dyn Error>> {
    for status in [301, 302, 307, 308, 400, 401, 403, 404, 409, 429] {
        for use_post in [false, true] {
            let client = client(3)?;
            client
                .test
                .lock()
                .map_err(|_| "poisoned transport")?
                .responses
                .extend([
                    Ok(response(status, br#"{"code":14}"#.to_vec())?),
                    Ok(response(200, b"must remain queued".to_vec())?),
                ]);
            let result = if use_post {
                client
                    .post_with_retry("/paymaster/sponsor", b"{}".to_vec())
                    .await
            } else {
                client.get("/wallet/policies").await
            };
            if status == 429 {
                assert!(matches!(result, Err(HttpError::ResourceExhausted)));
            } else {
                assert!(matches!(result, Err(HttpError::Unavailable)));
            }
            let transport = client.test.lock().map_err(|_| "poisoned transport")?;
            assert_eq!(transport.requests.len(), 1);
            assert_eq!(transport.responses.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn rate_limits_override_conflicting_codes_and_error_body_failures(
) -> Result<(), Box<dyn Error>> {
    for use_post in [false, true] {
        for header in [
            None,
            Some("3"),
            Some("5"),
            Some("6"),
            Some("7"),
            Some("9"),
            Some("14"),
            Some("16"),
        ] {
            for body_case in 0..4 {
                let client = client(3)?;
                let body = match body_case {
                    0 => {
                        Body::from(br#"{"code":16,"message":"private upstream details"}"#.to_vec())
                    }
                    1 => Body::from(br#"{"error":{"code":"ALREADY_EXISTS"}}"#.to_vec()),
                    2 => Body::from(vec![b'x'; 64 * 1024 + 1]),
                    _ => Body::wrap_stream(futures_util::stream::iter([Err::<Vec<u8>, _>(
                        std::io::Error::other("private upstream details"),
                    )])),
                };
                let mut builder = http::Response::builder().status(429);
                if let Some(code) = header {
                    builder = builder.header("grpc-status", code);
                }
                client
                    .test
                    .lock()
                    .map_err(|_| "poisoned transport")?
                    .responses
                    .extend([
                        Ok(builder.body(body)?.into()),
                        Ok(response(200, b"must remain queued".to_vec())?),
                    ]);
                let result = if use_post {
                    client
                        .post_with_retry("/paymaster/sponsor", b"{}".to_vec())
                        .await
                } else {
                    client.get("/wallet/policies").await
                };
                let error = result.err().ok_or("expected rate limit failure")?;
                assert!(matches!(error, HttpError::ResourceExhausted));
                assert!(!format!("{error:?} {error}").contains("private upstream details"));
                assert!(error.source().is_none());
                let transport = client.test.lock().map_err(|_| "poisoned transport")?;
                assert_eq!(transport.requests.len(), 1);
                assert_eq!(transport.responses.len(), 1);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn recognized_rpc_codes_use_the_same_mapping_in_supported_body_shapes(
) -> Result<(), Box<dyn Error>> {
    for (code, name, expected) in [
        (3, "INVALID_ARGUMENT", "invalid argument"),
        (5, "NOT_FOUND", "resource not found"),
        (6, "ALREADY_EXISTS", "resource already exists"),
        (7, "PERMISSION_DENIED", "permission denied"),
        (8, "RESOURCE_EXHAUSTED", "resource exhausted"),
        (9, "FAILED_PRECONDITION", "precondition failed"),
        (10, "ABORTED", "operation aborted"),
        (14, "UNAVAILABLE", "service unavailable"),
        (16, "UNAUTHENTICATED", "authentication required"),
    ] {
        for body in [
            format!(r#"{{"code":{code},"message":"private upstream text"}}"#),
            format!(r#"{{"code":"{code}"}}"#),
            format!(r#"{{"code":"{name}"}}"#),
            format!(r#"{{"error":{{"code":{code}}}}}"#),
            format!(r#"{{"error":{{"code":"{name}"}}}}"#),
        ] {
            let client = client(0)?;
            client
                .test
                .lock()
                .map_err(|_| "poisoned transport")?
                .responses
                .push_back(Ok(response(400, body.into_bytes())?));
            let result = client.post("/wallet/prepare", Vec::new()).await;
            assert!(result.is_err());
            if let Err(error) = result {
                assert_eq!(error.to_string(), expected);
                assert!(!format!("{error:?}").contains("private upstream text"));
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn status_fallback_does_not_invent_backend_codes() -> Result<(), Box<dyn Error>> {
    for body in [
        b"not JSON".to_vec(),
        b"{}".to_vec(),
        br#"{"message":"POLICY_NOT_FOUND"}"#.to_vec(),
        br#"{"code":2}"#.to_vec(),
        br#"{"code":"WALLET_NOT_FOUND"}"#.to_vec(),
        br#"{"code":"POLICY_NOT_FOUND"}"#.to_vec(),
        br#"{"code":"AUTHENTICATION_FAILED"}"#.to_vec(),
        br#"{"code":"INVALID_INPUT"}"#.to_vec(),
        br#"{"code":"RATE_LIMITED"}"#.to_vec(),
        br#"{"code":"SERVICE_UNAVAILABLE"}"#.to_vec(),
    ] {
        let client = client(0)?;
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .push_back(Ok(response(418, body)?));
        assert!(matches!(
            client.post("/wallet/prepare", Vec::new()).await,
            Err(HttpError::UnexpectedStatus(418))
        ));
    }
    for (status, expected) in [
        (401, "authentication required"),
        (403, "permission denied"),
        (404, "unexpected HTTP status 404"),
        (429, "resource exhausted"),
        (503, "service unavailable"),
        (504, "service unavailable"),
        (400, "unexpected HTTP status 400"),
        (409, "unexpected HTTP status 409"),
        (500, "unexpected HTTP status 500"),
    ] {
        let client = client(0)?;
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .push_back(Ok(response(status, b"not JSON".to_vec())?));
        let result = client.post("/wallet/prepare", Vec::new()).await;
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.to_string(), expected);
        }
    }
    Ok(())
}

#[tokio::test]
async fn grpc_status_takes_precedence_even_for_http_success() -> Result<(), Box<dyn Error>> {
    for status in [200, 400, 503] {
        for (code, expected) in [("9", "precondition failed"), ("14", "service unavailable")] {
            let client = client(0)?;
            let response = http::Response::builder()
                .status(status)
                .header("grpc-status", code)
                .body(br#"{"code":16}"#.to_vec())?;
            client
                .test
                .lock()
                .map_err(|_| "poisoned transport")?
                .responses
                .push_back(Ok(response.into()));
            let result = client.post("/wallet/prepare", Vec::new()).await;
            assert!(result.is_err());
            if let Err(error) = result {
                assert_eq!(error.to_string(), expected);
            }
        }
    }
    for code in ["0", "unknown"] {
        let client = client(0)?;
        let response = http::Response::builder()
            .header("grpc-status", code)
            .body(br#"{"code":16}"#.to_vec())?;
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .push_back(Ok(response.into()));
        assert_eq!(client.get("/wallet/policies").await?, br#"{"code":16}"#);
    }
    Ok(())
}

#[tokio::test]
async fn response_body_limits_apply_to_success_and_error_streams() -> Result<(), Box<dyn Error>> {
    for (status, limit) in [(200, 8 * 1024 * 1024), (400, 64 * 1024)] {
        let boundary_client = client(0)?;
        boundary_client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .push_back(Ok(response(status, vec![b'x'; limit])?));
        let result = boundary_client.post("/wallet/prepare", Vec::new()).await;
        if status == 200 {
            assert_eq!(result?.len(), limit);
        } else {
            assert!(matches!(result, Err(HttpError::UnexpectedStatus(400))));
        }

        for streamed in [false, true] {
            let client = client(0)?;
            let body = if streamed {
                Body::wrap_stream(futures_util::stream::iter([
                    Ok::<_, std::io::Error>(vec![b'x'; limit]),
                    Ok(vec![b'x']),
                ]))
            } else {
                Body::from(vec![b'x'; limit + 1])
            };
            let response = http::Response::builder().status(status).body(body)?;
            client
                .test
                .lock()
                .map_err(|_| "poisoned transport")?
                .responses
                .push_back(Ok(response.into()));
            assert!(matches!(
                client.post("/wallet/prepare", Vec::new()).await,
                Err(HttpError::ResponseTooLarge)
            ));
            assert_eq!(
                client
                    .test
                    .lock()
                    .map_err(|_| "poisoned transport")?
                    .requests
                    .len(),
                1
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn body_read_failure_is_a_transport_error_not_a_decoding_error() -> Result<(), Box<dyn Error>>
{
    for status in [200, 400] {
        let client = client(0)?;
        let body = Body::wrap_stream(futures_util::stream::iter([
            Ok(vec![b'{']),
            Err(std::io::Error::other("test-only stream failure")),
        ]));
        let response = http::Response::builder().status(status).body(body)?;
        client
            .test
            .lock()
            .map_err(|_| "poisoned transport")?
            .responses
            .push_back(Ok(response.into()));
        assert!(matches!(
            client.post("/wallet/prepare", Vec::new()).await,
            Err(HttpError::RequestFailed)
        ));
    }
    Ok(())
}

#[tokio::test]
async fn body_failures_retry_according_to_the_received_status() -> Result<(), Box<dyn Error>> {
    for use_post in [false, true] {
        for status in [200, 400, 503] {
            for too_large in [false, true] {
                let client = client(1)?;
                let limit = if status == 200 {
                    8 * 1024 * 1024
                } else {
                    64 * 1024
                };
                let body = if too_large {
                    Body::from(vec![b'x'; limit + 1])
                } else {
                    Body::wrap_stream(futures_util::stream::iter([Err::<Vec<u8>, _>(
                        std::io::Error::other("test-only stream failure"),
                    )]))
                };
                let failed = http::Response::builder().status(status).body(body)?;
                client
                    .test
                    .lock()
                    .map_err(|_| "poisoned transport")?
                    .responses
                    .extend([Ok(failed.into()), Ok(response(200, b"retried".to_vec())?)]);

                let result = if use_post {
                    client
                        .post_with_retry("/paymaster/sponsor", b"{}".to_vec())
                        .await
                } else {
                    client.get("/wallet/policies").await
                };
                let transport = client.test.lock().map_err(|_| "poisoned transport")?;
                if status == 400 || (status == 200 && too_large) {
                    if too_large {
                        assert!(matches!(result, Err(HttpError::ResponseTooLarge)));
                    } else {
                        assert!(matches!(result, Err(HttpError::RequestFailed)));
                    }
                    assert_eq!(transport.requests.len(), 1);
                    assert_eq!(transport.responses.len(), 1);
                } else {
                    assert_eq!(result?, b"retried");
                    assert_eq!(transport.requests.len(), 2);
                    assert!(transport.responses.is_empty());
                }
            }
        }
    }
    Ok(())
}
