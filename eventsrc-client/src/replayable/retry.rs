//! Retry policy types for reconnecting event sources.
//!
//! The reqwest integration keeps retry concerns split into two layers:
//!
//! - [`crate::replayable::EventSource`] classifies reconnectable transitions
//! - [`RetryPolicy`] maps that retry context to an optional delay
//!
//! This keeps transport and protocol semantics in the event source state machine
//! while still allowing callers to customize reconnect timing.
//!
//! Built-in policies:
//!
//! - [`ConstantBackoff`] for a fixed reconnect delay
//! - [`ExponentialBackoff`] for failure-sensitive backoff
//! - [`NeverRetry`] to stop reconnecting entirely
use std::{fmt::Debug, time::Duration};

use backon::{BackoffBuilder, ConstantBuilder, ExponentialBuilder};

/// Computes reconnect delays for retryable event source transitions.
pub trait RetryPolicy: Debug + Send + Sync + 'static {
    /// Returns the delay before the next reconnect attempt, or `None` to stop reconnecting.
    fn next_delay(&self, context: RetryContext) -> Option<Duration>;
}

/// Why the event source is reconnecting.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryCause {
    /// The stream reached EOF after a valid SSE response.
    Disconnect,
    /// Establishing the HTTP connection failed.
    ConnectError,
    /// Reading the SSE response body failed.
    StreamError,
}

/// Context passed to [`RetryPolicy`] when computing the next delay.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryContext {
    /// Why the reconnect is being scheduled.
    pub cause: RetryCause,
    /// Current consecutive failure count.
    ///
    /// This is `0` for [`RetryCause::Disconnect`], `1` for the first failure,
    /// `2` for the second consecutive failure, and so on.
    pub failure_streak: usize,
    /// Latest `retry:` directive observed from the server, if any.
    pub server_retry: Option<Duration>,
}

/// A fixed-delay reconnect policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantBackoff {
    delay: Duration,
    max_delay: Option<Duration>,
    max_retries: Option<usize>,
    jitter: bool,
}

impl ConstantBackoff {
    /// Creates a fixed-delay reconnect policy.
    pub fn new(delay: Duration) -> Self {
        Self { delay, max_delay: None, max_retries: None, jitter: false }
    }

    /// Clamps both configured and server-provided delays to `max_delay`.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = Some(max_delay);
        self
    }

    /// Limits how many consecutive failures may be retried.
    ///
    /// This budget is consumed only by failure causes and not by normal
    /// disconnects after a valid SSE response.
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Enables jitter on computed failure delays.
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }

    fn effective_delay(&self, server_retry: Option<Duration>) -> Duration {
        let delay = server_retry.unwrap_or(self.delay);

        match self.max_delay {
            Some(max_delay) => delay.min(max_delay),
            None => delay,
        }
    }

    fn builder(&self, delay: Duration) -> ConstantBuilder {
        let mut builder = ConstantBuilder::default().with_delay(delay);

        builder = match self.max_retries {
            Some(max_retries) => builder.with_max_times(max_retries),
            None => builder.without_max_times(),
        };

        if self.jitter { builder.with_jitter() } else { builder }
    }
}

impl Default for ConstantBackoff {
    /// Returns the default reconnect policy used by [`crate::replayable::EventSource`].
    ///
    /// The default is a constant 3 second delay.
    fn default() -> Self {
        Self::new(Duration::from_secs(3))
    }
}

impl RetryPolicy for ConstantBackoff {
    fn next_delay(&self, context: RetryContext) -> Option<Duration> {
        let delay = self.effective_delay(context.server_retry);

        match context.cause {
            RetryCause::Disconnect => Some(delay),
            RetryCause::ConnectError | RetryCause::StreamError => {
                let mut backoff = self.builder(delay).build();
                backoff.nth(context.failure_streak.saturating_sub(1))
            },
        }
    }
}

/// An exponential reconnect policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExponentialBackoff {
    initial_delay: Duration,
    max_delay: Option<Duration>,
    max_retries: Option<usize>,
    jitter: bool,
}

impl ExponentialBackoff {
    /// Creates an exponential reconnect policy using `initial_delay` as the base.
    pub fn new(initial_delay: Duration) -> Self {
        Self { initial_delay, max_delay: None, max_retries: None, jitter: false }
    }

