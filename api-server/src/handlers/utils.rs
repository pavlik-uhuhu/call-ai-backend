use axum::response::{IntoResponse, Json};
use http::StatusCode;
use serde::Serialize;

pub type RequestResult<T> = Result<Response<T>, crate::error::Error>;
pub type AppResponse<T> = Response<T>;

#[derive(Debug)]
pub struct Response<T> {
    status: StatusCode,
    payload: T,
}

impl<T> Response<T> {
    pub fn new(status: StatusCode, payload: T) -> Self {
        Self { status, payload }
    }

    #[cfg(test)]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    #[cfg(test)]
    pub fn payload(&self) -> &T {
        &self.payload
    }
}

impl<T: Serialize> IntoResponse for Response<T> {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self.payload)).into_response()
    }
}
