# SSE Client Design

## Goals

This document defines the current design of the `eventsrc-client` crate.

The client layer is responsible for:

- exposing two explicit SSE consumption modes
- adapting accepted byte streams into SSE event streams
- handling reconnect, request replay, and `Last-Event-ID` propagation
- keeping HTTP-client-specific logic in backend adapters
- remaining separate from protocol parsing in `eventsrc`

This client layer is **not** responsible for:

- SSE line parsing or protocol state machine details
- server-side SSE
- browser DOM compatibility
- payload decoding beyond SSE protocol handling
- supporting multiple public backend adapters in v0.1

---

## Relationship to Core

`eventsrc` remains the protocol crate.

Conceptually:

```text
HTTP client / backend adapter
    ↓
eventsrc-client
    ↓
eventsrc
```

The split is:

- `eventsrc`
  - bytes-to-frame parsing
  - bytes-to-event projection
  - protocol semantics for `event`, `data`, `id`, and `retry`
- `eventsrc-client`
  - one-shot and replayable client-facing modes
  - backend adapter boundaries
  - reconnect scheduling
  - request replay
  - `Last-Event-ID` request propagation

The client crate should not duplicate parser or event-building logic from core.

---

## Public Model

The client crate exposes two distinct public modes:

- `oneshot::EventSource`
- `replayable::EventSource`

They are intentionally separate because they represent different lifecycle models.

### One-Shot Mode

`oneshot::EventSource` consumes a single accepted body stream and yields SSE events until EOF or error.

This mode:

- does not reconnect
- does not replay requests
- does not own HTTP response validation

The constructor takes a body stream directly:

```rust
oneshot::EventSource::new(body_stream)
```

HTTP-specific validation such as status-code and `content-type` checks belongs to the backend adapter, not to the SSE stream itself.

### Replayable Mode

`replayable::EventSource` owns a reconnecting state machine.

This mode:

- starts new connection attempts through a backend-neutral connector
- injects `Last-Event-ID` across reconnects
- applies retry policy decisions
- treats valid `retry:` frames as reconnect-delay input

This mode does not directly know about `reqwest::Response` or `reqwest::RequestBuilder`.

---

## Module Boundaries

The current crate is organized around four public/internal areas.

### `error`

`error` defines the current client-level error model:

- `Error`
- `ErrorKind`
- `ErrorSource`

Current error categories are:

- `Transport`
- `InvalidRequest`
- `InvalidResponse`
- `Protocol`

This is a client-layer error model. It is intentionally separate from `eventsrc`'s `ProtocolError` and `StreamError<E>`.

### `oneshot`

`oneshot` contains:

- `oneshot::EventSource`
- `oneshot::EventSourceExt`

Responsibilities:

- accept a body byte stream
- project body bytes into `Event` values through `eventsrc::EventStream`
- surface client-layer errors

Non-responsibilities:

- validating HTTP head metadata
- reconnect scheduling
- request replay

### `replayable`

`replayable` contains:

- `replayable::EventSource`
- `replayable::EventSourceExt`
- `replayable::Connector`
- `replayable::RetryPolicy`
- built-in retry policy implementations

Responsibilities:

- run the reconnect state machine
- preserve effective `last_event_id`
- classify reconnect causes
- apply retry timing
- convert accepted body streams into `FrameStream`

### `reqwest`

`reqwest` is an internal adapter module gated behind the `reqwest` feature.

It is the only backend adapter in v0.1.

Responsibilities:

- implement `oneshot::EventSourceExt for reqwest::Response`
- implement `replayable::EventSourceExt for reqwest::RequestBuilder`
- validate HTTP status and `content-type`
- freeze replayable requests
- extract response bodies into byte streams

The `reqwest` adapter is not the public mode boundary. The public mode boundary is still `oneshot` and `replayable`.

---

## HTTP Validation Boundary

The client crate intentionally does **not** expose a public normalized response type.

This is a deliberate design choice.

In real clients, the usual order is:

1. establish an HTTP response
2. validate response status and headers
3. consume the response body as SSE

The client design follows that order.

Therefore:

- HTTP head validation lives in the backend adapter
- `oneshot::EventSource` only consumes body streams
- `replayable::Connector` only returns accepted body streams or an error

This avoids pushing HTTP response metadata into the SSE public API surface.

---

## One-Shot Design

`oneshot::EventSource` is a thin client wrapper around `eventsrc::EventStream`.

Conceptually:

