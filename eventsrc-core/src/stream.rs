use core::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::{
    builder::EventBuilder,
    error::{ProtocolError, StreamError},
    event::{Event, Frame},
    parser::Parser,
};

pin_project! {
    /// A protocol-level SSE stream that yields [`Frame`] values.
    #[derive(Debug)]
    pub struct FrameStream<S> {
        #[pin]
        source: S,
        parser: Parser,
        builder: EventBuilder,
        terminated: bool,
    }
}

impl<S> FrameStream<S> {
    /// Creates a new protocol-level stream from an upstream byte source.
    pub fn new(source: S) -> Self {
        Self { source, parser: Parser::new(), builder: EventBuilder::default(), terminated: false }
    }

    /// Seeds the stream with an initial effective last-event-id value.
    pub fn with_last_event_id(mut self, last_event_id: impl AsRef<str>) -> Self {
        self.builder.set_last_event_id(last_event_id);
        self
    }

    /// Returns the current effective last-event-id value.
    pub fn last_event_id(&self) -> &str {
        self.builder.last_event_id()
    }

    /// Returns the wrapped upstream source.
    pub fn into_inner(self) -> S {
        self.source
    }
}

fn try_next_frame(
    parser: &mut Parser,
    builder: &mut EventBuilder,
) -> Result<Option<Frame>, ProtocolError> {
    loop {
        let Some(line) = parser.next() else {
            return Ok(None);
        };

        if let Some(frame) = builder.feed(line)? {
            return Ok(Some(frame));
        }
    }
}

impl<S, B, E> Stream for FrameStream<S>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    type Item = Result<Frame, StreamError<E>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if *this.terminated {
            return Poll::Ready(None);
        }

        loop {
            match try_next_frame(this.parser, this.builder) {
                Ok(Some(frame)) => return Poll::Ready(Some(Ok(frame))),
                Ok(None) => {},
                Err(error) => {
                    *this.terminated = true;
                    let error = StreamError::Protocol(error);
                    return Poll::Ready(Some(Err(error)));
                },
            }

            match this.source.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => this.parser.push(chunk.as_ref()),
                Poll::Ready(Some(Err(error))) => {
                    *this.terminated = true;
                    let error = StreamError::Source(error);
                    return Poll::Ready(Some(Err(error)));
                },
                Poll::Ready(None) => {
                    *this.terminated = true;
                    return Poll::Ready(None);
                },
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Main entrypoint for creating an [`Frame`] stream.
pub trait FrameStreamExt: Sized {
    /// Create an frame stream from a stream of bytes
    fn frame_stream(self) -> FrameStream<Self>;
}

impl<S, B, E> FrameStreamExt for S
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    fn frame_stream(self) -> FrameStream<Self> {
        FrameStream::new(self)
    }
}

pin_project! {
    /// An event-only SSE stream built on top of [`FrameStream`].
    #[derive(Debug)]
    pub struct EventStream<S> {
        #[pin]
        inner: FrameStream<S>,
    }
}

impl<S> EventStream<S> {
    /// Creates a new event-only stream from an upstream byte source.
    pub fn new(source: S) -> Self {
        Self { inner: FrameStream::new(source) }
    }

    /// Seeds the underlying protocol stream with an initial last-event-id value.
    pub fn with_last_event_id(mut self, last_event_id: impl AsRef<str>) -> Self {
        self.inner = self.inner.with_last_event_id(last_event_id);
        self
    }

    /// Returns the current effective last-event-id value.
    pub fn last_event_id(&self) -> &str {
        self.inner.last_event_id()
    }

    /// Returns the wrapped protocol-level stream.
    pub fn into_inner(self) -> FrameStream<S> {
        self.inner
    }
}

impl<S, B, E> Stream for EventStream<S>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    type Item = Result<Event, StreamError<E>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(Frame::Event(event)))) => return Poll::Ready(Some(Ok(event))),
                Poll::Ready(Some(Ok(Frame::Retry(_)))) => {},
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(error))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Main entrypoint for creating an [`Event`] stream.
pub trait EventStreamExt: Sized {
    /// Create an event stream from a stream of bytes
    fn event_stream(self) -> EventStream<Self>;
}

