use std::{str, time::Duration};

use bytes::Bytes;

/// A protocol-level output frame emitted by the SSE decoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    /// A normalized SSE event ready for user-facing consumption.
    Event(Event),
    /// A `retry:` control instruction emitted from the stream.
    Retry(Duration),
}

/// A normalized SSE event with validated UTF-8 fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub(crate) event: Bytes,
    pub(crate) data: Bytes,
    pub(crate) id: Bytes,
}

impl Event {
    /// Returns the final event type used at dispatch time.
    ///
    /// When no `event:` field was provided, this is `"message"`.
    pub fn event(&self) -> &str {
        // SAFETY: event field is guaranteed valid UTF-8 by the protocol
        unsafe { str::from_utf8_unchecked(self.event.as_ref()) }
    }

    /// Returns the fully assembled event data.
    ///
    /// Multiple `data:` lines are joined with `\n`.
    pub fn data(&self) -> &str {
        // SAFETY: data field is guaranteed valid UTF-8 by the protocol
        unsafe { str::from_utf8_unchecked(self.data.as_ref()) }
    }

    /// Returns the effective last-event-id at dispatch time.
    pub fn id(&self) -> &str {
        // SAFETY: id field is guaranteed valid UTF-8 by the protocol
        unsafe { str::from_utf8_unchecked(self.id.as_ref()) }
    }
}
