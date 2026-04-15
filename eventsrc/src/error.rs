use core::{error, fmt, str::Utf8Error};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Data,
    Event,
    Id,
}

impl fmt::Display for FieldKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data => f.write_str("data"),
            Self::Event => f.write_str("event"),
            Self::Id => f.write_str("id"),
        }
    }
}

/// A protocol decoding failure produced after a textual SSE field was recognized.
///
/// This error is currently used for invalid UTF-8 in the `data`, `event`, or `id` fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolError {
    field: FieldKind,
    source: Utf8Error,
}

impl ProtocolError {
    pub(crate) fn invalid_utf8(field: FieldKind, source: Utf8Error) -> Self {
        Self { field, source }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UTF-8 in SSE {} field: {}", self.field, self.source)
    }
}

impl error::Error for ProtocolError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.source)
    }
}

/// A stream-layer error produced while adapting a byte stream into SSE output.
#[derive(Debug, PartialEq, Eq)]
pub enum StreamError<E> {
    /// The upstream byte source returned an error.
    Source(E),
    /// SSE protocol decoding failed for the current stream.
    Protocol(ProtocolError),
}

impl<E> From<ProtocolError> for StreamError<E> {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl<E> fmt::Display for StreamError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => fmt::Display::fmt(&error, f),
            Self::Protocol(error) => fmt::Display::fmt(&error, f),
        }
    }
}

impl<E> error::Error for StreamError<E>
where
    E: error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Protocol(error) => Some(error),
        }
    }
}
