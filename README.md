# eventsrc

[![](https://github.com/koushiro/eventsrc/actions/workflows/ci.yml/badge.svg)][actions]
[![](https://img.shields.io/docsrs/eventsrc)][docs.rs]
[![](https://img.shields.io/crates/v/eventsrc)][crates.io]
[![](https://img.shields.io/crates/l/eventsrc)][crates.io]
[![](https://img.shields.io/crates/d/eventsrc)][crates.io]
[![](https://img.shields.io/badge/MSRV-1.85.0-green?logo=rust)][whatrustisit]

[actions]: https://github.com/koushiro/eventsrc/actions
[docs.rs]: https://docs.rs/eventsrc
[crates.io]: https://crates.io/crates/eventsrc
[whatrustisit]: https://www.whatrustisit.com

A protocol-correct SSE / EventSource implementation for Rust, split into a transport-agnostic protocol crate and an explicit client crate.

## Overview

`eventsrc` is organized as a small workspace with two public crates:

- `eventsrc`
  - transport-agnostic SSE protocol parsing
  - exposes `FrameStream<S>` and `EventStream<S>`
- `eventsrc-client`
  - client-facing one-shot and replayable modes
  - owns reconnect, request replay, backend adapter boundaries, and retry

This split keeps protocol logic separate from HTTP client integration. There is no facade crate.

## Two Explicit Modes

The `eventsrc-client` public API keeps two lifecycle models separate:

- `oneshot::EventSource`
  - consumes one accepted body stream
  - does not reconnect
  - intended for API streaming and LLM-style responses
- `replayable::EventSource`
  - reconnects through a backend-neutral connector
  - preserves `Last-Event-ID`
  - applies retry policy
  - intended for classic EventSource-style subscriptions

## Protocol Usage

Use `eventsrc` when you already have a byte stream and only need SSE protocol parsing.

```rust
use std::convert::Infallible;

use bytes::Bytes;
use eventsrc::EventStream;
use futures_util::{stream, StreamExt};

# async fn demo() -> Result<(), eventsrc::StreamError<Infallible>> {
let chunks = stream::iter([Ok::<Bytes, Infallible>(Bytes::from_static(b"data: hello\n\n"))]);
let mut stream = EventStream::new(chunks);

let event = stream.next().await.unwrap()?;
assert_eq!(event.event(), "message");
assert_eq!(event.data(), "hello");
# Ok(())
# }
```

## Client Usage

The client examples below assume:

- the `eventsrc-client` crate is used
- the default `reqwest` feature is enabled
- a Tokio runtime is available

### One-Shot Streaming

```rust
use eventsrc_client::oneshot::EventSourceExt as _;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::Client::new()
        .get("https://example.com/v1/stream")
        .send()
        .await?;

    let mut stream = response.event_source()?;

    while let Some(event) = stream.next().await {
        let event = event?;
        println!("{}", event.data());
    }

    Ok(())
}
```

### Replayable Streaming

```rust
use eventsrc_client::replayable::{ConstantBackoff, EventSourceExt as _};
use futures_util::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = reqwest::Client::new()
        .get("https://example.com/events")
        .event_source()?
        .with_retry_policy(ConstantBackoff::new(Duration::from_secs(1)));

    while let Some(event) = stream.next().await {
        let event = event?;
        println!("{}", event.data());
    }

    Ok(())
}
```

See [`eventsrc`](https://docs.rs/eventsrc) and
[`eventsrc-client`](https://docs.rs/eventsrc-client) documentation for more details.

## Performance

See [benchmarks](https://github.com/koushiro/eventsrc/blob/main/benchmarks/README.md) for more details

## License

This project is licensed under the Apache License, Version 2.0 - see the [LICENSE](LICENSE) file for details.
