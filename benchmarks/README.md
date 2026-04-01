# Benchmarks

Development-only benchmark crate for comparing `eventsrc` against other Rust
SSE implementations on the same in-memory SSE payloads.

- Hardware: Apple M1 Pro
- Toolchain: rustc 1.94.0 (4a4ef493e 2026-03-02)

## Compared crates

- `eventsource-stream`
- `sseer`
- `eventsrc`

## What is measured

The benchmark focuses on the parser / byte-stream consumption path:

- fixed SSE payloads
- deterministic chunking patterns
- no real network or TLS
- no proxy, socket, or server jitter

This keeps the comparison centered on SSE parsing and stream adaptation rather
than HTTP stack variance.

## Scenarios

- `json_msg`
- `multiline`

Each scenario is executed with:

- `whole-buffer`
- `chunk-64`
- `chunk-7`

Both scenarios currently use `1/32/256` events.

## Results

### Median Times

| Scenario                                | `eventsource-stream` |     `sseer` |  `eventsrc` |
| --------------------------------------- | -------------------: | ----------: | ----------: |
| `json_msg / events-1 / whole-buffer`    |          `625.81 ns` | `360.52 ns` | `307.82 ns` |
| `json_msg / events-1 / chunk-64`        |          `616.30 ns` | `359.53 ns` | `302.42 ns` |
| `json_msg / events-1 / chunk-7`         |          `1.5353 µs` | `868.91 ns` | `778.53 ns` |
| `json_msg / events-32 / whole-buffer`   |          `17.440 µs` | `5.6578 µs` | `3.9814 µs` |
| `json_msg / events-32 / chunk-64`       |          `18.407 µs` | `7.3827 µs` | `5.8420 µs` |
| `json_msg / events-32 / chunk-7`        |          `48.138 µs` | `19.443 µs` | `16.988 µs` |
| `json_msg / events-256 / whole-buffer`  |          `246.65 µs` | `44.048 µs` | `30.647 µs` |
| `json_msg / events-256 / chunk-64`      |          `149.21 µs` | `57.825 µs` | `46.120 µs` |
| `json_msg / events-256 / chunk-7`       |          `396.05 µs` | `159.48 µs` | `141.44 µs` |
| `multiline / events-1 / whole-buffer`   |          `657.61 ns` | `419.37 ns` | `365.38 ns` |
| `multiline / events-1 / chunk-64`       |          `642.92 ns` | `420.07 ns` | `370.79 ns` |
| `multiline / events-1 / chunk-7`        |          `1.4257 µs` | `923.62 ns` | `835.71 ns` |
| `multiline / events-32 / whole-buffer`  |          `20.255 µs` | `7.3871 µs` | `5.8879 µs` |
| `multiline / events-32 / chunk-64`      |          `19.781 µs` | `9.3564 µs` | `7.8620 µs` |
| `multiline / events-32 / chunk-7`       |          `45.463 µs` | `23.580 µs` | `21.264 µs` |
| `multiline / events-256 / whole-buffer` |          `284.22 µs` | `57.641 µs` | `45.958 µs` |
| `multiline / events-256 / chunk-64`     |          `165.37 µs` | `74.781 µs` | `61.455 µs` |
| `multiline / events-256 / chunk-7`      |          `390.17 µs` | `197.03 µs` | `182.20 µs` |

### json_msg

```bash
cargo bench --bench json_msg -- --quiet
# Or `just bench json_msg`
```

```text
json_msg/eventsource-stream: (events-1, whole-buffer)
                        time:   [625.15 ns 625.81 ns 626.53 ns]
json_msg/sseer: (events-1, whole-buffer)
                        time:   [359.84 ns 360.52 ns 361.34 ns]
json_msg/eventsrc: (events-1, whole-buffer)
                        time:   [302.81 ns 307.82 ns 315.01 ns]

json_msg/eventsource-stream: (events-1, chunk-64)
                        time:   [615.56 ns 616.30 ns 617.07 ns]
json_msg/sseer: (events-1, chunk-64)
                        time:   [359.00 ns 359.53 ns 360.12 ns]
json_msg/eventsrc: (events-1, chunk-64)
                        time:   [301.96 ns 302.42 ns 302.94 ns]

json_msg/eventsource-stream: (events-1, chunk-7)
                        time:   [1.5326 µs 1.5353 µs 1.5384 µs]
json_msg/sseer: (events-1, chunk-7)
                        time:   [856.65 ns 868.91 ns 888.25 ns]
json_msg/eventsrc: (events-1, chunk-7)
                        time:   [777.40 ns 778.53 ns 779.75 ns]

json_msg/eventsource-stream: (events-32, whole-buffer)
                        time:   [17.401 µs 17.440 µs 17.482 µs]
json_msg/sseer: (events-32, whole-buffer)
                        time:   [5.6475 µs 5.6578 µs 5.6697 µs]
json_msg/eventsrc: (events-32, whole-buffer)
                        time:   [3.9745 µs 3.9814 µs 3.9884 µs]

json_msg/eventsource-stream: (events-32, chunk-64)
                        time:   [18.376 µs 18.407 µs 18.441 µs]
json_msg/sseer: (events-32, chunk-64)
                        time:   [7.3661 µs 7.3827 µs 7.4026 µs]
json_msg/eventsrc: (events-32, chunk-64)
                        time:   [5.7522 µs 5.8420 µs 6.0314 µs]

json_msg/eventsource-stream: (events-32, chunk-7)
                        time:   [48.082 µs 48.138 µs 48.194 µs]
json_msg/sseer: (events-32, chunk-7)
                        time:   [19.316 µs 19.443 µs 19.565 µs]
json_msg/eventsrc: (events-32, chunk-7)
                        time:   [16.934 µs 16.988 µs 17.062 µs]

json_msg/eventsource-stream: (events-256, whole-buffer)
                        time:   [246.23 µs 246.65 µs 247.13 µs]
json_msg/sseer: (events-256, whole-buffer)
                        time:   [43.956 µs 44.048 µs 44.166 µs]
json_msg/eventsrc: (events-256, whole-buffer)
                        time:   [30.605 µs 30.647 µs 30.692 µs]

json_msg/eventsource-stream: (events-256, chunk-64)
                        time:   [147.77 µs 149.21 µs 151.49 µs]
json_msg/sseer: (events-256, chunk-64)
                        time:   [57.741 µs 57.825 µs 57.919 µs]
json_msg/eventsrc: (events-256, chunk-64)
                        time:   [45.561 µs 46.120 µs 47.215 µs]

json_msg/eventsource-stream: (events-256, chunk-7)
                        time:   [395.30 µs 396.05 µs 396.86 µs]
json_msg/sseer: (events-256, chunk-7)
                        time:   [156.75 µs 159.48 µs 164.75 µs]
json_msg/eventsrc: (events-256, chunk-7)
                        time:   [141.16 µs 141.44 µs 141.79 µs]
```

