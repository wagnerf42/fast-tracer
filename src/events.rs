//! Events and the places they are stored into.
use super::{Span, Storage};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::sync::Arc;
use std::sync::Mutex;

pub(super) enum RawEvent {
    NewSpan(u64, &'static str, u64),
    StrField(u64, &'static str, &'static str),
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
    let mut max_time = std::u128::MIN;
    let mut entered_count = 0;
    let mut exited_count = 0;
    let mut all_active_spans = Vec::new();

    for (thread, log) in LOGS.lock().unwrap().iter().enumerate() {
        let mut thread_active_spans = Vec::new();
        for event in log.iter() {
            match event {
                RawEvent::NewSpan(id, name, parent) => {
                    let span = spans.entry(*id).or_insert_with(|| Span::new(*id));
                    span.name = name;
                    span.parent = if *parent == 0 {
                        thread_active_spans.last().cloned()
                    } else {
                        Some(*parent)
                    };
                    span.creation_thread = thread;
                }
                RawEvent::Enter(id, time) => {
                    entered_count += 1;
                    let span = spans.entry(*id).or_insert_with(|| Span::new(*id));
                    span.start = *time;
                    span.execution_thread = thread;
                    thread_active_spans.push(*id);
                    min_time = min_time.min(*time);
                    max_time = max_time.max(*time);
                }
                RawEvent::Exit(id, time) => {
                    exited_count += 1;
                    let span = spans.entry(*id).or_insert_with(|| Span::new(*id));
                    span.end = *time;
                    assert_eq!(span.execution_thread, thread);
                    assert_eq!(thread_active_spans.pop(), Some(*id));
                    min_time = min_time.min(*time);
                    max_time = max_time.max(*time);
                }
                RawEvent::StrField(id, field_name, value) => {
                    if field_name == &"label" {
                        spans.get_mut(id).unwrap().name = value
                    } else {
                        eprintln!("discarded field : {}", field_name)
                    }
                }
            }
        }

        all_active_spans.append(&mut thread_active_spans);

        // log.reset();
    }

    let unfinished_spans = all_active_spans.into_iter().fold(0, |count, id| {
        let span = spans
            .get_mut(&id)
            .expect("Span should be in the hashmap already.");

        if span.end == 0 {
            span.end = max_time;
            count + 1
        } else {
            count
        }
    });

    assert_eq!(entered_count, exited_count + unfinished_spans);
    assert_eq!(entered_count, spans.len());
    // now translate times
    spans.values_mut().for_each(|s| {
        s.start -= min_time;
        s.end -= min_time;
    });

    spans
}
