use std::{
    fmt,
    fmt::Debug,
    mem,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use eventsrc_core::{Event, Frame, FrameStream, StreamError};
use futures_core::Stream;
use tokio::time;

mod connector;
mod retry;

pub use self::{
    connector::{BoxBodyStream, ConnectFuture, Connector},
    retry::*,
};
use crate::error::{Error, ErrorKind};

type BoxFrameStream = Pin<Box<FrameStream<BoxBodyStream>>>;
type SleepFuture = Pin<Box<time::Sleep>>;

enum ConnectionState {
    Idle,
    Connecting(ConnectFuture),
    Streaming(BoxFrameStream),
    Waiting(SleepFuture),
    Closed,
}

/// Reconnecting SSE event source backed by a backend-neutral connector.
pub struct EventSource {
    connector: Arc<dyn Connector>,
    retry_policy: Arc<dyn RetryPolicy>,
    last_event_id: Option<String>,
    server_retry_delay: Option<Duration>,
    consecutive_failures: usize,
    state: ConnectionState,
}

impl Clone for EventSource {
    fn clone(&self) -> Self {
        Self {
            connector: self.connector.clone(),
            retry_policy: self.retry_policy.clone(),
            last_event_id: self.last_event_id.clone(),
            server_retry_delay: self.server_retry_delay,
            consecutive_failures: self.consecutive_failures,
            state: ConnectionState::Idle,
        }
    }
}

impl Debug for EventSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("replayable::EventSource")
            .field("connector", &self.connector)
            .field("retry_policy", &self.retry_policy)
            .field("last_event_id", &self.last_event_id)
            .field("server_retry_delay", &self.server_retry_delay)
            .field("consecutive_failures", &self.consecutive_failures)
            .finish()
    }
}

impl EventSource {
    /// Creates a reconnecting event source from a backend-neutral connector.
    pub fn new<C>(connector: C) -> Self
    where
        C: Connector,
    {
        let retry_policy = Arc::new(ConstantBackoff::default());

        Self {
            connector: Arc::new(connector),
            retry_policy,
            last_event_id: None,
            server_retry_delay: None,
            consecutive_failures: 0,
            state: ConnectionState::Idle,
        }
    }

    /// Replaces the reconnect timing policy.
    ///
    /// Built-in policies include [`ConstantBackoff`], [`crate::replayable::ExponentialBackoff`],
    /// and [`crate::replayable::NeverRetry`].
    /// Custom policies may implement [`crate::replayable::RetryPolicy`] directly.
    pub fn with_retry_policy<P>(mut self, retry_policy: P) -> Self
    where
        P: RetryPolicy,
    {
        self.retry_policy = Arc::new(retry_policy);
        self
    }

    /// Returns the current effective `Last-Event-ID`, if one is stored.
    ///
    /// This value is updated from the SSE stream and reused on future reconnect
    /// attempts through the underlying connector.
    pub fn last_event_id(&self) -> Option<&str> {
        self.last_event_id.as_deref()
    }

    /// Replaces the stored `Last-Event-ID`.
    ///
    /// This affects the next reconnect attempt and overrides any previously
    /// remembered value until the stream updates it again.
    pub fn set_last_event_id(&mut self, last_event_id: impl Into<String>) {
        self.last_event_id = Some(last_event_id.into());
    }

    /// Clears the stored `Last-Event-ID`.
    ///
    /// Future reconnect attempts will omit the header until the stream observes
    /// a new event id.
    pub fn clear_last_event_id(&mut self) {
        self.last_event_id = None;
    }

    fn connect(&self) -> Result<ConnectFuture, Error> {
        self.connector.connect(self.last_event_id.as_deref())
    }

    fn update_last_event_id_from_stream(&mut self, stream: &BoxFrameStream) {
        let last_event_id = stream.as_ref().get_ref().last_event_id();

        if last_event_id.is_empty() {
            self.last_event_id = None;
        } else {
            self.last_event_id = Some(last_event_id.to_owned());
        }
    }

