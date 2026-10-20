use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    ramp_support::{client, transfer},
    types::wire::{encode_transaction, tests::transaction::native_transaction},
    *,
};
use serde_json::{json, Value};
use solana_signature::Signature;
use std::error::Error;
#[tokio::test]
async fn submission_encodes_native_transaction_and_empty_replay() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Devnet)?;
    let signature = Signature::from([8; 64]);
    let result = json!({"transfer":{"transferId":"transfer","state":"TRANSFER_STATE_SUBMITTED","solanaSignature":signature.to_string(),"expiresAt":"2026-10-19T12:00:00Z"}});
    queue(
        &client.http,
        vec![Ok(response(200, result.clone())), Ok(response(200, result))],
    )?;
    let transaction = native_transaction(false);
    let submitted = client
        .submit_transfer(SubmitRampTransferArgs {
            order_id: "saved".into(),
            transfer_id: "transfer".into(),
            signed_transaction: Some(transaction.clone()),
        })
        .await?;
    assert!(submitted.state == RampTransferState::Submitted { signature });
    client
        .submit_transfer(SubmitRampTransferArgs {
            order_id: "saved".into(),
            transfer_id: "transfer".into(),
            signed_transaction: None,
        })
        .await?;
    let test = client.http.test.lock().map_err(|_| "lock failed")?;
    let bodies = test
        .requests
        .iter()
        .map(|request| {
            request
                .body()
                .and_then(reqwest::Body::as_bytes)
                .ok_or("missing body")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let first: Value = serde_json::from_slice(bodies.first().ok_or("missing first body")?)?;
    let second: Value = serde_json::from_slice(bodies.get(1).ok_or("missing second body")?)?;
    assert_eq!(
        first,
        json!({"transferId":"transfer","signedTransaction":encode_transaction(&transaction)?})
    );
    assert_eq!(
        second,
        json!({"transferId":"transfer","signedTransaction":""})
    );
    Ok(())
}

#[tokio::test]
async fn submission_rejects_malformed_native_transactions_before_http() -> Result<(), Box<dyn Error>>
{
    let client = client(Network::Mainnet)?;
    let mut transaction = native_transaction(false);
    transaction.signatures.clear();
    assert!(matches!(
        client
            .submit_transfer(SubmitRampTransferArgs {
                order_id: "saved".into(),
                transfer_id: "transfer".into(),
                signed_transaction: Some(transaction),
            })
            .await,
        Err(SubmitRampTransferError::InvalidInput(_))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    Ok(())
}

fn args() -> SubmitRampTransferArgs {
    SubmitRampTransferArgs {
        order_id: "saved".into(),
        transfer_id: "transfer".into(),
        signed_transaction: None,
    }
}

#[tokio::test]
async fn positional_submission_objects_are_rejected_without_retry() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!([transfer("transfer")]),
        json!({"transfer":["transfer","TRANSFER_STATE_PREPARED",null,"2026-10-19T12:00:00Z"]}),
    ] {
        for body in [payload.clone(), json!({"data":payload})] {
            let ramp = client(Network::Mainnet)?;
            queue(&ramp.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                ramp.submit_transfer(args()).await,
                Err(SubmitRampTransferError::InvalidResponse(
                    "malformed Ramp submission"
                ))
            ));
            assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn submitted_transfer_aliases_and_null_signature_remain_accepted(
) -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Mainnet)?;
    queue(
        &ramp.http,
        vec![Ok(response(
            200,
            json!({"data":{"transfer":{
                "transfer_id":"transfer","state":"TRANSFER_STATE_PREPARED","solana_signature":null,
                "expires_at":"2026-10-19T12:00:00Z"
            }}}),
        ))],
    )?;
    let transfer = ramp.submit_transfer(args()).await?;
    assert!(transfer.state == RampTransferState::Prepared);
    Ok(())
}

#[tokio::test]
async fn submission_works_without_configuration_and_maps_preconditions_once(
) -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    let precondition: ::http::Response<String> = ::http::Response::builder()
        .status(503)
        .header("grpc-status", "9")
        .body(json!({"code":"UNAVAILABLE"}).to_string())?;
    let unavailable: ::http::Response<String> = ::http::Response::builder()
        .status(400)
        .header("grpc-status", "14")
        .body("{}".into())?;
    queue(
        &ramp.http,
        vec![
            Ok(response(200, json!({"transfer":transfer("transfer")}))),
            Ok(precondition.into()),
            Ok(unavailable.into()),
            Ok(response(400, json!({}))),
            Err(HttpError::RequestFailed),
        ],
    )?;
    ramp.submit_transfer(args()).await?;
    assert!(matches!(
        ramp.submit_transfer(args()).await,
        Err(SubmitRampTransferError::PreconditionFailed)
    ));
    assert!(matches!(
        ramp.submit_transfer(args()).await,
        Err(SubmitRampTransferError::ServiceUnavailable)
    ));
    assert!(matches!(
        ramp.submit_transfer(args()).await,
        Err(SubmitRampTransferError::UnexpectedServiceFailure)
    ));
    assert!(matches!(
        ramp.submit_transfer(args()).await,
        Err(SubmitRampTransferError::RequestFailed)
    ));
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 5);
    Ok(())
}
#[tokio::test]
async fn submission_rejects_empty_identifiers_before_http() -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    for order_is_empty in [true, false] {
        let mut request = args();
        if order_is_empty {
            request.order_id = " ".into();
        } else {
            request.transfer_id = " ".into();
        }
        assert!(matches!(
            ramp.submit_transfer(request).await,
            Err(SubmitRampTransferError::InvalidInput(_))
        ));
    }
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    Ok(())
}
#[tokio::test]
async fn submission_rejects_malformed_or_uncorrelated_receipts_without_retry(
) -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    for body in [
        json!({"transfer":transfer("wrong")}),
        json!({"transfer":{"transferId":"transfer","state":"TRANSFER_STATE_SUBMITTED","expiresAt":"today"}}),
        json!({"transfer":{"transferId":"transfer","state":"TRANSFER_STATE_SUBMITTED","expiresAt":"today","solanaSignature":"bad"}}),
        json!({"transfer":{"transferId":"transfer","state":{"TRANSFER_STATE_PREPARED":null},"expiresAt":"today"}}),
        json!({"transfer":{}}),
    ] {
        queue(&ramp.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            ramp.submit_transfer(args()).await,
            Err(SubmitRampTransferError::InvalidResponse(_))
        ));
    }
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 5);
    Ok(())
}
