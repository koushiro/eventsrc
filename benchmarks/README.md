# Benchmarks

Development-only benchmark crate for comparing `eventsrc` against other Rust
SSE implementations on the same in-memory SSE payloads.

- Hardware: Apple M1 Pro
- Toolchain: rustc 1.94.1 (e408947bf 2026-03-25)

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

### Criterion Time Estimates

| Scenario                                | `eventsource-stream` |     `sseer` |  `eventsrc` |
| --------------------------------------- | -------------------: | ----------: | ----------: |
| `json_msg / events-1 / whole-buffer`    |          `631.42 ns` | `359.33 ns` | `305.87 ns` |
| `json_msg / events-1 / chunk-64`        |          `631.17 ns` | `359.38 ns` | `307.20 ns` |
| `json_msg / events-1 / chunk-7`         |          `1.5608 µs` | `849.16 ns` | `782.79 ns` |
| `json_msg / events-32 / whole-buffer`   |          `17.967 µs` | `5.7285 µs` | `3.9863 µs` |
| `json_msg / events-32 / chunk-64`       |          `18.459 µs` | `7.4189 µs` | `5.8470 µs` |
| `json_msg / events-32 / chunk-7`        |          `48.403 µs` | `19.056 µs` | `17.006 µs` |
| `json_msg / events-256 / whole-buffer`  |          `246.94 µs` | `44.574 µs` | `30.641 µs` |
| `json_msg / events-256 / chunk-64`      |          `148.69 µs` | `57.880 µs` | `45.809 µs` |
| `json_msg / events-256 / chunk-7`       |          `396.73 µs` | `157.79 µs` | `143.08 µs` |
| `multiline / events-1 / whole-buffer`   |          `654.75 ns` | `421.24 ns` | `373.92 ns` |
| `multiline / events-1 / chunk-64`       |          `649.17 ns` | `422.24 ns` | `372.17 ns` |
| `multiline / events-1 / chunk-7`        |          `1.4356 µs` | `933.36 ns` | `843.94 ns` |
| `multiline / events-32 / whole-buffer`  |          `20.417 µs` | `7.6133 µs` | `6.1404 µs` |
| `multiline / events-32 / chunk-64`      |          `20.737 µs` | `9.4595 µs` | `7.7787 µs` |
| `multiline / events-32 / chunk-7`       |          `45.770 µs` | `23.779 µs` | `21.417 µs` |
| `multiline / events-256 / whole-buffer` |          `290.81 µs` | `58.170 µs` | `45.736 µs` |
| `multiline / events-256 / chunk-64`     |          `168.07 µs` | `77.043 µs` | `62.746 µs` |
| `multiline / events-256 / chunk-7`      |          `392.90 µs` | `197.88 µs` | `181.24 µs` |

### json_msg

```bash
cargo bench --bench json_msg -- --quiet
# Or `just bench json_msg`
```

```text
json_msg/eventsource-stream: (events-1, whole-buffer)
                        time:   [629.91 ns 631.42 ns 633.03 ns]
json_msg/sseer: (events-1, whole-buffer)
                        time:   [358.24 ns 359.33 ns 360.57 ns]
json_msg/eventsrc: (events-1, whole-buffer)
                        time:   [305.08 ns 305.87 ns 306.77 ns]

json_msg/eventsource-stream: (events-1, chunk-64)
                        time:   [622.11 ns 631.17 ns 649.17 ns]
json_msg/sseer: (events-1, chunk-64)
                        time:   [358.14 ns 359.38 ns 360.78 ns]
json_msg/eventsrc: (events-1, chunk-64)
                        time:   [305.61 ns 307.20 ns 309.15 ns]

json_msg/eventsource-stream: (events-1, chunk-7)
                        time:   [1.5554 µs 1.5608 µs 1.5669 µs]
json_msg/sseer: (events-1, chunk-7)
                        time:   [846.33 ns 849.16 ns 852.46 ns]
json_msg/eventsrc: (events-1, chunk-7)
                        time:   [780.99 ns 782.79 ns 784.84 ns]

json_msg/eventsource-stream: (events-32, whole-buffer)
                        time:   [17.783 µs 17.967 µs 18.251 µs]
json_msg/sseer: (events-32, whole-buffer)
                        time:   [5.6877 µs 5.7285 µs 5.7781 µs]
json_msg/eventsrc: (events-32, whole-buffer)
                        time:   [3.9769 µs 3.9863 µs 3.9967 µs]

json_msg/eventsource-stream: (events-32, chunk-64)
                        time:   [18.370 µs 18.459 µs 18.565 µs]
json_msg/sseer: (events-32, chunk-64)
                        time:   [7.3929 µs 7.4189 µs 7.4536 µs]
json_msg/eventsrc: (events-32, chunk-64)
                        time:   [5.7811 µs 5.8470 µs 5.9770 µs]

json_msg/eventsource-stream: (events-32, chunk-7)
                        time:   [48.281 µs 48.403 µs 48.568 µs]
json_msg/sseer: (events-32, chunk-7)
                        time:   [18.973 µs 19.056 µs 19.187 µs]
json_msg/eventsrc: (events-32, chunk-7)
                        time:   [16.977 µs 17.006 µs 17.040 µs]

json_msg/eventsource-stream: (events-256, whole-buffer)
                        time:   [246.32 µs 246.94 µs 247.63 µs]
json_msg/sseer: (events-256, whole-buffer)
                        time:   [44.080 µs 44.574 µs 45.370 µs]
json_msg/eventsrc: (events-256, whole-buffer)
                        time:   [30.572 µs 30.641 µs 30.723 µs]

json_msg/eventsource-stream: (events-256, chunk-64)
                        time:   [148.24 µs 148.69 µs 149.18 µs]
json_msg/sseer: (events-256, chunk-64)
                        time:   [57.745 µs 57.880 µs 58.043 µs]
json_msg/eventsrc: (events-256, chunk-64)
                        time:   [45.702 µs 45.809 µs 45.937 µs]

json_msg/eventsource-stream: (events-256, chunk-7)
                        time:   [395.37 µs 396.73 µs 398.95 µs]
json_msg/sseer: (events-256, chunk-7)
                        time:   [157.27 µs 157.79 µs 158.36 µs]
json_msg/eventsrc: (events-256, chunk-7)
                        time:   [142.70 µs 143.08 µs 143.51 µs]
```

