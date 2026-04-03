use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use eventsrc_client::{
    error::{Error, ErrorKind},
    replayable::{
        ConstantBackoff, EventSourceExt as _, NeverRetry, RetryCause, RetryContext, RetryPolicy,
    },
};
use futures_util::{StreamExt, poll, stream};
use reqwest::Url;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

#[tokio::test]
async fn reconnects_and_yields_events_across_connections() {
    let server = TestServer::spawn(vec![
        ScriptedResponse::sse("data: first\n\n"),
        ScriptedResponse::sse("data: second\n\n"),
    ])
    .await;

    let mut source = test_client()
        .get(server.url())
        .event_source()
        .unwrap()
        .with_retry_policy(ConstantBackoff::new(Duration::ZERO));

    let first = source.next().await.unwrap().unwrap();
    let second = source.next().await.unwrap().unwrap();

    assert_eq!(first.event(), "message");
    assert_eq!(first.data(), "first");
    assert_eq!(first.id(), "");
    assert_eq!(second.event(), "message");
    assert_eq!(second.data(), "second");
    assert_eq!(second.id(), "");
    assert_eq!(server.request_count(), 2);
}

#[tokio::test]
async fn propagates_last_event_id_on_reconnect() {
    let server = TestServer::spawn(vec![
        ScriptedResponse::sse("id: 42\ndata: first\n\n"),
        ScriptedResponse::sse("data: second\n\n"),
    ])
    .await;

    let mut source = test_client()
        .get(server.url())
        .event_source()
        .unwrap()
        .with_retry_policy(ConstantBackoff::new(Duration::ZERO));

    let first = source.next().await.unwrap().unwrap();
    let second = source.next().await.unwrap().unwrap();

    assert_eq!(first.data(), "first");
    assert_eq!(first.event(), "message");
    assert_eq!(first.id(), "42");
    assert_eq!(second.data(), "second");
    assert_eq!(second.event(), "message");
    assert_eq!(second.id(), "42");
    assert_eq!(source.last_event_id(), Some("42"));

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].to_ascii_lowercase().contains("get /events http/1.1"));
    assert!(requests[1].to_ascii_lowercase().contains("last-event-id: 42"));
}

#[tokio::test(start_paused = true)]
async fn applies_server_retry_directive_to_reconnect_delay() {
    let server = TestServer::spawn(vec![
        ScriptedResponse::sse("retry: 50\ndata: first\n\n"),
        ScriptedResponse::sse("data: second\n\n"),
    ])
    .await;

    let mut source = test_client()
        .get(server.url())
        .event_source()
        .unwrap()
        .with_retry_policy(ConstantBackoff::new(Duration::ZERO));

    let first = source.next().await.unwrap().unwrap();
    assert_eq!(first.data(), "first");
    assert_eq!(first.event(), "message");
    assert_eq!(first.id(), "");
    assert_eq!(server.request_count(), 1);

    let mut next = Box::pin(source.next());
    assert!(poll!(next.as_mut()).is_pending());

    tokio::task::yield_now().await;
    assert_eq!(server.request_count(), 1);

    tokio::time::advance(Duration::from_millis(49)).await;
    tokio::task::yield_now().await;
    assert_eq!(server.request_count(), 1);
    assert!(poll!(next.as_mut()).is_pending());

    tokio::time::advance(Duration::from_millis(1)).await;
    let second = next.await.unwrap().unwrap();

    assert_eq!(second.data(), "second");
    assert_eq!(second.event(), "message");
    assert_eq!(second.id(), "");
    assert_eq!(server.request_count(), 2);
}

#[tokio::test]
async fn surfaces_parse_errors_without_leaking_core_stream_error() {
    let bytes = vec![b'd', b'a', b't', b'a', b':', b' ', 0xff, b'\n', b'\n'];
    let response = ScriptedResponse::sse_bytes(bytes);
    let server = TestServer::spawn(vec![response]).await;

    let mut source = test_client()
        .get(server.url())
        .event_source()
        .unwrap()
        .with_retry_policy(ConstantBackoff::new(Duration::ZERO));

    let error = source.next().await.unwrap().unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Protocol);
    assert!(
        error.message().contains("invalid UTF-8 in SSE data field"),
        "unexpected error: {error:?}"
    );
}

