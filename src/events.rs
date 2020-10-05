//! Events and the places they are stored into.
use super::{Span, Storage};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::sync::Arc;
use std::sync::Mutex;

pub(super) enum RawEvent {
    NewSpan(u64, &'static str, u64),
    Record(u64, &'static str, u64),
    Enter(u64, u128),
    Exit(u64, u128),
}

lazy_static! {
    static ref LOGS: Mutex<LinkedList<Arc<Storage<RawEvent>>>> = Mutex::new(LinkedList::new());
}

thread_local! {
    static THREAD_LOGS: Arc<Storage<RawEvent>> = {
        let storage = Arc::new(Storage::new());
        LOGS.lock().unwrap().push_back(storage.clone());
        storage
            };
}

pub(super) fn reset_events() {
    for log in LOGS.lock().unwrap().iter() {
        log.reset()
    }
}

pub(super) fn log_event(event: RawEvent) {
    THREAD_LOGS.with(|logs| logs.push(event))
}

pub(super) fn extract_spans() -> HashMap<u64, Span> {
    let mut spans: HashMap<u64, Span> = HashMap::new();
    let mut min_time = std::u128::MAX;
    for (thread, log) in LOGS.lock().unwrap().iter().enumerate() {
        let mut active_spans = Vec::new();
        for event in log.iter() {
            match event {
                RawEvent::NewSpan(id, name, parent) => {
                    let span = spans.entry(*id).or_insert_with(|| Span::new(*id));
                    span.name = name;
                    span.parent = if *parent == 0 {
                        active_spans.last().cloned()
                    } else {
                        Some(*parent)
                    };
                    span.creation_thread = thread;
                }
                RawEvent::Enter(id, time) => {
                    let span = spans.entry(*id).or_insert_with(|| Span::new(*id));
                    span.start = *time;
                    if span.end == 0 {
                        span.end = span.start
                    }
                    assert!(span.end >= span.start);
                    span.execution_thread = thread;
                    active_spans.push(*id);
                    min_time = min_time.min(*time);
                }
                RawEvent::Exit(id, time) => {
                    let span = spans.entry(*id).or_insert_with(|| Span::new(*id));
                    span.end = *time;
                    assert_eq!(span.execution_thread, thread);
                    assert_eq!(active_spans.pop(), Some(*id));
                    min_time = min_time.min(*time);
                }
                _ => unimplemented!(),
            }
        }
        log.reset();
    }
    // now translate times
    spans.values_mut().for_each(|s| {
        s.start -= min_time;
        s.end -= min_time;
    });

    spans
}
