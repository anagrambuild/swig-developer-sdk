use std::collections::VecDeque;

use reqwest::{Request, Response};

use crate::http::HttpError;

#[derive(Default)]
pub(crate) struct TestTransport {
    pub(crate) requests: Vec<Request>,
    pub(crate) responses: VecDeque<Result<Response, HttpError>>,
}