    /// Clamps both configured and server-provided delays to `max_delay`.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = Some(max_delay);
        self
    }

    /// Limits how many consecutive failures may be retried.
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Enables jitter on computed failure delays.
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }

    fn effective_initial_delay(&self, server_retry: Option<Duration>) -> Duration {
        let delay = server_retry.unwrap_or(self.initial_delay);

        match self.max_delay {
            Some(max_delay) => delay.min(max_delay),
            None => delay,
        }
    }

    fn builder(&self, initial_delay: Duration) -> ExponentialBuilder {
        let mut builder = ExponentialBuilder::default().with_min_delay(initial_delay);

        builder = match self.max_delay {
            Some(max_delay) => builder.with_max_delay(max_delay),
            None => builder.without_max_delay(),
        };

        builder = match self.max_retries {
            Some(max_retries) => builder.with_max_times(max_retries),
            None => builder.without_max_times(),
        };

        if self.jitter { builder.with_jitter() } else { builder }
    }
}

impl RetryPolicy for ExponentialBackoff {
    fn next_delay(&self, context: RetryContext) -> Option<Duration> {
        let initial_delay = self.effective_initial_delay(context.server_retry);

        match context.cause {
            RetryCause::Disconnect => Some(initial_delay),
            RetryCause::ConnectError | RetryCause::StreamError => {
                let mut backoff = self.builder(initial_delay).build();
                backoff.nth(context.failure_streak.saturating_sub(1))
            },
        }
    }
}

/// A reconnect policy that never retries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NeverRetry;

impl RetryPolicy for NeverRetry {
    fn next_delay(&self, _context: RetryContext) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_backoff_reuses_the_same_delay_until_exhausted() {
        let policy = ConstantBackoff::new(Duration::from_millis(10)).with_max_retries(2);

        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 1,
                server_retry: None,
            }),
            Some(Duration::from_millis(10)),
        );
        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 2,
                server_retry: None,
            }),
            Some(Duration::from_millis(10)),
        );
        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 3,
                server_retry: None,
            }),
            None,
        );
    }

    #[test]
    fn disconnect_does_not_consume_retry_budget() {
        let policy = ConstantBackoff::new(Duration::from_millis(10)).with_max_retries(1);

        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::Disconnect,
                failure_streak: 0,
                server_retry: None,
            }),
            Some(Duration::from_millis(10)),
        );
        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 1,
                server_retry: None,
            }),
            Some(Duration::from_millis(10)),
        );
        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 2,
                server_retry: None,
            }),
            None,
        );
    }

    #[test]
    fn server_retry_replaces_the_policy_base_delay() {
        let policy = ConstantBackoff::new(Duration::from_secs(1));

        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::Disconnect,
                failure_streak: 0,
                server_retry: Some(Duration::from_millis(250)),
            }),
            Some(Duration::from_millis(250)),
        );
    }

    #[test]
    fn max_delay_clamps_server_retry_delay_for_constant_backoff() {
        let policy = ConstantBackoff::new(Duration::from_millis(10))
            .with_max_delay(Duration::from_millis(25));

        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::Disconnect,
                failure_streak: 0,
                server_retry: Some(Duration::from_millis(40)),
            }),
            Some(Duration::from_millis(25)),
        );
    }

    #[test]
    fn exponential_backoff_increases_delay_and_respects_max_delay() {
        let policy = ExponentialBackoff::new(Duration::from_millis(10))
            .with_max_delay(Duration::from_millis(25))
            .with_max_retries(4);

        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 1,
                server_retry: None,
            }),
            Some(Duration::from_millis(10)),
        );
        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 2,
                server_retry: None,
            }),
            Some(Duration::from_millis(20)),
        );
        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 3,
                server_retry: None,
            }),
            Some(Duration::from_millis(25)),
        );
    }

    #[test]
    fn never_retry_always_stops_reconnecting() {
        let policy = NeverRetry;

        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::Disconnect,
                failure_streak: 0,
                server_retry: None,
            }),
            None,
        );
        assert_eq!(
            policy.next_delay(RetryContext {
                cause: RetryCause::ConnectError,
                failure_streak: 1,
                server_retry: Some(Duration::from_secs(1)),
            }),
            None,
        );
    }
}
