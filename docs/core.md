# SSE Core Design

## Goals

This document defines the core design for a Rust SSE implementation focused on:

- strict protocol semantics
- incremental streaming parsing
- bytes-first internal representation
- minimal and stable public API
- clean separation between protocol decoding and higher-level transport / reconnection / payload decoding

This core is responsible for:

- parsing SSE lines from arbitrary byte chunks
- interpreting `event`, `data`, `id`, and `retry`
- assembling normalized events
- exposing protocol-level and event-level `Stream` interfaces

This core is **not** responsible for:

- establishing HTTP connections
- reconnect scheduling or retry timing behavior
- sending `Last-Event-ID` headers
- JSON deserialization as part of the core abstraction

---

## Non-Goals

The following are explicitly out of scope for this crate's core layer:

- browser `EventSource` compatibility at the DOM/task queue level
- HTTP client integration details
- backoff policies
- origin / URL redirect tracking
- task queue semantics
- transport-specific errors beyond wrapping upstream stream errors
- generic payload decoding built into the main stream types

---

## High-Level Model

The implementation is structured around four conceptual layers:

1. **raw line parsing**
2. **protocol state / event building**
3. **protocol stream state machine**
4. **event-only projection**

Conceptually:

```text
Bytes Stream
    ↓
Parser (internal, bytes-only)
    ↓
EventBuilder (internal, protocol semantics)
    ↓
FrameStream<S>
    ↓
EventStream<S>
```

---

## Public API

Only the following types are part of the intended stable public API:

- `Event`
- `Frame`
- `FrameStream<S>`
- `EventStream<S>`

Internal implementation details such as raw lines, parser buffers, and builder state are not public.

---

## Public Types

### `Event`

`Event` is a normalized, validated, immutable value object.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    event: Bytes,
    data: Bytes,
    id: Bytes,
}
```

Fields are private.

#### Accessors

`Event` is intentionally opaque except for accessor methods:

```rust
impl Event {
    pub fn event(&self) -> &str;
    pub fn data(&self) -> &str;
    pub fn id(&self) -> &str;
}
```

Public users do not access raw `Bytes` fields directly.

#### Semantics

##### `event()`

The final event type used at dispatch time.

- if the event type buffer is empty, the value is `"message"`
- otherwise it is the current event type buffer

##### `data()`

The fully assembled event data.

- built from one or more `data:` fields
- joined with `\n` between lines
- if no `data:` field was seen for the dispatch boundary, no `Event` is emitted

##### `id()`

The **effective last-event-id at dispatch time**.

This is not "the `id:` field explicitly carried by the current event". Instead it is the current value of the persistent last-event-id buffer when dispatch occurs.

#### Invariants

All three internal fields are guaranteed to be valid UTF-8.

As a result, the accessor methods may return `&str` directly and may internally rely on `expect(...)` / unchecked invariants, because construction is fully controlled by the crate.

---

### `Frame`

`Frame` is the protocol-level output.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    Event(Event),
    Retry(Duration),
}
```

#### Semantics

- `Frame::Event(Event)` is an emitted message event
- `Frame::Retry(Duration)` is a protocol control instruction derived from a valid `retry:` field

`retry` is intentionally **not** stored on `Event`.

---

### `FrameStream<S>`

`FrameStream<S>` is the protocol-level stream adapter.

It produces:

```rust
Stream<Item = Result<Frame, StreamError<E>>>
```

where the underlying source stream is conceptually:

```rust
Stream<Item = Result<Bytes, E>>
```

`FrameStream` is responsible for:

- reading bytes from the source stream
- incrementally parsing complete lines
- applying SSE protocol semantics
- maintaining stream-level last-event-id state
- yielding either `Frame::Event` or `Frame::Retry`

It is **not** responsible for reconnection behavior.

---

### `EventStream<S>`

`EventStream<S>` is a thin wrapper over `FrameStream<S>`.

It produces:

```rust
Stream<Item = Result<Event, StreamError<E>>>
```

It filters out `Frame::Retry(_)` and yields only `Event`.

#### `last_event_id()`

Both `FrameStream` and `EventStream` should expose a read-only accessor:

```rust
impl<S> FrameStream<S> {
    pub fn last_event_id(&self) -> &str;
}

impl<S> EventStream<S> {
    pub fn last_event_id(&self) -> &str;
}
```

This returns the current effective last-event-id value as normalized UTF-8.

