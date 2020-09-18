use super::{log_event, RawEvent};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::collections::LinkedList;
use std::sync::atomic::AtomicU64;
use tracing::event::Event;
use tracing::span::{Attributes, Record};
use tracing::subscriber::Subscriber;
use tracing::Metadata;
use tracing::{span, Id, Level, Span};
use tracing_core::span::Current;

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

const CAPACITY: usize = 4096;

thread_local! {
    /// remember all current spans.
    // invariant : last element if any is in the last block.
    // TODO: should we switch to atomic list to avoid the refcell ?
    static SPANS_STACK: RefCell<LinkedList<Vec<Id>>> = RefCell::new(std::iter::once(Vec::with_capacity(CAPACITY)).collect());
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
    fn current_span(&self) -> Current {
        SPANS_STACK.with(|stack| {
            if let Some(current_id) = stack
                .borrow()
                .back()
                .and_then(|block| block.last())
                .cloned()
            {
                Current::new(current_id, FAKE_SPAN.metadata().unwrap())
            } else {
                Current::none()
            }
        })
    }
    fn enter(&self, span: &Id) {
        SPANS_STACK.with(|stack| {
            if stack
                .borrow()
                .back()
                .map(|block| block.len())
                .unwrap_or(CAPACITY)
                == CAPACITY
            {
                stack.borrow_mut().push_back(Vec::with_capacity(CAPACITY));
            }
            stack
                .borrow_mut()
                .back_mut()
                .map(|block| block.push(span.clone()));
        });
        log_event(RawEvent::Enter(span.into_u64(), now()))
    }
    fn exit(&self, span: &Id) {
        log_event(RawEvent::Exit(span.into_u64(), now()));
        SPANS_STACK.with(|stack| {
            let removed = stack.borrow_mut().back_mut().and_then(|block| block.pop());
            assert_eq!(removed, Some(span.clone()));
            if stack
                .borrow()
                .back()
                .map(|block| block.is_empty())
                .unwrap_or(false)
                && removed.is_some()
            {
                stack.borrow_mut().pop_back();
            }
        })
    }
}
