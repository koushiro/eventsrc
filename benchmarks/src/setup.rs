use std::convert::Infallible;

use bytes::Bytes;
use futures_util::{StreamExt, pin_mut, stream};

#[derive(Clone, Copy, Debug)]
pub struct ChunkCase {
    pub name: &'static str,
    pub chunk_size: usize,
}

pub fn chunk_cases() -> [ChunkCase; 3] {
    [
        ChunkCase { name: "whole-buffer", chunk_size: usize::MAX },
        ChunkCase { name: "chunk-64", chunk_size: 64 },
        ChunkCase { name: "chunk-7", chunk_size: 7 },
    ]
}

pub fn payload_chunks(payload: &[u8], chunk_size: usize) -> Vec<Bytes> {
    if chunk_size == usize::MAX || chunk_size >= payload.len() {
        return vec![Bytes::copy_from_slice(payload)];
    }

    payload.chunks(chunk_size).map(Bytes::copy_from_slice).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsumeStats {
    pub events: usize,
    pub score: usize,
}

pub async fn consume_eventsource_stream(chunks: Vec<Bytes>) -> ConsumeStats {
    let stream = stream::iter(chunks.into_iter().map(Ok::<_, Infallible>));

    let stream = eventsource_stream::EventStream::new(stream);
    pin_mut!(stream);

    let mut stats = ConsumeStats { events: 0, score: 0 };

    while let Some(event) = stream.next().await {
        let event = event.expect("eventsource-stream parse failed");
        stats.events += 1;
        stats.score += event.data.len() + event.event.len() + event.id.len();
    }

    stats
}

pub async fn consume_sseer(chunks: Vec<Bytes>) -> ConsumeStats {
    let stream = stream::iter(chunks.into_iter().map(Ok::<_, Infallible>));

    let mut stream = sseer::EventStream::new(stream);

    let mut stats = ConsumeStats { events: 0, score: 0 };

    while let Some(event) = stream.next().await {
        let event = event.expect("sseer parse failed");
        stats.events += 1;
        stats.score += event.data.len() + event.event.len() + event.id.len();
    }

    stats
}

pub async fn consume_eventsrc(chunks: Vec<Bytes>) -> ConsumeStats {
    let stream = stream::iter(chunks.into_iter().map(Ok::<_, Infallible>));

    let mut stream = eventsrc::EventStream::new(stream);

    let mut stats = ConsumeStats { events: 0, score: 0 };

    while let Some(event) = stream.next().await {
        let event = event.expect("eventsrc parse failed");
        stats.events += 1;
        stats.score += event.data().len() + event.event().len() + event.id().len();
    }

    stats
}

#[tokio::test]
async fn all_benchmark_adapters_agree_on_event_counts_and_scores() {
    use std::fmt::Write;

    #[derive(Clone, Debug)]
    struct Case {
        name: &'static str,
        payload: Vec<u8>,
    }

    fn build_event_payload(events: usize) -> Vec<u8> {
        let mut payload = String::new();

        for index in 0..events {
            let _ = write!(
                payload,
                "event: msg\nid: msg-{index}\ndata: {{\"msg\":\"hello {index}\",\"done\":false}}\n\n"
            );
        }

        payload.into_bytes()
    }

    let cases = [
        Case { name: "128 events", payload: build_event_payload(128) },
        Case { name: "256 events", payload: build_event_payload(256) },
    ];

    for case in cases {
        for pattern in chunk_cases() {
            let chunks = payload_chunks(&case.payload, pattern.chunk_size);

            let eventsrc = consume_eventsrc(chunks.clone()).await;
            let eventsource_stream = consume_eventsource_stream(chunks.clone()).await;
            let sseer = consume_sseer(chunks.clone()).await;

            assert_eq!(
                eventsource_stream, eventsrc,
                "eventsource-stream mismatch for {}",
                case.name
            );
            assert_eq!(sseer, eventsrc, "sseer mismatch for {}", case.name);
        }
    }
}