### multiline

```bash
cargo bench --bench multiline -- --quiet
# Or `just bench multiline`
```

```text
multiline/eventsource-stream: (events-1, whole-buffer)
                        time:   [652.50 ns 654.75 ns 657.30 ns]
multiline/sseer: (events-1, whole-buffer)
                        time:   [420.13 ns 421.24 ns 422.46 ns]
multiline/eventsrc: (events-1, whole-buffer)
                        time:   [372.02 ns 373.92 ns 376.79 ns]

multiline/eventsource-stream: (events-1, chunk-64)
                        time:   [647.67 ns 649.17 ns 650.83 ns]
multiline/sseer: (events-1, chunk-64)
                        time:   [421.04 ns 422.24 ns 423.57 ns]
multiline/eventsrc: (events-1, chunk-64)
                        time:   [370.75 ns 372.17 ns 373.79 ns]

multiline/eventsource-stream: (events-1, chunk-7)
                        time:   [1.4321 µs 1.4356 µs 1.4395 µs]
multiline/sseer: (events-1, chunk-7)
                        time:   [920.87 ns 933.36 ns 956.88 ns]
multiline/eventsrc: (events-1, chunk-7)
                        time:   [841.88 ns 843.94 ns 846.07 ns]

multiline/eventsource-stream: (events-32, whole-buffer)
                        time:   [20.366 µs 20.417 µs 20.471 µs]
multiline/sseer: (events-32, whole-buffer)
                        time:   [7.4235 µs 7.6133 µs 7.8988 µs]
multiline/eventsrc: (events-32, whole-buffer)
                        time:   [6.0078 µs 6.1404 µs 6.3693 µs]

multiline/eventsource-stream: (events-32, chunk-64)
                        time:   [20.232 µs 20.737 µs 21.606 µs]
multiline/sseer: (events-32, chunk-64)
                        time:   [9.4326 µs 9.4595 µs 9.4878 µs]
multiline/eventsrc: (events-32, chunk-64)
                        time:   [7.7553 µs 7.7787 µs 7.8053 µs]

multiline/eventsource-stream: (events-32, chunk-7)
                        time:   [45.651 µs 45.770 µs 45.906 µs]
multiline/sseer: (events-32, chunk-7)
                        time:   [23.705 µs 23.779 µs 23.859 µs]
multiline/eventsrc: (events-32, chunk-7)
                        time:   [21.329 µs 21.417 µs 21.527 µs]

multiline/eventsource-stream: (events-256, whole-buffer)
                        time:   [290.09 µs 290.81 µs 291.56 µs]
multiline/sseer: (events-256, whole-buffer)
                        time:   [57.931 µs 58.170 µs 58.443 µs]
multiline/eventsrc: (events-256, whole-buffer)
                        time:   [45.588 µs 45.736 µs 45.901 µs]

multiline/eventsource-stream: (events-256, chunk-64)
                        time:   [167.62 µs 168.07 µs 168.56 µs]
multiline/sseer: (events-256, chunk-64)
                        time:   [75.767 µs 77.043 µs 78.844 µs]
multiline/eventsrc: (events-256, chunk-64)
                        time:   [62.049 µs 62.746 µs 63.550 µs]

multiline/eventsource-stream: (events-256, chunk-7)
                        time:   [391.63 µs 392.90 µs 394.24 µs]
multiline/sseer: (events-256, chunk-7)
                        time:   [197.26 µs 197.88 µs 198.64 µs]
multiline/eventsrc: (events-256, chunk-7)
                        time:   [180.67 µs 181.24 µs 181.95 µs]
```