### multiline

```bash
cargo bench --bench multiline -- --quiet
# Or `just bench multiline`
```

```text
multiline/eventsource-stream: (events-1, whole-buffer)
                        time:   [648.93 ns 657.61 ns 671.14 ns]
multiline/sseer: (events-1, whole-buffer)
                        time:   [418.66 ns 419.37 ns 420.15 ns]
multiline/eventsrc: (events-1, whole-buffer)
                        time:   [364.18 ns 365.38 ns 366.98 ns]

multiline/eventsource-stream: (events-1, chunk-64)
                        time:   [641.75 ns 642.92 ns 644.29 ns]
multiline/sseer: (events-1, chunk-64)
                        time:   [419.41 ns 420.07 ns 420.80 ns]
multiline/eventsrc: (events-1, chunk-64)
                        time:   [364.33 ns 370.79 ns 382.68 ns]

multiline/eventsource-stream: (events-1, chunk-7)
                        time:   [1.4231 µs 1.4257 µs 1.4287 µs]
multiline/sseer: (events-1, chunk-7)
                        time:   [907.85 ns 923.62 ns 948.74 ns]
multiline/eventsrc: (events-1, chunk-7)
                        time:   [832.86 ns 835.71 ns 839.21 ns]

multiline/eventsource-stream: (events-32, whole-buffer)
                        time:   [20.225 µs 20.255 µs 20.286 µs]
multiline/sseer: (events-32, whole-buffer)
                        time:   [7.3706 µs 7.3871 µs 7.4069 µs]
multiline/eventsrc: (events-32, whole-buffer)
                        time:   [5.8710 µs 5.8879 µs 5.9052 µs]

multiline/eventsource-stream: (events-32, chunk-64)
                        time:   [19.758 µs 19.781 µs 19.808 µs]
multiline/sseer: (events-32, chunk-64)
                        time:   [9.3412 µs 9.3564 µs 9.3729 µs]
multiline/eventsrc: (events-32, chunk-64)
                        time:   [7.7455 µs 7.8620 µs 8.0354 µs]

multiline/eventsource-stream: (events-32, chunk-7)
                        time:   [45.367 µs 45.463 µs 45.602 µs]
multiline/sseer: (events-32, chunk-7)
                        time:   [23.534 µs 23.580 µs 23.635 µs]
multiline/eventsrc: (events-32, chunk-7)
                        time:   [21.225 µs 21.264 µs 21.310 µs]

multiline/eventsource-stream: (events-256, whole-buffer)
                        time:   [283.87 µs 284.22 µs 284.58 µs]
multiline/sseer: (events-256, whole-buffer)
                        time:   [57.522 µs 57.641 µs 57.773 µs]
multiline/eventsrc: (events-256, whole-buffer)
                        time:   [45.569 µs 45.958 µs 46.419 µs]

multiline/eventsource-stream: (events-256, chunk-64)
                        time:   [165.15 µs 165.37 µs 165.63 µs]
multiline/sseer: (events-256, chunk-64)
                        time:   [74.603 µs 74.781 µs 74.980 µs]
multiline/eventsrc: (events-256, chunk-64)
                        time:   [61.321 µs 61.455 µs 61.586 µs]

multiline/eventsource-stream: (events-256, chunk-7)
                        time:   [389.11 µs 390.17 µs 391.41 µs]
multiline/sseer: (events-256, chunk-7)
                        time:   [196.71 µs 197.03 µs 197.37 µs]
multiline/eventsrc: (events-256, chunk-7)
                        time:   [181.08 µs 182.20 µs 183.82 µs]
```
