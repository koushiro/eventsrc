//! Backend-neutral SSE client APIs with two explicit public modes:
//!
//! - [`oneshot::EventSource`] consumes a single accepted body stream
//! - [`replayable::EventSource`] reconnects through a backend-neutral connector
//!
//! The default `reqwest` feature provides extension trait impls for
//! `reqwest::Response` and `reqwest::RequestBuilder`, including validation of
//! HTTP status and `content-type`.
//!
//! # One-Shot Usage
//!
//! Inside an async context:
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use futures_util::StreamExt;
//! let response = reqwest::Client::new()
//!     .get("https://example.com/v1/stream")
//!     .send()
//!     .await?;
//!
//! use eventsrc_client::oneshot::EventSourceExt as _;
//!
//! let mut stream = response.event_source()?;
//!
//! while let Some(event) = stream.next().await {
//!     let event = event?;
//!     println!("{}", event.data());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Replayable Usage
//!
//! Inside an async context:
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use eventsrc_client::replayable::EventSourceExt as _;
//! use futures_util::StreamExt;
//!
//! let mut stream = reqwest::Client::new()
//!     .get("https://example.com/events")
//!     .event_source()?;
//!
//! while let Some(event) = stream.next().await {
//!     let event = event?;
//!     println!("{}", event.data());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Retry Policies
//!
//! Replayable mode separates retry classification from retry timing:
//!
//! - [`replayable::EventSource`] decides which transitions are retryable
//! - [`replayable::RetryPolicy`] decides how long to wait before retrying
//!
//! Built-in policies:
//!
//! - [`replayable::ConstantBackoff`] for a fixed delay
//! - [`replayable::ExponentialBackoff`] for failure-sensitive backoff
//! - [`replayable::NeverRetry`] to stop reconnecting after the first disconnect or failure

#![deny(unused_imports)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Unified client-layer error types and helpers.
pub mod error;
/// One-shot SSE consumption over a single accepted body stream.
pub mod oneshot;
/// Reconnecting SSE consumption with replay and retry support.
pub mod replayable;
#[cfg(feature = "reqwest")]
mod reqwest;
