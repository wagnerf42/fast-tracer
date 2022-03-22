use super::{log_event, RawEvent};
use lazy_static::lazy_static;
use std::sync::atomic::AtomicU64;
use tracing::event::Event;
use tracing::field::{Field, Value, Visit};
use tracing::span::{Attributes, Record};
use tracing::subscriber::Subscriber;
use tracing::Id;
use tracing::Metadata;

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
        span.record(&mut FastVisitor(new_id));
        Id::from_u64(new_id)
    }
    fn record(&self, span: &Id, values: &Record) {
        values.record(&mut FastVisitor(span.into_u64()))
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

pub fn initialize_logger() {
    let subscriber: FastSubscriber = FastSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)
        .expect("another subscriber is already registered");
}

struct FastVisitor(u64);

impl Visit for FastVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        let static_str = unsafe { std::mem::transmute::<&str, &'static str>(value) };
        log_event(RawEvent::StrField(self.0, field.name(), static_str));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {}
}
