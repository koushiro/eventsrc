//! Transport-agnostic SSE protocol parsing and stream adaptation.
//!
//! This crate exposes only normalized events, protocol frames, and stream adapters.
//! Raw line parsing and event-building state remain internal implementation details.
//!
//! # Event-Only Consumption
//!
//! ```no_run
//! use std::convert::Infallible;
//!
//! use bytes::Bytes;
//! use eventsrc_core::EventStream;
//! use futures_util::{StreamExt, stream};
//!
//! # async fn demo() -> Result<(), eventsrc_core::StreamError<Infallible>> {
//! let chunks = stream::iter([Ok::<Bytes, Infallible>(Bytes::from_static(b"data: hello\n\n"))]);
//! let mut stream = EventStream::new(chunks);
//!
//! let event = stream.next().await.unwrap()?;
//! assert_eq!(event.event(), "message");
//! assert_eq!(event.data(), "hello");
//! assert_eq!(event.id(), "");
//! # Ok(())
//! # }
//! ```

#![deny(unused_imports)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod builder;
mod error;
mod event;
mod parser;
mod stream;

pub use error::{ProtocolError, StreamError};
pub use event::{Event, Frame};
pub use stream::{EventStream, Eventsource, FrameStream};
