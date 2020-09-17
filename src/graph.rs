use super::Span;
use either::Either;
use itertools::Itertools;
use std::collections::HashMap;

struct Task {
    start: u128,
    end: u128,
    thread: usize,
}

struct Node {
    children: Either<Vec<Node>, Task>,
    is_parallel: bool,
    size: [u128; 2],
    position: [f64; 2],
}

impl Node {
    fn new_from_children<I: Iterator<Item = Node>>(children: I, is_parallel: bool) -> Self {
        Node {
            children: Either::Left(children.collect()),
            is_parallel,
            size: [0; 2],
            position: [0.0; 2],
        }
    }
    fn new_from_task(task: Task) -> Self {
        Node {
            children: Either::Right(task),
            is_parallel: false,
            size: [0; 2],
            position: [0.0; 2],
        }
    }
}

pub(super) struct Graph {
    root: Node,
}

impl Graph {
    pub(super) fn new(spans: &HashMap<u64, Span>) -> Graph {
        let mut roots = Vec::new();
        let mut children = HashMap::new();
        for (span_id, span) in spans {
            if let Some(parent) = span.parent {
                children
                    .entry(parent)
                    .or_insert_with(Vec::new)
                    .push(*span_id)
            } else {
                roots.push(*span_id)
            }
        }
        assert_eq!(roots.len(), 1); // TODO: for now
        let root_id = roots.first().unwrap();
        assert_eq!(spans[root_id].name, "main_task");
        Graph {
            root: build_graph(root_id, &children, spans),
        }
    }
}

fn build_graph(
    root_id: &u64,
    children: &HashMap<u64, Vec<u64>>,
    spans: &HashMap<u64, Span>,
) -> Node {
    let subgraphs = children.get(root_id).into_iter().flat_map(|my_children| {
        my_children
            .iter()
            .map(|child_id| build_graph(child_id, children, spans))
    });
    if spans[root_id].name == "parallel" {
        // parallel display
        Node::new_from_children(subgraphs, true)
    } else {
        // sequential display
        // we interleave "fake" tasks between the real children
        let times = children.get(root_id).into_iter().flat_map(|my_children| {
            my_children
                .iter()
                .map(|child_id| (spans[child_id].start, spans[child_id].end))
        });
        let root_span = &spans[root_id];
        let all_times = std::iter::once((0, root_span.start))
            .chain(times)
            .chain(std::iter::once((root_span.end, 0)));
        let intervals = all_times.tuple_windows().map(|(a, b)| (a.1, b.0));
        let tasks = intervals.map(|(start, end)| {
            Node::new_from_task(Task {
                start,
                end,
                thread: root_span.execution_thread,
            })
        });
        Node::new_from_children(tasks.interleave(subgraphs), false)
    }
}
