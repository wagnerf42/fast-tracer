#[derive(Debug)]
pub(super) struct Span {
    pub(super) id: u64,
    pub(super) parent: Option<u64>,
    pub(super) start: u128,
    pub(super) end: u128,
    pub(super) name: &'static str,
    pub(super) execution_thread: usize,
    pub(super) creation_thread: usize,
}

impl Span {
    pub(super) fn new(id: u64) -> Self {
        Span {
            id,
            parent: None,
            start: 0,
            end: 0,
            name: "",
            execution_thread: 0,
            creation_thread: 0,
        }
    }
}