impl<S, B, E> EventStreamExt for S
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    fn event_stream(self) -> EventStream<Self> {
        EventStream::new(self)
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, error::Error as _, io, time::Duration};

    use bytes::Bytes;
    use futures_util::{FutureExt, StreamExt, stream};

    use super::*;

    fn bytes(value: impl AsRef<[u8]>) -> Bytes {
        Bytes::copy_from_slice(value.as_ref())
    }

    #[test]
    fn frame_stream_emits_protocol_frames_from_chunked_input() {
        let stream = stream::iter([
            Ok::<Vec<u8>, Infallible>(b"retry: 1500\nda".to_vec()),
            Ok(b"ta: hello\n\n".to_vec()),
        ]);
        let stream = FrameStream::new(stream);

        let items = stream.collect::<Vec<_>>().now_or_never().expect("test stream should not pend");
        let items = items.into_iter().map(Result::unwrap).collect::<Vec<_>>();

        assert_eq!(
            items,
            vec![
                Frame::Retry(Duration::from_millis(1500)),
                Frame::Event(Event {
                    event: bytes("message"),
                    data: bytes("hello"),
                    id: Bytes::new(),
                }),
            ]
        );
    }

    #[test]
    fn frame_stream_exposes_effective_last_event_id() {
        let stream = stream::iter([Ok::<Vec<u8>, Infallible>(b"id: 42\ndata: hello\n\n".to_vec())]);
        let mut stream = FrameStream::new(stream);
        let item = stream.next().now_or_never().expect("test stream should not pend");

        assert!(matches!(item, Some(Ok(Frame::Event(_)))));
        assert_eq!(stream.last_event_id(), "42");
    }

    #[test]
    fn frame_stream_wraps_source_errors() {
        let stream = stream::iter([Err::<Vec<u8>, io::Error>(io::Error::other("boom"))]);
        let stream = FrameStream::new(stream);

        let items = stream.collect::<Vec<_>>().now_or_never().expect("test stream should not pend");

        assert_eq!(items.len(), 1);
        match &items[0] {
            Err(StreamError::Source(error)) => assert_eq!(error.to_string(), "boom"),
            other => panic!("unexpected item: {other:?}"),
        }
    }

    #[test]
    fn frame_stream_wraps_protocol_errors() {
        let stream = stream::iter([Ok::<Vec<u8>, Infallible>(vec![
            b'd', b'a', b't', b'a', b':', b' ', 0xff, b'\n', b'\n',
        ])]);
        let stream = FrameStream::new(stream);

        let items = stream.collect::<Vec<_>>().now_or_never().expect("test stream should not pend");

        assert_eq!(items.len(), 1);
        match &items[0] {
            Err(StreamError::Protocol(error)) => assert!(
                error.to_string().contains("invalid UTF-8 in SSE data field"),
                "unexpected error: {error:?}"
            ),
            other => panic!("unexpected item: {other:?}"),
        }
        match &items[0] {
            Err(StreamError::Protocol(error)) => assert!(error.source().is_some()),
            other => panic!("unexpected item: {other:?}"),
        }
    }

    #[test]
    fn frame_stream_discards_trailing_partial_line_at_eof() {
        let stream = stream::iter([
            Ok::<Vec<u8>, Infallible>(b"data: one\n\n".to_vec()),
            Ok(b"data: two".to_vec()),
        ]);
        let stream = FrameStream::new(stream);

        let items = stream.collect::<Vec<_>>().now_or_never().expect("test stream should not pend");
        let items = items.into_iter().map(Result::unwrap).collect::<Vec<_>>();

        assert_eq!(
            items,
            vec![Frame::Event(Event {
                event: bytes("message"),
                data: bytes("one"),
                id: Bytes::new(),
            })]
        );
    }

    #[test]
    fn event_stream_emits_only_events() {
        let stream = stream::iter([
            Ok::<Vec<u8>, Infallible>(b"retry: 5\ndata: one\n\n".to_vec()),
            Ok(b"data: two\n\n".to_vec()),
        ]);
        let stream = EventStream::new(stream);

        let items = stream.collect::<Vec<_>>().now_or_never().expect("test stream should not pend");
        let items = items.into_iter().map(Result::unwrap).collect::<Vec<_>>();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].data(), "one");
        assert_eq!(items[1].data(), "two");
    }

    #[test]
    fn event_stream_exposes_effective_last_event_id() {
        let stream = stream::iter([Ok::<Vec<u8>, Infallible>(b"id: 9\ndata: hello\n\n".to_vec())]);
        let mut stream = EventStream::new(stream);
        let item = stream.next().now_or_never().expect("test stream should not pend");

        assert!(matches!(item, Some(Ok(_))));
        assert_eq!(stream.last_event_id(), "9");
    }
}
