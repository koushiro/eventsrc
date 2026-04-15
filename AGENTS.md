# AGENTS.md

## Project structure

This project is a Rust SSE/EventSource client library with:

- eventsrc (protocol)
- eventsrc-client (client layer with reqwest adapter)

Source of truth:

- docs/core.md
- docs/client.md

---

## Core rules

- NEVER collapse the oneshot and replayable EventSource modes
- eventsrc must NOT depend on reqwest
- reqwest is the only backend adapter in v0.1
- No server-side SSE
- No extra transports

---

## Architecture constraints

- core = protocol / parser / stream
- client = oneshot / replayable / adapter boundary / reconnect

---

## Critical design rules

### 1. Two distinct modes

- oneshot::EventSource = one-shot
- replayable::EventSource = reconnecting / request replay

Do NOT collapse.

---

### 2. Parser must be transport-agnostic

No async/runtime coupling in parser.

---

### 3. Retry is protocol directive

Do NOT force into Event.

---

### 4. Request replay must be explicit

Do NOT rely on cloning RequestBuilder.

---

### 5. Avoid over-engineering

Keep backend abstraction minimal.

Current public backend-facing boundary:

- `replayable::Connector`

Do not introduce a larger transport/client trait hierarchy.

---

## Implementation workflow

Before coding:

1. Read docs
2. Summarize plan (5–10 bullets)
3. Identify ambiguities

After coding:

- files changed
- API changes
- tests added
- next steps

---

## Testing priorities

Core:

- data/event/id/retry
- multi-line data
- comments
- blank-line dispatch
- chunk boundaries
- UTF-8

Client / reqwest adapter:

- valid streaming
- invalid status
- invalid content-type
- reconnect
- Last-Event-ID

---

## Scope discipline

If something requires redesign:

- STOP
- explain issue
- propose minimal change
