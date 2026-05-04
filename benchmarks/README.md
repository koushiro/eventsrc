# Benchmarks

Development-only benchmark crate for comparing `eventsrc` against other Rust
SSE implementations on the same in-memory SSE payloads.

- Hardware: Apple M1 Pro
- Toolchain: rustc 1.95.0 (59807616e 2026-04-14)

## Compared crates

- `eventsource-stream`
- `sse-core`
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

| Scenario                                | `eventsource-stream` |     `sseer` | `sse-core` |  `eventsrc` |
| --------------------------------------- | -------------------: | ----------: | ---------: | ----------: |
| `json_msg / events-1 / whole-buffer`    |          `779.65 ns` | `454.66 ns` | `529.21 ns` | `386.43 ns` |
| `json_msg / events-1 / chunk-64`        |          `771.12 ns` | `455.51 ns` | `522.62 ns` | `388.40 ns` |
| `json_msg / events-1 / chunk-7`         |          `1.9919 µs` | `1.0835 µs` | `880.17 ns` | `1.0023 µs` |
| `json_msg / events-32 / whole-buffer`   |          `22.357 µs` | `7.2425 µs` | `7.2515 µs` | `5.1123 µs` |
| `json_msg / events-32 / chunk-64`       |          `23.293 µs` | `9.4699 µs` | `8.9093 µs` | `7.4636 µs` |
| `json_msg / events-32 / chunk-7`        |          `62.062 µs` | `24.256 µs` | `18.648 µs` | `21.794 µs` |
| `json_msg / events-256 / whole-buffer`  |          `317.18 µs` | `56.763 µs` | `53.573 µs` | `39.383 µs` |
| `json_msg / events-256 / chunk-64`      |          `186.89 µs` | `75.082 µs` | `68.345 µs` | `58.402 µs` |
| `json_msg / events-256 / chunk-7`       |          `502.76 µs` | `200.44 µs` | `155.78 µs` | `181.82 µs` |
| `multiline / events-1 / whole-buffer`   |          `810.66 ns` | `531.81 ns` | `512.68 ns` | `462.11 ns` |
| `multiline / events-1 / chunk-64`       |          `810.46 ns` | `533.00 ns` | `514.61 ns` | `462.33 ns` |
| `multiline / events-1 / chunk-7`        |          `1.7952 µs` | `1.1417 µs` | `771.51 ns` | `1.0810 µs` |
| `multiline / events-32 / whole-buffer`  |          `25.895 µs` | `9.9146 µs` | `7.6548 µs` | `7.4832 µs` |
| `multiline / events-32 / chunk-64`      |          `24.853 µs` | `11.921 µs` | `9.0718 µs` | `9.8328 µs` |
| `multiline / events-32 / chunk-7`       |          `57.690 µs` | `30.080 µs` | `17.294 µs` | `27.141 µs` |
| `multiline / events-256 / whole-buffer` |          `367.89 µs` | `73.561 µs` | `57.058 µs` | `57.747 µs` |
| `multiline / events-256 / chunk-64`     |          `209.17 µs` | `95.060 µs` | `70.178 µs` | `78.344 µs` |
| `multiline / events-256 / chunk-7`      |          `494.93 µs` | `255.03 µs` | `159.59 µs` | `242.24 µs` |

### json_msg

```bash
cargo bench --bench json_msg -- --quiet
# Or `just bench json_msg`
```