#[tokio::test]
async fn returns_terminal_error_after_retry_exhaustion() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        }
    });

    let url = Url::parse(&format!("http://{addr}/events")).unwrap();
    let mut source = test_client()
        .get(url)
        .event_source()
        .unwrap()
        .with_retry_policy(ConstantBackoff::new(Duration::ZERO).with_max_retries(1));

    let error = source.next().await.unwrap().unwrap_err();
    server.abort();

    assert_eq!(error.kind(), ErrorKind::Transport);
}

#[tokio::test]
async fn rejects_invalid_status_without_retrying() {
    let server = TestServer::spawn(vec![ScriptedResponse {
        status_line: "400 Bad Request",
        content_type: Some("text/event-stream"),
        body: b"data: ignored\n\n".to_vec(),
    }])
    .await;

    let mut source = test_client()
        .get(server.url())
        .event_source()
        .unwrap()
        .with_retry_policy(ConstantBackoff::new(Duration::ZERO));

    let error = source.next().await.unwrap().unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidResponse);
    assert_eq!(error.message(), "invalid status code");
    assert_context_contains(&error, "status_code", "400 Bad Request");

    assert_eq!(server.request_count(), 1);
}

#[tokio::test]
async fn rejects_invalid_content_type_without_retrying() {
    let server = TestServer::spawn(vec![ScriptedResponse {
        status_line: "200 OK",
        content_type: Some("application/json"),
        body: b"{\"ok\":true}".to_vec(),
    }])
    .await;

    let mut source = test_client()
        .get(server.url())
        .event_source()
        .unwrap()
        .with_retry_policy(ConstantBackoff::new(Duration::ZERO));

    let error = source.next().await.unwrap().unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidResponse);
    assert_eq!(error.message(), "invalid SSE content type");
    assert_context_contains(&error, "content_type", "application/json");

    assert_eq!(server.request_count(), 1);
}

#[tokio::test]
async fn passes_disconnect_context_to_custom_retry_policy() {
    let contexts = Arc::new(Mutex::new(Vec::new()));
    let server = TestServer::spawn(vec![
        ScriptedResponse::sse("data: first\n\n"),
        ScriptedResponse::sse("data: second\n\n"),
    ])
    .await;

    let mut source =
        test_client().get(server.url()).event_source().unwrap().with_retry_policy(
            RecordingPolicy::new(Arc::clone(&contexts), Some(Duration::ZERO), None),
        );

    let first = source.next().await.unwrap().unwrap();
    let second = source.next().await.unwrap().unwrap();

    assert_eq!(first.data(), "first");
    assert_eq!(second.data(), "second");
    let contexts = contexts.lock().unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].cause, RetryCause::Disconnect);
    assert_eq!(contexts[0].failure_streak, 0);
    assert_eq!(contexts[0].server_retry, None);
}

#[tokio::test]
async fn passes_connect_error_context_to_custom_retry_policy() {
    let contexts = Arc::new(Mutex::new(Vec::new()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        }
    });

    let url = Url::parse(&format!("http://{addr}/events")).unwrap();
    let mut source =
        test_client()
            .get(url)
            .event_source()
            .unwrap()
            .with_retry_policy(RecordingPolicy::new(
                Arc::clone(&contexts),
                Some(Duration::ZERO),
                Some(1),
            ));

    let error = source.next().await.unwrap().unwrap_err();
    server.abort();

    assert_eq!(error.kind(), ErrorKind::Transport);

    let contexts = contexts.lock().unwrap();
    assert_eq!(contexts.len(), 2);
    assert_eq!(contexts[0].cause, RetryCause::ConnectError);
    assert_eq!(contexts[0].failure_streak, 1);
    assert_eq!(contexts[0].server_retry, None);
    assert_eq!(contexts[1].cause, RetryCause::ConnectError);
    assert_eq!(contexts[1].failure_streak, 2);
    assert_eq!(contexts[1].server_retry, None);
}

