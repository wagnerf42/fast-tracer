//! Events and the places they are stored into.
use super::Storage;
use lazy_static::lazy_static;
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

pub(super) fn log_event(event: RawEvent) {
    THREAD_LOGS.with(|logs| logs.push(event))
}