```text
json_msg/eventsource-stream: (events-1, whole-buffer)
                        time:   [773.95 ns 779.65 ns 787.12 ns]
json_msg/sseer: (events-1, whole-buffer)
                        time:   [452.60 ns 454.66 ns 457.46 ns]
json_msg/sse-core: (events-1, whole-buffer)
                        time:   [524.40 ns 529.21 ns 535.16 ns]
json_msg/eventsrc: (events-1, whole-buffer)
                        time:   [384.12 ns 386.43 ns 389.46 ns]

json_msg/eventsource-stream: (events-1, chunk-64)
                        time:   [768.35 ns 771.12 ns 775.18 ns]
json_msg/sseer: (events-1, chunk-64)
                        time:   [453.21 ns 455.51 ns 458.76 ns]
json_msg/sse-core: (events-1, chunk-64)
                        time:   [519.03 ns 522.62 ns 527.02 ns]
json_msg/eventsrc: (events-1, chunk-64)
                        time:   [385.56 ns 388.40 ns 392.12 ns]

json_msg/eventsource-stream: (events-1, chunk-7)
                        time:   [1.9755 µs 1.9919 µs 2.0133 µs]
json_msg/sseer: (events-1, chunk-7)
                        time:   [1.0783 µs 1.0835 µs 1.0907 µs]
json_msg/sse-core: (events-1, chunk-7)
                        time:   [877.10 ns 880.17 ns 884.00 ns]
json_msg/eventsrc: (events-1, chunk-7)
                        time:   [993.64 ns 1.0023 µs 1.0146 µs]

json_msg/eventsource-stream: (events-32, whole-buffer)
                        time:   [22.209 µs 22.357 µs 22.575 µs]
json_msg/sseer: (events-32, whole-buffer)
                        time:   [7.1977 µs 7.2425 µs 7.3036 µs]
json_msg/sse-core: (events-32, whole-buffer)
                        time:   [7.2080 µs 7.2515 µs 7.3227 µs]
json_msg/eventsrc: (events-32, whole-buffer)
                        time:   [5.0906 µs 5.1123 µs 5.1497 µs]

json_msg/eventsource-stream: (events-32, chunk-64)
                        time:   [23.187 µs 23.293 µs 23.425 µs]
json_msg/sseer: (events-32, chunk-64)
                        time:   [9.4142 µs 9.4699 µs 9.5474 µs]
json_msg/sse-core: (events-32, chunk-64)
                        time:   [8.8435 µs 8.9093 µs 8.9951 µs]
json_msg/eventsrc: (events-32, chunk-64)
                        time:   [7.4160 µs 7.4636 µs 7.5209 µs]

json_msg/eventsource-stream: (events-32, chunk-7)
                        time:   [61.611 µs 62.062 µs 62.624 µs]
json_msg/sseer: (events-32, chunk-7)
                        time:   [24.162 µs 24.256 µs 24.402 µs]
json_msg/sse-core: (events-32, chunk-7)
                        time:   [18.554 µs 18.648 µs 18.804 µs]
json_msg/eventsrc: (events-32, chunk-7)
                        time:   [21.642 µs 21.794 µs 22.023 µs]

json_msg/eventsource-stream: (events-256, whole-buffer)
                        time:   [315.53 µs 317.18 µs 319.40 µs]
json_msg/sseer: (events-256, whole-buffer)
                        time:   [56.403 µs 56.763 µs 57.282 µs]
json_msg/sse-core: (events-256, whole-buffer)
                        time:   [53.212 µs 53.573 µs 54.122 µs]
json_msg/eventsrc: (events-256, whole-buffer)
                        time:   [39.205 µs 39.383 µs 39.661 µs]

json_msg/eventsource-stream: (events-256, chunk-64)
                        time:   [185.17 µs 186.89 µs 189.17 µs]
json_msg/sseer: (events-256, chunk-64)
                        time:   [74.064 µs 75.082 µs 77.053 µs]
json_msg/sse-core: (events-256, chunk-64)
                        time:   [67.924 µs 68.345 µs 68.894 µs]
json_msg/eventsrc: (events-256, chunk-64)
                        time:   [58.014 µs 58.402 µs 59.005 µs]

json_msg/eventsource-stream: (events-256, chunk-7)
                        time:   [499.55 µs 502.76 µs 507.20 µs]
json_msg/sseer: (events-256, chunk-7)
                        time:   [199.28 µs 200.44 µs 202.23 µs]
json_msg/sse-core: (events-256, chunk-7)
                        time:   [155.34 µs 155.78 µs 156.32 µs]
json_msg/eventsrc: (events-256, chunk-7)
                        time:   [180.99 µs 181.82 µs 183.25 µs]
```

### multiline

```bash
cargo bench --bench multiline -- --quiet
# Or `just bench multiline`
```