```text
accepted body stream
    ↓
EventStream<S>
    ↓
oneshot::EventSource
```

The type is intentionally concrete rather than public-generic.

Internally it erases the specific body stream type into a boxed stream of:

```rust
Stream<Item = Result<Event, Error>>
```

This keeps the public API simpler while still allowing any backend adapter to pass in an accepted body stream.

`oneshot::EventSourceExt` exists for adapter ergonomics only.

For the current `reqwest` adapter:

- the extension method validates status and `content-type`
- then converts the body to a byte stream
- then constructs `oneshot::EventSource`

---

## Replayable Design

Replayable mode introduces one extra abstraction beyond one-shot mode: `Connector`.

### `Connector`

`Connector` is the minimal backend-neutral interface needed for reconnecting SSE.

It exists because replayable mode cannot be backend-neutral without some way to say:

- start one connection attempt
- use the current `Last-Event-ID`
- return an accepted byte stream or an error

Current shape:

- input: `Option<&str>` for `Last-Event-ID`
- output: `ConnectFuture`
- success value: `BoxBodyStream`
- error value: `Error`

This is intentionally minimal. It is not meant to be a general transport framework.

### Replay State Machine

`replayable::EventSource` manages these internal states:

- idle
- connecting
- streaming
- waiting
- closed

Conceptually:

```text
Idle
  ↓
Connector::connect(last_event_id)
  ↓
accepted body stream
  ↓
FrameStream<BoxBodyStream>
  ↓
Event / Retry / error
```

### `Last-Event-ID`

Replayable mode owns `Last-Event-ID` state.

It:

- updates the effective id from `FrameStream::last_event_id()`
- injects that id into future connection attempts through the connector

The adapter is responsible for encoding this into the backend-specific request shape.

### Retry

Replayable mode separates:

- retry classification
- retry timing

`EventSource` decides whether a transition is retryable.

`RetryPolicy` decides:

- the next delay
- whether retrying should stop

Built-in policies are:

- `ConstantBackoff`
- `ExponentialBackoff`
- `NeverRetry`

`retry:` frames from the server update the reconnect base delay, but they do not change the retry classification rules.

---

## Error Model

The client crate currently uses a single public `Error` type with:

- `ErrorKind`
- human-readable message
- optional structured context entries
- optional source error

This model is used to keep client-layer error handling uniform across:

- request freezing
- response validation
- response body streaming
- transport failures
- protocol failures surfaced from core

Current category mapping is:

- `Transport`
  - request execution or connection setup failures
- `InvalidRequest`
  - invalid replayable request setup
- `InvalidResponse`
  - invalid HTTP response head
  - invalid response body stream
- `Protocol`
  - SSE protocol violations surfaced from `eventsrc`

This error model belongs to the client crate only.

`eventsrc` should keep its own typed protocol and stream error model.

---

## Feature Model

The current feature layout is:

```toml
[features]
default = ["reqwest"]
reqwest = ["dep:reqwest"]
```

Implications:

- by default, `reqwest` extension methods are available
- without default features, the crate still exposes `oneshot`, `replayable`, and `error`
- the `reqwest` adapter is optional, but it is the only backend adapter in v0.1

This keeps the public mode APIs backend-neutral while preserving a practical default integration.

---

## Public API Summary

The intended public client-facing API is centered on:

- `eventsrc_client::oneshot::EventSource`
- `eventsrc_client::oneshot::EventSourceExt`
- `eventsrc_client::replayable::EventSource`
- `eventsrc_client::replayable::EventSourceExt`
- `eventsrc_client::replayable::Connector`
- `eventsrc_client::replayable::RetryPolicy`
- `eventsrc_client::error::{Error, ErrorKind}`

The internal `reqwest` module is not part of the main conceptual model.

---

## Non-Goals

The following remain out of scope for the current client crate:

- multiple public backend adapters
- a general-purpose transport abstraction layer
- server-side SSE
- non-HTTP transports
- collapsing one-shot and replayable into one type
- moving protocol parsing out of `eventsrc`

---

## Summary

The client crate is designed as a thin, mode-oriented layer above `eventsrc`.

Its core design principles are:

- keep one-shot and replayable explicit
- keep HTTP validation in backend adapters
- keep SSE parsing in core
- keep reconnect and replay in replayable mode
- keep `reqwest` practical but optional

This gives the workspace a stable split:

```text
eventsrc        = protocol
eventsrc-client = client modes + adapter boundary
```
