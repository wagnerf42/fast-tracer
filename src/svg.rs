use super::{extract_spans, Graph};
use tracing::{span, Level};

pub fn svg<P: AsRef<std::path::Path>, R, F: FnOnce() -> R>(path: P, op: F) -> std::io::Result<R> {
    let span = span!(Level::TRACE, "main_task");
    let r = {
        let _enter = span.enter();
        op()
    };
    let spans = extract_spans();
    let graph = Graph::new(&spans);
    Ok((r))
}
