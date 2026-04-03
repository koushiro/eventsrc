use bytes::Bytes;
use eventsrc_client::{
    error::{Error, ErrorKind},
    oneshot::{EventSource, EventSourceExt as _},
};
use futures_util::{FutureExt, StreamExt, stream};
use reqwest::{Body, Response};

#[test]
fn consumes_valid_sse_body_stream() {
    let bytes = b"data: hello\n\ndata: world\n\n".to_vec();
    let stream = stream::iter([Ok::<Bytes, Error>(bytes.into())]);
    let stream = EventSource::new(stream);

    let items = stream.collect::<Vec<_>>().now_or_never().expect("test stream should not pend");
    let items = items.into_iter().map(Result::unwrap).collect::<Vec<_>>();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].event(), "message");
    assert_eq!(items[0].data(), "hello");
    assert_eq!(items[0].id(), "");
    assert_eq!(items[1].event(), "message");
    assert_eq!(items[1].data(), "world");
    assert_eq!(items[1].id(), "");
}

#[test]
fn response_extension_builds_event_source() {
    let response = build_mock_response(
        http::StatusCode::OK,
        Some("text/event-stream"),
        b"data: hello world\n\n".to_vec(),
    );

    let stream = response.event_source().unwrap();
    let items = stream.collect::<Vec<_>>().now_or_never().expect("test stream should not pend");
    let items = items.into_iter().map(Result::unwrap).collect::<Vec<_>>();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].data(), "hello world");
}

#[test]
fn response_extension_rejects_invalid_status() {
    let response = build_mock_response(
        http::StatusCode::BAD_REQUEST,
        Some("text/event-stream"),
        b"data: ignored\n\n".to_vec(),
    );

    let error = response.event_source().unwrap_err();

    match error.kind() {
        ErrorKind::InvalidResponse => assert_eq!(error.message(), "invalid status code"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn response_extension_rejects_invalid_content_type() {
    let response = build_mock_response(
        http::StatusCode::OK,
        Some("application/json"),
        b"data: ignored\n\n".to_vec(),
    );

    let error = response.event_source().unwrap_err();

    match error.kind() {
        ErrorKind::InvalidResponse => assert_eq!(error.message(), "invalid SSE content type"),
        other => panic!("unexpected error: {other:?}"),
    }
}

fn build_mock_response(
    status: http::StatusCode,
    content_type: Option<&str>,
    body: Vec<u8>,
) -> Response {
    let mut builder = http::Response::builder().status(status);

    if let Some(content_type) = content_type {
        builder = builder.header(http::header::CONTENT_TYPE, content_type);
    }

    Response::from(builder.body(Body::from(body)).unwrap())
}