#[tokio::test]
async fn never_retry_stops_after_disconnect() {
    let server = TestServer::spawn(vec![ScriptedResponse::sse("data: once\n\n")]).await;

    let mut source = test_client()
        .get(server.url())
        .event_source()
        .unwrap()
        .with_retry_policy(NeverRetry);

    let first = source.next().await.unwrap().unwrap();
    let second = source.next().await;

    assert_eq!(first.data(), "once");
    assert!(second.is_none(), "event source should stop after disconnect");
    assert_eq!(server.request_count(), 1);
}

#[test]
fn rejects_unreplayable_request_bodies() {
    let stream = stream::iter([Ok::<Vec<u8>, Error>(b"payload".to_vec())]);
    let body = reqwest::Body::wrap_stream(stream);
    let builder = test_client().post("http://example.com/events").body(body);

    let error = builder.event_source().unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidRequest);
    assert_eq!(error.message(), "request body is not replayable");
}

#[test]
fn rejects_requests_with_last_event_id_header() {
    let builder = test_client().get("http://example.com/events").header("Last-Event-ID", "42");

    let error = builder.event_source().unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidRequest);
    assert_eq!(error.message(), "last event id header is reserved");
}

#[derive(Clone)]
struct ScriptedResponse {
    status_line: &'static str,
    content_type: Option<&'static str>,
    body: Vec<u8>,
}

impl ScriptedResponse {
    fn sse(body: &str) -> Self {
        Self {
            status_line: "200 OK",
            content_type: Some("text/event-stream"),
            body: body.as_bytes().to_vec(),
        }
    }

    fn sse_bytes(body: Vec<u8>) -> Self {
        Self { status_line: "200 OK", content_type: Some("text/event-stream"), body }
    }

    fn render(&self) -> Vec<u8> {
        let mut response = format!("HTTP/1.1 {}\r\n", self.status_line);

        if let Some(content_type) = self.content_type {
            response.push_str(&format!("content-type: {content_type}\r\n"));
        }

        response.push_str(&format!("content-length: {}\r\n", self.body.len()));
        response.push_str("connection: close\r\n\r\n");

        let mut bytes = response.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

struct TestServer {
    url: Url,
    requests: Arc<Mutex<Vec<String>>>,
    request_count: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}

impl TestServer {
    async fn spawn(responses: Vec<ScriptedResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = Url::parse(&format!("http://{addr}/events")).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let request_count = Arc::new(AtomicUsize::new(0));
        let requests_clone = Arc::clone(&requests);
        let request_count_clone = Arc::clone(&request_count);

        let task = tokio::spawn(async move {
            let mut responses = VecDeque::from(responses);

            while let Some(response) = responses.pop_front() {
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = read_request(&mut socket).await;

                request_count_clone.fetch_add(1, Ordering::SeqCst);
                requests_clone.lock().unwrap().push(request);

                let bytes = response.render();
                socket.write_all(&bytes).await.unwrap();
                socket.shutdown().await.unwrap();
            }
        });

        Self { url, requests, request_count, task }
    }

    fn url(&self) -> Url {
        self.url.clone()
    }

    fn request_count(&self) -> usize {
        self.request_count.load(Ordering::SeqCst)
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0, "connection closed before request headers completed");
        bytes.extend_from_slice(&chunk[..read]);

        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    String::from_utf8(bytes).unwrap()
}

fn assert_context_contains(error: &Error, key: &'static str, value: &str) {
    assert!(
        error.context().iter().any(|(candidate_key, candidate_value)| {
            *candidate_key == key && candidate_value == value
        }),
        "missing context entry {key}={value} in {error:?}"
    );
}

fn test_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

#[derive(Debug, Clone)]
struct RecordingPolicy {
    contexts: Arc<Mutex<Vec<RetryContext>>>,
    delay: Option<Duration>,
    stop_after: Option<usize>,
}

impl RecordingPolicy {
    fn new(
        contexts: Arc<Mutex<Vec<RetryContext>>>,
        delay: Option<Duration>,
        stop_after: Option<usize>,
    ) -> Self {
        Self { contexts, delay, stop_after }
    }
}

impl RetryPolicy for RecordingPolicy {
    fn next_delay(&self, context: RetryContext) -> Option<Duration> {
        let mut contexts = self.contexts.lock().unwrap();
        contexts.push(context);

        if self.stop_after.is_some_and(|limit| contexts.len() > limit) { None } else { self.delay }
    }
}
