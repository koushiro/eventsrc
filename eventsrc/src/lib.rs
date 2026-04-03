//! `eventsrc` is the small public facade for this workspace.
//!
//! The crate keeps two SSE consumption modes explicit:
//!
//! - [`oneshot::EventSource`] for one-shot API streaming
//! - [`replayable::EventSource`] for reconnecting request replay
//!
//! For lower-level protocol access, use the re-exported `eventsrc-core` types
//! such as [`EventStream`] and [`FrameStream`].
//!
//! # One-Shot Mode
//!
//! The facade re-exports mode APIs, but does not directly depend on `reqwest`.
//! These examples are illustrative; compile-checked reqwest examples live in
//! `eventsrc-client`.
//!
//! ```ignore
//! use eventsrc::oneshot::EventSourceExt as _;
//! use futures_util::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let response = reqwest::Client::new()
//!         .get("https://example.com/v1/stream")
//!         .send()
//!         .await?;
//!
//!     let mut stream = response.event_source()?;
//!
//!     while let Some(event) = stream.next().await {
//!         let event = event?;
//!         println!("{}", event.data());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Reconnecting Mode
//!
//! ```ignore
//! use eventsrc::replayable::{ConstantBackoff, EventSourceExt as _};
//! use futures_util::StreamExt;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut stream = reqwest::Client::new()
//!         .get("https://example.com/events")
//!         .event_source()?
//!         .with_retry_policy(ConstantBackoff::new(Duration::from_secs(1)));
//!
//!     while let Some(event) = stream.next().await {
//!         let event = event?;
//!         println!("{}", event.data());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Retry Policies
//!
//! Replayable mode accepts any [`replayable::RetryPolicy`] implementation.
//! The facade re-exports three built-in choices:
//!
//! - [`replayable::ConstantBackoff`] for a fixed delay between reconnects
//! - [`replayable::ExponentialBackoff`] for increasing delays after failures
//! - [`replayable::NeverRetry`] to disable reconnect entirely

#![deny(unused_imports)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use eventsrc_client as client;
pub use eventsrc_core::*;