    fn schedule_reconnect(&mut self, cause: RetryCause) -> bool {
        let failure_streak = match cause {
            RetryCause::Disconnect => 0,
            RetryCause::ConnectError | RetryCause::StreamError => self.consecutive_failures + 1,
        };

        let Some(delay) = self.retry_policy.next_delay(RetryContext {
            cause,
            failure_streak,
            server_retry: self.server_retry_delay,
        }) else {
            self.state = ConnectionState::Closed;
            return false;
        };

        if matches!(cause, RetryCause::ConnectError | RetryCause::StreamError) {
            self.consecutive_failures += 1;
        }

        self.state = ConnectionState::Waiting(Box::pin(tokio::time::sleep(delay)));
        true
    }
}

impl Stream for EventSource {
    type Item = Result<Event, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let mut scheduled_reconnect = false;

        loop {
            match mem::replace(&mut this.state, ConnectionState::Closed) {
                ConnectionState::Idle => match this.connect() {
                    Ok(connect) => {
                        this.state = ConnectionState::Connecting(connect);
                    },
                    Err(error) => return Poll::Ready(Some(Err(error))),
                },
                ConnectionState::Connecting(mut connect) => match connect.as_mut().poll(cx) {
                    Poll::Pending => {
                        this.state = ConnectionState::Connecting(connect);
                        return Poll::Pending;
                    },
                    Poll::Ready(Ok(body)) => {
                        let stream = match this.last_event_id.as_deref() {
                            Some(last_event_id) => eventsrc_core::FrameStream::new(body)
                                .with_last_event_id(last_event_id),
                            None => eventsrc_core::FrameStream::new(body),
                        };

                        this.consecutive_failures = 0;
                        this.state = ConnectionState::Streaming(Box::pin(stream));
                    },
                    Poll::Ready(Err(err)) => {
                        if let ErrorKind::Transport = err.kind()
                            && this.schedule_reconnect(RetryCause::ConnectError)
                        {
                            scheduled_reconnect = true;
                            continue;
                        }
                        return Poll::Ready(Some(Err(err)));
                    },
                },
                ConnectionState::Streaming(mut stream) => match stream.as_mut().poll_next(cx) {
                    Poll::Pending => {
                        this.state = ConnectionState::Streaming(stream);
                        return Poll::Pending;
                    },
                    Poll::Ready(Some(Ok(Frame::Retry(delay)))) => {
                        this.server_retry_delay = Some(delay);
                        this.state = ConnectionState::Streaming(stream);
                    },
                    Poll::Ready(Some(Ok(Frame::Event(event)))) => {
                        this.update_last_event_id_from_stream(&stream);
                        this.state = ConnectionState::Streaming(stream);
                        return Poll::Ready(Some(Ok(event)));
                    },
                    Poll::Ready(Some(Err(StreamError::Protocol(error)))) => {
                        this.update_last_event_id_from_stream(&stream);
                        return Poll::Ready(Some(Err(error.into())));
                    },
                    Poll::Ready(Some(Err(StreamError::Source(error)))) => {
                        this.update_last_event_id_from_stream(&stream);

                        if this.schedule_reconnect(RetryCause::StreamError) {
                            scheduled_reconnect = true;
                            continue;
                        }

                        return Poll::Ready(Some(Err(error)));
                    },
                    Poll::Ready(None) => {
                        this.update_last_event_id_from_stream(&stream);
                        let _ = this.schedule_reconnect(RetryCause::Disconnect);
                        scheduled_reconnect = true;
                        continue;
                    },
                },
                ConnectionState::Waiting(mut sleep) => match sleep.as_mut().poll(cx) {
                    Poll::Pending => {
                        this.state = ConnectionState::Waiting(sleep);
                        return Poll::Pending;
                    },
                    Poll::Ready(()) => {
                        this.state = ConnectionState::Idle;

                        if scheduled_reconnect {
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }
                    },
                },
                ConnectionState::Closed => return Poll::Ready(None),
            }
        }
    }
}

/// Extension methods for building reconnecting SSE event sources from backend-specific clients.
pub trait EventSourceExt {
    /// Converts this backend-specific request source into a reconnecting [`EventSource`].
    fn event_source(self) -> Result<EventSource, Error>;
}
