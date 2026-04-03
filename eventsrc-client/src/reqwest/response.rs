use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_core::stream::{BoxStream, Stream};

use super::SSE_CONTENT_TYPE;
use crate::{
    error::{Error, ErrorKind},
    replayable::BoxBodyStream,
};

pub(crate) struct ResponseBodyStream {
    inner: BoxStream<'static, Result<Bytes, ::reqwest::Error>>,
}

impl Stream for ResponseBodyStream {
    type Item = Result<Bytes, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(Error::new(
                ErrorKind::InvalidResponse,
                "invalid response body stream",
            )
            .set_source(error)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub(crate) fn validate_response(response: &::reqwest::Response) -> Result<(), Error> {
    if response.status() != ::reqwest::StatusCode::OK {
        return Err(Error::new(ErrorKind::InvalidResponse, "invalid status code")
            .with_context("status_code", response.status()));
    }

    let content_type = response
        .headers()
        .get(::reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    match content_type {
        Some(value) if value.contains(SSE_CONTENT_TYPE) => Ok(()),
        Some(value) => Err(Error::new(ErrorKind::InvalidResponse, "invalid SSE content type")
            .with_context("content_type", value)),
        None => Err(Error::new(ErrorKind::InvalidResponse, "missing SSE content type")),
    }
}

pub(crate) fn into_body_stream(response: ::reqwest::Response) -> BoxBodyStream {
    let body = ResponseBodyStream { inner: Box::pin(response.bytes_stream()) };
    Box::pin(body)
}
