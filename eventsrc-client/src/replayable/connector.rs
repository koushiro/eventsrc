use std::fmt::Debug;

use bytes::Bytes;
use futures_core::{future::BoxFuture, stream::BoxStream};

use crate::error::Error;

/// An erased byte stream used by backend-neutral replayable connectors.
pub type BoxBodyStream = BoxStream<'static, Result<Bytes, Error>>;

/// An erased async connection attempt used by backend-neutral replayable connectors.
pub type ConnectFuture = BoxFuture<'static, Result<BoxBodyStream, Error>>;

/// A backend-specific connector that can establish one SSE connection attempt.
pub trait Connector: Debug + Send + Sync + 'static {
    /// Starts a single connection attempt using the current `Last-Event-ID`, if any.
    fn connect(&self, last_event_id: Option<&str>) -> Result<ConnectFuture, Error>;
}
