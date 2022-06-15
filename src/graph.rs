use super::{Span, SVG_HEIGHT, SVG_WIDTH};
use either::Either;
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Debug)]
pub(super) struct Task {
    pub(super) start: u128,
    pub(super) end: u128,
    pub(super) thread: usize,
    pub(super) label: &'static str,
}

pub(super) struct Node {
    pub(super) children: Either<Vec<Node>, Task>,
    pub(super) is_parallel: bool,
    size: [u128; 2],
    pub(super) scaled_size: [f64; 2],
    pub(super) position: [f64; 2],
}

impl Node {
    fn new_from_children<I: Iterator<Item = Node>>(children: I, is_parallel: bool) -> Self {
        let mut size = [0, 0];
        // let's compute dimensions and collect children in one pass
        let children_vec = if is_parallel {
            children
                .scan(&mut size, |size, child| {
                    size[0] += child.size[0];
                    size[1] = size[1].max(child.size[1]);
                    Some(child)
                })
                .collect()
        } else {
            children
                .scan(&mut size, |size, child| {
                    size[0] = size[0].max(child.size[0]);
                    size[1] += child.size[1];
                    Some(child)
                })
                .collect()
        };
        Node {
            children: Either::Left(children_vec),
            is_parallel,
            size,
            scaled_size: [0.0; 2],
            position: [0.0; 2],
        }
    }
    fn new_from_task(task: Task) -> Self {
        let width = task.end - task.start;
        Node {
            children: Either::Right(task),
            is_parallel: false,
            size: [width, 1],
            scaled_size: [0.0; 2],
            position: [0.0; 2],
        }
    }
    pub(super) fn width(&self) -> f64 {
        self.scaled_size[0]
    }
    pub(super) fn height(&self) -> f64 {
        self.scaled_size[1]
    }
    fn scale_size(&mut self, x_scale: f64, y_scale: f64) {
        self.scaled_size = [self.size[0] as f64 / x_scale, self.size[1] as f64 / y_scale];
    }
    fn compute_positions(&mut self, x_scale: f64, y_scale: f64) {
        let width = self.width();
        let height = self.height();
        match &mut self.children {
            Either::Left(children) => {
                let mut position = self.position.clone();
                for child in children {
                    child.scale_size(x_scale, y_scale);
                    if self.is_parallel {
                        // center on height
                        position[1] = self.position[1] + (height - child.height()) / 2.0
                    } else {
                        // center on width
                        position[0] = self.position[0] + (width - child.width()) / 2.0
                    }
                    child.position = position;
                    child.compute_positions(x_scale, y_scale);
                    if self.is_parallel {
                        position[0] += child.width()
                    } else {
                        position[1] += child.height()
                    }
                }
            }
            _ => (),
        }
    }
}

pub(super) struct Graph {
    pub(super) root: Node,
    pub(super) start: u128,
    pub(super) end: u128,
    pub(super) threads_number: usize,
    pub(super) x_scale: f64,
    pub(super) y_scale: f64,
}

impl Graph {
    pub(super) fn new(spans: &HashMap<u64, Span>) -> Graph {
        let mut roots = Vec::new();
        let mut children = HashMap::new();
        let mut start = u128::MAX;
        let mut end = 0;
        let mut max_thread = 0;
        for (span_id, span) in spans {
            max_thread = max_thread.max(span.execution_thread);
            start = start.min(span.start);
            end = end.max(span.end);
            if let Some(parent) = span.parent {
                children
                    .entry(parent)
                    .or_insert_with(Vec::new)
                    .push(*span_id)
            } else {
                roots.push(*span_id)
            }
        }
        // sort children by starting order
        children.values_mut().for_each(|children| {
            children.sort_by_key(|child_id| (spans[child_id].name, spans[child_id].start));
        });

        assert_eq!(roots.len(), 1); // TODO: for now
        let root_id = roots.first().unwrap();
        assert_eq!(spans[root_id].name, "main_task");
        let root = build_graph(root_id, &children, spans);
        let x_scale = root.size[0] as f64 / SVG_WIDTH as f64;
        // add some extra space at bottom to display threads idling
        let y_scale = (root.size[1] + max_thread as u128) as f64 / SVG_HEIGHT as f64;
        let mut graph = Graph {
            root,
            start,
            end,
            threads_number: max_thread + 1,
            x_scale,
            y_scale,
        };
        // re-scale sizes of root node
        graph.root.scale_size(x_scale, y_scale);
        // now, re-scale all node sizes and compute their positions
        graph.root.compute_positions(x_scale, y_scale);
        graph
    }
}

fn build_graph(
    root_id: &u64,
    children: &HashMap<u64, Vec<u64>>,
    spans: &HashMap<u64, Span>,
) -> Node {
    let subgraphs = children
        .get(root_id)
        .into_iter()
        .flatten()
        .map(|child_id| build_graph(child_id, children, spans));
    if spans[root_id].name == "parallel" {
        // parallel display
        Node::new_from_children(subgraphs, true)
    } else {
        // sequential display
        // we interleave "fake" tasks between the real children
        let times = children
            .get(root_id)
            .into_iter()
            .flatten()
            .map(|child_id| (spans[child_id].start, spans[child_id].end));
        let root_span = &spans[root_id];
        let all_times = std::iter::once((0, root_span.start))
            .chain(times)
            .chain(std::iter::once((root_span.end, 0)));
        let intervals = all_times.tuple_windows().map(|(a, b)| (a.1, b.0));
        let tasks = intervals.map(|(start, end)| {
            assert!(end >= start);
            Node::new_from_task(Task {
                start,
                end,
                thread: root_span.execution_thread,
                label: spans[root_id].name,
            })
        });
        Node::new_from_children(tasks.interleave(subgraphs), false)
    }
}
