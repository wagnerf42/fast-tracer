mod list;
mod storage;
pub(crate) use storage::Storage;
mod subscriber;
pub use subscriber::FastSubscriber;
mod events;
pub(crate) use events::{log_event, RawEvent};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
