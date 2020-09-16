use super::storage::Storage;
use lazy_static::lazy_static;
use std::sync::atomic::AtomicU64;
use tracing::event::Event;
use tracing::span::{Attributes, Record};
use tracing::subscriber::Subscriber;
use tracing::Metadata;
use tracing::{span, Id, Level, Span};
use tracing_core::span::Current;

struct FastSubscriber {
    next_task_id: AtomicU64,
}

impl FastSubscriber {
    fn new() -> Self {
        FastSubscriber {
            next_task_id: AtomicU64::new(1),
        }
    }
}

thread_local! {
    /// remember all current spans
    static SPANS_STACK: Storage<Id> = Storage::new();
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
    fn new_span(&self, _span: &Attributes) -> Id {
        Id::from_u64(
            self.next_task_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        )
    }
    fn record(&self, span: &Id, values: &Record) {
        unimplemented!()
    }
    fn record_follows_from(&self, span: &Id, follows: &Id) {
        unimplemented!()
    }
    fn event(&self, event: &Event) {
        unimplemented!()
    }
    fn current_span(&self) -> Current {
        SPANS_STACK.with(|stack| {
            if let Some(current_id) = stack.last().cloned() {
                Current::new(current_id, FAKE_SPAN.metadata().unwrap())
            } else {
                Current::none()
            }
        })
    }
    fn enter(&self, span: &Id) {
        SPANS_STACK.with(|stack| {
            stack.push(span.clone());
        })
    }
    fn exit(&self, span: &Id) {
        SPANS_STACK.with(|stack| {
            let removed = stack.pop();
            assert_eq!(removed, Some(span.clone()));
        })
    }
}
