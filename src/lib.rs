// atomic list
mod list;
// list of blocks to store events
mod storage;
use storage::Storage;
// the subscriber used by tracing to record spans and events
mod subscriber;
pub use subscriber::{initialize_logger, FastSubscriber};
// stored events
mod events;
use events::{extract_spans, log_event, reset_events, RawEvent};
mod spans;
use spans::Span;
mod graph;
use graph::{Graph, Node};
mod svg;
use itertools::Itertools;
use std::collections::HashMap;
pub use svg::{display_svg, gantt_svg, svg};
use svg::{SVG_HEIGHT, SVG_WIDTH};
use tracing::{span, Level};

pub fn stats<R, F: FnOnce() -> R>(op: F) -> R {
    let subscriber: FastSubscriber = FastSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).err();
    reset_events();
    let span = span!(Level::TRACE, "main_task");
    let r = {
        let _enter = span.enter();
        op()
    };
    let spans = extract_spans();

    let span_hash = spans.values().fold(HashMap::new(), |mut h, s| {
        h.entry(s.name)
            .or_insert_with(Vec::new)
            .push(s.end - s.start);
        h
    });

    let main_duration = span_hash.get("main_task").unwrap()[0];

    for (name, spans) in span_hash
        .into_iter()
        .sorted_by(|(n1, _), (n2, _)| n1.cmp(&n2))
    {
        let sum = spans.iter().sum::<u128>();
        let average = sum / spans.len() as u128;
        println!(
            "{}: {}ns avg ({} spans): {}%, total: {}ns",
            name,
            average,
            spans.len(),
            (sum as f64 / main_duration as f64) * 100.0,
            sum
        );
    }
    r
}
