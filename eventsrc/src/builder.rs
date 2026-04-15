use core::{mem, str, time::Duration};

use bytes::{BufMut, Bytes, BytesMut};

use crate::{
    error::{FieldKind, ProtocolError},
    event::{Event, Frame},
    parser::{RawField, RawLine},
};

#[derive(Debug, Default)]
enum DataBuffer {
    #[default]
    Empty,
    Single(Bytes),
    Multi(BytesMut),
}

impl DataBuffer {
    fn push(&mut self, value: Bytes) {
        match self {
            Self::Empty => *self = Self::Single(value),
            Self::Single(existing) => {
                let mut buffer = BytesMut::with_capacity(existing.len() + 1 + value.len());
                buffer.extend_from_slice(existing.as_ref());
                buffer.put_u8(b'\n');
                buffer.extend_from_slice(value.as_ref());
                *self = Self::Multi(buffer);
            },
            Self::Multi(buffer) => {
                buffer.put_u8(b'\n');
                buffer.extend_from_slice(value.as_ref());
            },
        }
    }

    fn finish(&mut self) -> Option<Bytes> {
        match mem::take(self) {
            Self::Empty => None,
            Self::Single(value) => Some(value),
            Self::Multi(buffer) => Some(buffer.freeze()),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct EventBuilder {
    data: DataBuffer,
    event: Option<Bytes>,
    last_event_id: Bytes,
}

impl EventBuilder {
    pub(crate) fn last_event_id(&self) -> &str {
        // SAFETY: last-event-id state is only built from validated UTF-8 or UTF-8 input.
        unsafe { str::from_utf8_unchecked(self.last_event_id.as_ref()) }
    }

    pub(crate) fn set_last_event_id(&mut self, last_event_id: impl AsRef<str>) {
        let bytes = last_event_id.as_ref().as_bytes();
        self.last_event_id = Bytes::copy_from_slice(bytes);
    }

    pub(crate) fn feed(&mut self, line: RawLine) -> Result<Option<Frame>, ProtocolError> {
        match line {
            RawLine::Empty => Ok(self.dispatch().map(Frame::Event)),
            RawLine::Comment => Ok(None),
            RawLine::Field(field) => self.feed_field(field),
        }
    }

    fn feed_field(&mut self, field: RawField) -> Result<Option<Frame>, ProtocolError> {
        match field.name() {
            // If the field name is "event"
            // Set the event type buffer to the field value.
            b"event" => {
                let value = field.value_bytes();
                validate_utf8(FieldKind::Event, value.as_ref())?;
                self.event = Some(value);
                Ok(None)
            },
            // If the field name is "data"
            // Append the field value to the data buffer, then append a single U+000A LINE FEED (LF)
            // character to the data buffer.
            b"data" => {
                let value = field.value_bytes();
                validate_utf8(FieldKind::Data, value.as_ref())?;
                self.data.push(value);
                Ok(None)
            },
            // If the field name is "id"
            // If the field value does not contain U+0000 NULL, then set the last event ID buffer to
            // the field value. Otherwise, ignore the field.
            b"id" => {
                let value = field.value_bytes();
                validate_utf8(FieldKind::Id, value.as_ref())?;
                if !value.contains(&0) {
                    self.last_event_id = value;
                }
                Ok(None)
            },
            // If the field name is "retry"
            // If the field value consists of only ASCII digits, then interpret the field value as
            // an integer in base ten, and set the event stream's reconnection time to that integer.
            // Otherwise, ignore the field.
            b"retry" => match parse_retry(field.value()) {
                Some(delay) => Ok(Some(Frame::Retry(delay))),
                None => Ok(None),
            },
            // Otherwise, The field is ignored.
            _ => Ok(None),
        }
    }

    fn dispatch(&mut self) -> Option<Event> {
        let data = match self.data.finish() {
            Some(data) => data,
            None => {
                self.event = None;
                return None;
            },
        };

        let event = self
            .event
            .take()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(default_event);

        Some(Event { event, data, id: self.last_event_id.clone() })
    }
}

fn validate_utf8(field: FieldKind, bytes: &[u8]) -> Result<(), ProtocolError> {
    str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|source| ProtocolError::invalid_utf8(field, source))
}

fn parse_retry(value: &[u8]) -> Option<Duration> {
    // retry value must be ASCII digits
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }

    let mut n: u64 = 0;
    for &b in value {
        let digit = (b - b'0') as u64;
        n = n.checked_mul(10)?.checked_add(digit)?;
    }
    Some(Duration::from_millis(n))
}

const fn default_event() -> Bytes {
    Bytes::from_static(b"message")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_event_and_id_are_normalized() {
        let mut builder = EventBuilder::default();

        assert!(builder.feed(RawLine::field("id", "42")).unwrap().is_none());
        assert!(builder.feed(RawLine::field("data", "hello")).unwrap().is_none());
        let frame = builder.feed(RawLine::Empty).unwrap();

        match frame {
            Some(Frame::Event(event)) => {
                assert_eq!(event.event(), "message");
                assert_eq!(event.data(), "hello");
                assert_eq!(event.id(), "42");
            },
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[test]
    fn multiline_data_is_joined_with_newlines() {
        let mut builder = EventBuilder::default();

        assert!(builder.feed(RawLine::field("data", "alpha")).unwrap().is_none());
        assert!(builder.feed(RawLine::field("data", "beta")).unwrap().is_none());
        let frame = builder.feed(RawLine::Empty).unwrap();

        match frame {
            Some(Frame::Event(event)) => assert_eq!(event.data(), "alpha\nbeta"),
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[test]
    fn event_name_defaults_to_message_when_empty() {
        let mut builder = EventBuilder::default();

        assert!(builder.feed(RawLine::field("event", "")).unwrap().is_none());
        assert!(builder.feed(RawLine::field("data", "payload")).unwrap().is_none());
        let frame = builder.feed(RawLine::Empty).unwrap();

        match frame {
            Some(Frame::Event(event)) => assert_eq!(event.event(), "message"),
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[test]
    fn retry_is_emitted_as_control_frame() {
        let mut builder = EventBuilder::default();

        let frame = builder.feed(RawLine::field("retry", "1500")).unwrap();
        assert_eq!(frame, Some(Frame::Retry(Duration::from_millis(1500))));
    }

    #[test]
    fn invalid_retry_is_ignored() {
        let mut builder = EventBuilder::default();

        assert!(builder.feed(RawLine::field("retry", "oops")).unwrap().is_none());
    }

    #[test]
    fn invalid_retry_with_plus_or_whitespace_is_ignored() {
        let mut builder = EventBuilder::default();

        assert!(builder.feed(RawLine::field("retry", "+1")).unwrap().is_none());
        assert!(builder.feed(RawLine::field("retry", "1 ")).unwrap().is_none());
    }

    #[test]
    fn id_with_nul_is_ignored() {
        let mut builder = EventBuilder::default();

        builder.set_last_event_id("42");
        assert!(builder.feed(RawLine::field("id", "bad\0id")).unwrap().is_none());
        assert_eq!(builder.last_event_id(), "42");
    }

    #[test]
    fn invalid_utf8_in_data_is_a_protocol_error() {
        use alloc::string::ToString;
        use core::error::Error as _;

        let mut builder = EventBuilder::default();

        let error = builder.feed(RawLine::field("data", Bytes::from_static(&[0xff]))).unwrap_err();
        assert!(error.to_string().contains("invalid UTF-8 in SSE data field"));
        assert!(error.source().is_some());
    }

    #[test]
    fn empty_dispatch_without_data_produces_no_event() {
        let mut builder = EventBuilder::default();

        assert!(builder.feed(RawLine::field("event", "update")).unwrap().is_none());
        assert!(builder.feed(RawLine::Empty).unwrap().is_none());
    }
}
