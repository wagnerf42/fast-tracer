use super::{log_event, RawEvent};
use lazy_static::lazy_static;
use std::sync::atomic::AtomicU64;
use tracing::event::Event;
use tracing::span::{Attributes, Record};
use tracing::subscriber::Subscriber;
use tracing::Metadata;
use tracing::{span, Id, Level, Span};

lazy_static! {
    static ref START: std::time::Instant = std::time::Instant::now();
}

fn now() -> u128 {
    START.elapsed().as_nanos()
}

pub struct FastSubscriber {
    next_task_id: AtomicU64,
}

impl FastSubscriber {
    pub fn new() -> Self {
        FastSubscriber {
            next_task_id: AtomicU64::new(1),
        }
    }
}

lazy_static! {
    /// There is a problem with `Current` it requires a metadata
    /// which we do not store (would require a shared hash table).
    /// We need to feed him a fake one but it is not possible to build one.
    static ref FAKE_SPAN: Span = span!(Level::TRACE, "fake");
}

impl Subscriber for FastSubscriber {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn new_span(&self, span: &Attributes) -> Id {
        let new_id = self
            .next_task_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let parent = span.parent().map(|p| p.into_u64()).unwrap_or(0);
        let name = span.metadata().name();
        log_event(RawEvent::NewSpan(new_id, name, parent));
        Id::from_u64(new_id)
    }
    fn record(&self, _span: &Id, _values: &Record) {
        unimplemented!()
    }
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {
        unimplemented!()
    }
    fn event(&self, _event: &Event) {
        unimplemented!()
    }
    fn enter(&self, span: &Id) {
        log_event(RawEvent::Enter(span.into_u64(), now()))
    }
    fn exit(&self, span: &Id) {
        log_event(RawEvent::Exit(span.into_u64(), now()));
    }
}