Although the stream stores this state internally as `Bytes`, the public accessor returns `&str` to match the semantic shape of [`Event::id()`] and avoid leaking the storage type through the API boundary.

---

## Internal Types

The following are internal implementation types and are not public API:

- `RawLine`
- `Parser`
- `DataBuffer`
- `EventBuilder`

---

## Internal Raw Parsing Layer

### `RawLine`

Internal representation of a fully parsed line.

```rust
enum RawLine<'a> {
    Empty,
    Comment(&'a [u8]),
    Field {
        name: &'a [u8],
        value: &'a [u8],
    },
}
```

#### Semantics

- `Empty` means a complete empty line
- `Comment` means a complete comment line beginning with `:`
- `Field` means a complete field line

This layer is **bytes-only** and does not perform UTF-8 validation.

---

### Parser

Parser is incremental and operates on a persistent `BytesMut` buffer.

Suggested internal shape:

```rust
struct Parser {
    buf: BytesMut,
    leading_bom_pending: bool,
}
```

#### Responsibilities

- accept arbitrary byte chunks
- find complete line endings
- yield complete `RawLine` values borrowing from the internal buffer
- compact consumed prefix when safe

#### Parser Rules

1. only complete lines are parsed
2. only end-of-line terminates a line
3. EOF does **not** terminate a line
4. trailing unterminated bytes at EOF are discarded
5. parser performs no UTF-8 validation
6. optional UTF-8 BOM is handled only once at stream start, before normal line parsing begins

---

## Internal Protocol State

### `DataBuffer`

`DataBuffer` stores the current event's assembled data in an allocation-aware way.

```rust
#[derive(Debug, Default)]
enum DataBuffer {
    #[default]
    Empty,
    Single(Bytes),
    Multi(BytesMut),
}
```

#### Rationale

Most events contain either:

- no `data:` field
- exactly one `data:` field

This representation avoids allocating a `BytesMut` for the common single-line case.

#### Semantics

- `Empty` means no `data:` field has been seen
- `Single(Bytes)` means exactly one `data:` field has been seen
- `Multi(BytesMut)` means multiple `data:` fields have been seen and joined with `\n`

#### Methods

```rust
impl DataBuffer {
    fn push(&mut self, value: Bytes);
    fn finish(&mut self) -> Option<Bytes>;
}
```

##### `push`

Behavior:

- `Empty -> Single(value)`
- `Single(existing) -> Multi(existing + '\n' + value)`
- `Multi(buf) -> append '\n' + value`

##### `finish`

Behavior:

- `Empty -> None`
- `Single(value) -> Some(value)`
- `Multi(buf) -> Some(buf.freeze())`

#### Important Note

This representation is semantically equivalent to the spec's "append value, then append LF, and trim final LF during dispatch" model, but avoids appending a trailing LF in the common case.

---

### `EventBuilder`

`EventBuilder` is the internal protocol state holder.

```rust
#[derive(Debug, Default)]
struct EventBuilder {
    data: DataBuffer,
    event: Option<Bytes>,
    last_event_id: Bytes,
}
```

#### Fields

##### `data`

Current event's data buffer.

##### `event`

Current event type buffer.

##### `last_event_id`

Persistent last-event-id buffer.

This is stream-level protocol state that survives dispatch.

---

## Field Handling Semantics

The builder interprets validated semantic fields according to the spec.

### `data:`

- validate field value as UTF-8
- push into `DataBuffer`

### `event:`

- validate field value as UTF-8
- replace current event type buffer

### `id:`

- validate field value as UTF-8
- ignore field if it contains `NUL`
- otherwise replace `last_event_id`

### `retry:`

- accept only ASCII digits
- parse as base-10 integer milliseconds
- emit `Frame::Retry(Duration)`

### comment || unknown field

ignored

---

## UTF-8 Validation Strategy

UTF-8 validation belongs to the semantic layer, not the raw parsing layer.

### Not validated in parser

Parser operates purely on bytes and line boundaries.

### Validated in builder

Validation is performed when recognized fields are interpreted:

- `data`
- `event`
- `id`

`retry` is parsed as ASCII digits, not general UTF-8 text.

#### Rationale

This keeps responsibilities clean:

- parser = bytes syntax
- builder = protocol semantics

---

## Dispatch Semantics

Dispatch is spec-aligned, minus browser-specific details such as DOM task queueing and `MessageEvent`.

### Trigger

Dispatch is attempted when an empty line is encountered.

### Behavior

Dispatch follows these protocol semantics:

1. use the current `last_event_id` buffer as the event's effective id
2. if the data buffer is empty:
   - clear current event-specific state
   - do not emit an event
3. otherwise:
   - assemble `data`
   - default event type to `"message"` if no event type was set
   - emit `Event { event, data, id: last_event_id.clone() }`
   - clear event-specific state
   - preserve `last_event_id`

### Event-specific state cleared by dispatch

- `data`
- `event`

### State preserved across dispatch

- `last_event_id`

### Suggested implementation shape

```rust
impl EventBuilder {
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
```

### Note on spec equivalence

The builder does not literally append a trailing LF to every `data:` line and trim it later. Instead, `DataBuffer` stores the equivalent normalized representation directly.

This preserves spec semantics while reducing work.

---

## Stream State Machine

### `FrameStream<S>`

Suggested internal shape:

```rust
pub struct FrameStream<S> {
    source: S,
    parser: Parser,
    builder: EventBuilder,
    terminated: bool,
}
```

#### Fields

##### `source`

Underlying byte stream.

##### `parser`

Internal incremental line parser.

##### `builder`

Internal protocol state.

##### `terminated`

Whether the stream has fully terminated.

After this becomes `true`, all subsequent polls must return `None`.

---

## Core Polling Rules

### Rule 1 — Drain first

Each `poll_next` must first attempt to make progress from already buffered internal state before polling the underlying source.

This means:

- try to read any complete lines already in the parser
- feed them into the builder
- if a frame becomes available, return it immediately

### Rule 2 — Strict EOF

EOF does not terminate a line.

Consequences:

- no implicit completion of partial trailing lines
- no implicit final dispatch
- trailing unterminated bytes are discarded

### Rule 3 — Errors are terminal

Both source errors and protocol errors are fatal for the stream.

Behavior:

- emit exactly one `Err(...)`
- set `terminated = true`
- all future polls return `None`

### Rule 4 — One output per poll

A single `poll_next` call may process multiple lines and may poll the source multiple times, but it must yield at most one item.

Possible outcomes of one poll:

- `Ready(Some(Ok(Frame)))`
- `Ready(Some(Err(...)))`
- `Pending`
- `Ready(None)`

---

## EventStream Behavior

`EventStream<S>` is implemented as a thin adapter over `FrameStream<S>`.

Suggested internal shape:

```rust
pub struct EventStream<S> {
    inner: FrameStream<S>,
}
```

### Behavior

- `Frame::Event(event)` -> return `event`
- `Frame::Retry(_)` -> ignore and continue
- errors propagate unchanged
- termination propagates unchanged

This keeps the protocol-level stream and event-only view separate and simple.

---

## Errors

### `ProtocolError`

```rust
#[derive(Debug)]
pub struct ProtocolError { /* private fields */ }
```

`ProtocolError` is an opaque error value used for recognized textual fields that fail UTF-8 validation.
It should provide meaningful `Display` output and expose the underlying `Utf8Error` through the standard error source chain.

Invalid `retry` values are ignored per SSE rules and do not produce a protocol error.

### `StreamError<E>`

```rust
#[derive(Debug)]
pub enum StreamError<E> {
    Source(E),
    Protocol(ProtocolError),
}
```

---

## Summary of Final Decisions

### Public API

- `Event`, `Frame`, `FrameStream`, `EventStream`
- no public parser/builder/raw-line types

### Event shape

- fields are private
- access via `event()`, `data()`, `id()`

### Event semantics

- normalized
- UTF-8 guaranteed
- `id()` is effective last-event-id at dispatch time

### Retry handling

- `retry` is `Frame::Retry(Duration)`
- not part of `Event`

### Internal builder shape

- single unified `EventBuilder`
- `DataBuffer` with `Empty / Single / Multi`
- persistent `last_event_id`

### Dispatch

- empty data buffer means no event
- clears event-local state only
- preserves `last_event_id`

### Parsing

- strict line-based
- EOF does not complete partial lines

### Stream semantics

- drain internal state before polling source
- errors are terminal
- one output per poll

---

## Recommended Implementation Order

1. implement `Parser`
2. implement `DataBuffer`
3. implement `EventBuilder`
4. implement `FrameStream::poll_next`
5. implement `EventStream`
6. add optional extensions such as JSON helpers

---

## Final One-Sentence Summary

This design implements SSE as a strict incremental bytes protocol, using an internal builder with persistent last-event-id state and an allocation-aware data buffer, while exposing only normalized events and protocol frames through stable futures-based stream interfaces.
