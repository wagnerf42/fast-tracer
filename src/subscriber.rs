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
        })
    }
    fn exit(&self, span: &Id) {
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
