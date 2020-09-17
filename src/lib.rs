mod list;
mod storage;
use storage::Storage;
mod subscriber;
pub use subscriber::FastSubscriber;
mod events;
use events::{extract_spans, log_event, RawEvent};
mod spans;
use spans::Span;
mod graph;
use graph::{Graph, Node, Task};
mod svg;
pub use svg::svg;
use svg::{SVG_HEIGHT, SVG_WIDTH};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