```text
multiline/eventsource-stream: (events-1, whole-buffer)
                        time:   [806.91 ns 810.66 ns 815.64 ns]
multiline/sseer: (events-1, whole-buffer)
                        time:   [529.00 ns 531.81 ns 535.65 ns]
multiline/sse-core: (events-1, whole-buffer)
                        time:   [508.43 ns 512.68 ns 520.17 ns]
multiline/eventsrc: (events-1, whole-buffer)
                        time:   [459.43 ns 462.11 ns 465.71 ns]

multiline/eventsource-stream: (events-1, chunk-64)
                        time:   [806.05 ns 810.46 ns 816.41 ns]
multiline/sseer: (events-1, chunk-64)
                        time:   [529.52 ns 533.00 ns 537.74 ns]
multiline/sse-core: (events-1, chunk-64)
                        time:   [509.63 ns 514.61 ns 521.93 ns]
multiline/eventsrc: (events-1, chunk-64)
                        time:   [459.45 ns 462.33 ns 466.60 ns]

multiline/eventsource-stream: (events-1, chunk-7)
                        time:   [1.7858 µs 1.7952 µs 1.8101 µs]
multiline/sseer: (events-1, chunk-7)
                        time:   [1.1363 µs 1.1417 µs 1.1490 µs]
multiline/sse-core: (events-1, chunk-7)
                        time:   [768.95 ns 771.51 ns 775.96 ns]
multiline/eventsrc: (events-1, chunk-7)
                        time:   [1.0662 µs 1.0810 µs 1.1003 µs]

multiline/eventsource-stream: (events-32, whole-buffer)
                        time:   [25.765 µs 25.895 µs 26.076 µs]
multiline/sseer: (events-32, whole-buffer)
                        time:   [9.5771 µs 9.9146 µs 10.474 µs]
multiline/sse-core: (events-32, whole-buffer)
                        time:   [7.6232 µs 7.6548 µs 7.6941 µs]
multiline/eventsrc: (events-32, whole-buffer)
                        time:   [7.4321 µs 7.4832 µs 7.5605 µs]

multiline/eventsource-stream: (events-32, chunk-64)
                        time:   [24.765 µs 24.853 µs 24.963 µs]
multiline/sseer: (events-32, chunk-64)
                        time:   [11.870 µs 11.921 µs 11.989 µs]
multiline/sse-core: (events-32, chunk-64)
                        time:   [9.0344 µs 9.0718 µs 9.1185 µs]
multiline/eventsrc: (events-32, chunk-64)
                        time:   [9.7534 µs 9.8328 µs 9.9338 µs]

multiline/eventsource-stream: (events-32, chunk-7)
                        time:   [57.294 µs 57.690 µs 58.234 µs]
multiline/sseer: (events-32, chunk-7)
                        time:   [29.876 µs 30.080 µs 30.370 µs]
multiline/sse-core: (events-32, chunk-7)
                        time:   [17.174 µs 17.294 µs 17.457 µs]
multiline/eventsrc: (events-32, chunk-7)
                        time:   [26.961 µs 27.141 µs 27.392 µs]

multiline/eventsource-stream: (events-256, whole-buffer)
                        time:   [366.38 µs 367.89 µs 369.94 µs]
multiline/sseer: (events-256, whole-buffer)
                        time:   [73.253 µs 73.561 µs 73.941 µs]
multiline/sse-core: (events-256, whole-buffer)
                        time:   [56.699 µs 57.058 µs 57.515 µs]
multiline/eventsrc: (events-256, whole-buffer)
                        time:   [57.507 µs 57.747 µs 58.090 µs]

multiline/eventsource-stream: (events-256, chunk-64)
                        time:   [207.67 µs 209.17 µs 211.05 µs]
multiline/sseer: (events-256, chunk-64)
                        time:   [94.589 µs 95.060 µs 95.685 µs]
multiline/sse-core: (events-256, chunk-64)
                        time:   [69.816 µs 70.178 µs 70.681 µs]
multiline/eventsrc: (events-256, chunk-64)
                        time:   [77.859 µs 78.344 µs 79.056 µs]

multiline/eventsource-stream: (events-256, chunk-7)
                        time:   [491.30 µs 494.93 µs 500.31 µs]
multiline/sseer: (events-256, chunk-7)
                        time:   [252.16 µs 255.03 µs 259.46 µs]
multiline/sse-core: (events-256, chunk-7)
                        time:   [154.57 µs 159.59 µs 166.43 µs]
multiline/eventsrc: (events-256, chunk-7)
                        time:   [235.11 µs 242.24 µs 252.11 µs]
```
