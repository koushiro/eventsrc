use std::{
    error::Error as StdError,
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use eventsrc_core::{Event, EventStream};
use futures_core::stream::Stream;

use crate::error::Error;

struct OneshotStream<S> {
    inner: Pin<Box<EventStream<S>>>,
}

impl<S> OneshotStream<S> {
    fn new<B, E>(body: S) -> Self
    where
        S: Stream<Item = Result<B, E>>,
        B: AsRef<[u8]>,
    {
        Self { inner: Box::pin(EventStream::new(body)) }
    }
}

impl<S, B, E> Stream for OneshotStream<S>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: StdError + Send + Sync + 'static,
{
    type Item = Result<Event, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => Poll::Ready(Some(Ok(event))),
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

type BoxEventStream = Pin<Box<dyn Stream<Item = Result<Event, Error>> + 'static>>;

/// One-shot SSE event source backed by a single body stream.
pub struct EventSource {
    inner: BoxEventStream,
}

impl fmt::Debug for EventSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("oneshot::EventSource { .. }")
    }
}

impl EventSource {
    /// Creates a one-shot SSE event source from a byte stream body.
    pub fn new<S, B, E>(body: S) -> Self
    where
        S: Stream<Item = Result<B, E>> + 'static,
        B: AsRef<[u8]>,
        E: StdError + Send + Sync + 'static,
    {
        let inner = Box::pin(OneshotStream::new(body));
        Self { inner }
    }
}

impl Stream for EventSource {
    type Item = Result<Event, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

/// Extension methods for building one-shot SSE event sources from backend-specific responses.
pub trait EventSourceExt: Sized {
    /// Validates the backend response and converts its body into a one-shot [`EventSource`].
    fn event_source(self) -> Result<EventSource, Error>;
}
