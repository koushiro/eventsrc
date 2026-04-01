use std::{fmt::Write as _, hint::black_box};

use criterion::{Criterion, criterion_group, criterion_main};

mod setup;

fn build_json_msg_payload(events: usize) -> Vec<u8> {
    let mut payload = String::new();

    for index in 0..events {
        let _ = write!(
            payload,
            "event: msg\nid: msg-{index}\ndata: {{\"msg\":\"hello {index}\",\"done\":false}}\n\n"
        );
    }

    payload.into_bytes()
}

fn json_msg(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("benchmark runtime");

    for events in [1, 32, 256] {
        let payload = build_json_msg_payload(events);

        for chunk in setup::chunk_cases() {
            let mut group = c.benchmark_group("json_msg");

            group.bench_with_input(
                format!("eventsource-stream: (events-{events}, {})", chunk.name),
                &(payload.as_slice(), chunk.chunk_size),
                |b, (payload, chunk_size)| {
                    b.iter(|| {
                        let chunks = setup::payload_chunks(payload, *chunk_size);
                        black_box(runtime.block_on(setup::consume_eventsource_stream(chunks)))
                    });
                },
            );

            group.bench_with_input(
                format!("sseer: (events-{events}, {})", chunk.name),
                &(payload.as_slice(), chunk.chunk_size),
                |b, (payload, chunk_size)| {
                    b.iter(|| {
                        let chunks = setup::payload_chunks(payload, *chunk_size);
                        black_box(runtime.block_on(setup::consume_sseer(chunks)))
                    });
                },
            );

            group.bench_with_input(
                format!("eventsrc: (events-{events}, {})", chunk.name),
                &(payload.as_slice(), chunk.chunk_size),
                |b, (payload, chunk_size)| {
                    b.iter(|| {
                        let chunks = setup::payload_chunks(payload, *chunk_size);
                        black_box(runtime.block_on(setup::consume_eventsrc(chunks)))
                    });
                },
            );

            group.finish();
        }
    }
}

criterion_group!(benches, json_msg);
criterion_main!(benches);
