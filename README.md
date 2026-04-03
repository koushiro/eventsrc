# eventsrc

A protocol-correct SSE / EventSource client for Rust, with explicit one-shot and replayable modes.

## Overview

`eventsrc` is organized as a small workspace rather than a single crate:

- `eventsrc`
  - facade crate
  - re-exports the public API from `eventsrc-core` and `eventsrc-client`
- `eventsrc-core`
  - transport-agnostic SSE protocol parsing
  - exposes `FrameStream<S>` and `EventStream<S>`
- `eventsrc-client`
  - client-facing one-shot and replayable modes
  - owns reconnect, request replay, backend adapter boundaries, and retry

This split keeps protocol logic separate from HTTP client integration.

## Two Explicit Modes

The public API keeps two lifecycle models separate:

- `oneshot::EventSource`
  - consumes one accepted body stream
  - does not reconnect
  - intended for API streaming and LLM-style responses
- `replayable::EventSource`
  - reconnects through a backend-neutral connector
  - preserves `Last-Event-ID`
  - applies retry policy
  - intended for classic EventSource-style subscriptions

## Usage

The examples below assume:

- the default `reqwest` feature is enabled
- a Tokio runtime is available

### One-Shot Streaming

```rust
use eventsrc::oneshot::EventSourceExt as _;
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
use eventsrc::replayable::{ConstantBackoff, EventSourceExt as _};
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

See [documentation](https://docs.rs/eventsrc) for more details.

## Performance

See [benchmarks](https://github.com/koushiro/eventsrc/blob/main/benchmarks/README.md) for more details

## License

This project is licensed under the Apache License, Version 2.0 - see the [LICENSE](LICENSE) file for details.
