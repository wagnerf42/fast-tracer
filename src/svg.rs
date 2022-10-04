use crate::spans::Span;

use super::FastSubscriber;
use super::{extract_spans, reset_events, Graph};
use super::{Node, Task};
use either::Either;
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;
use tracing::{span, Level};

pub(super) const SVG_WIDTH: u128 = 1920;
pub(super) const SVG_HEIGHT: u128 = 1080;

const COLORS: [&str; 7] = [
    "red", "blue", "green", "yellow", "purple", "brown", "orange",
];

pub fn display_svg<R, F: FnOnce() -> R>(op: F) -> std::io::Result<R> {
    let mut tmp = std::env::temp_dir();
    let id = rand::random::<u64>();
    tmp.push(id.to_string());
    tmp.set_extension("svg");
    let r = svg(&tmp, op)?;
    std::process::Command::new("firefox").arg(tmp).status()?;
    Ok(r)
}

pub fn svg<P: AsRef<std::path::Path>, R, F: FnOnce() -> R>(path: P, op: F) -> std::io::Result<R> {
    let subscriber: FastSubscriber = FastSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).err();
    reset_events();
    let span = span!(Level::TRACE, "main_task");
    let r = {
        let _enter = span.enter();
        op()
    };
    let spans = extract_spans();
    let graph = Graph::new(&spans);
    graph.save_svg(path)?;
    Ok(r)
}

/// Saves an svg displaying the gantt diagram
/// of the recorded execution of `op`.
pub fn gantt_svg<P: AsRef<std::path::Path>, R, F: FnOnce() -> R>(
    path: P,
    op: F,
) -> std::io::Result<R> {
    let subscriber: FastSubscriber = FastSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).err();
    reset_events();
    let span = span!(Level::TRACE, "main_task");
    let r = {
        let _enter = span.enter();
        op()
    };
    let spans = extract_spans();
    let gantt = Gantt::new(&spans);
    gantt.save_svg(&path)?;
    Ok(r)
}

#[derive(Debug)]
pub(super) struct Gantt<'a> {
    pub(super) start: u128,
    pub(super) end: u128,
    pub(super) min_exec_time: u128,
    pub(super) spans: &'a HashMap<u64, Span>,
    pub(super) span_colors: HashMap<&'static str, usize>,
    pub(super) nb_threads: u32,
}

impl<'a> Gantt<'a> {
    fn new(spans: &'a HashMap<u64, Span>) -> Self {
        let mut nb_threads = 0;
        let mut start = u128::MAX;
        let mut end: u128 = 0;
        let mut min_exec_time = u128::MAX;
        let mut span_colors: HashMap<&'static str, usize> = HashMap::new();
        let mut colors = (0..7).cycle();
        for (_, span) in spans {
            nb_threads = nb_threads.max(1 + span.execution_thread as u32);
            start = start.min(span.start);
            end = end.max(span.end);
            min_exec_time = min_exec_time.min(span.end - span.start);
            span_colors
                .entry(span.name)
                .or_insert_with(|| colors.next().unwrap());
        }
        Gantt {
            start,
            end,
            min_exec_time,
            spans: &spans,
            span_colors,
            nb_threads,
        }
    }

    fn save_svg<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        let mut svg_file = std::fs::File::create(path)?;
        let random_id = rand::random::<u64>();
        writeln!(
            &mut svg_file,
            "<svg version='1.1' viewBox='0 0 {} {}' xmlns='http://www.w3.org/2000/svg'>",
            SVG_WIDTH, SVG_HEIGHT
        )?;
        self.write_tasks(&mut svg_file, random_id)?;
        writeln!(&mut svg_file, "</svg>")?;
        Ok(())
    }

    fn write_tasks<W: Write>(&self, writer: &mut W, random_id: u64) -> std::io::Result<()> {
        let mut seen: HashSet<u64> = HashSet::new();
        for (_, span) in self.spans {
            self.write_task(writer, span, &mut seen, random_id)?;
        }
        for (span_id, span) in self.spans {
            self.write_task_hover(writer, random_id, &span_id, span)?;
        }
        write_javascript_code(writer, random_id)?;
        Ok(())
    }

    fn write_task<W: Write>(
        &self,
        writer: &mut W,
        span: &Span,
        seen: &mut HashSet<u64>,
        random_id: u64,
    ) -> std::io::Result<()> {
        if !seen.contains(&span.id) {
            if let Some(father) = span.parent {
                self.write_task(writer, self.spans.get(&father).unwrap(), seen, random_id)?;
            }
            writeln!(
                writer,
                "<rect class='{}' id='{}' width='{}' height='{}' x='{}' y='{}' fill='{}'/>",
                format!("task{}", random_id),
                span.id,
                ((span.end - span.start) * SVG_WIDTH) as f32 / (self.end - self.start) as f32,
                SVG_HEIGHT as u32 / self.nb_threads,
                ((span.start - self.start) * SVG_WIDTH) as f32 / (self.end - self.start) as f32,
                SVG_HEIGHT as f32 / (self.nb_threads as f32) * span.execution_thread as f32,
                COLORS[*self.span_colors.get(span.name).unwrap()],
            )?;
            seen.insert(span.id);
        }
        Ok(())
    }

    fn write_task_hover<W: Write>(
        &self,
        writer: &mut W,
        random_id: u64,
        span_id: &u64,
        span: &Span,
    ) -> std::io::Result<()> {
        let label = format!(
            "start {} end {}\nduration {}\nlabel {}",
            span.start,
            span.end,
            time_string(span.end - span.start),
            span.name
        );
        writeln!(writer, "<g id=\"tip_{}_{}\">", random_id, span_id)?;
        let x = SVG_WIDTH - 400;
        let height = label.lines().count() as u32 * 20;
        let mut y = SVG_HEIGHT as u32 - height - 40;
        writeln!(
            writer,
            "<rect x=\"{}\" y=\"{}\" width=\"300\" height=\"{}\" fill=\"white\" stroke=\"black\"/>",
            x,
            y,
            height + 10
        )?;
        for line in label.lines() {
            y += 20;
            writeln!(writer, "<text x=\"{}\" y=\"{}\">{}</text>", x + 5, y, line)?;
        }
        writeln!(writer, "</g>")
    }
}

impl Graph {
    fn save_svg<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        let mut svg_file = std::fs::File::create(path)?;
        writeln!(
            &mut svg_file,
            "<svg version='1.1' viewBox='0 0 {} {}' xmlns='http://www.w3.org/2000/svg'>",
            SVG_WIDTH, SVG_HEIGHT
        )?;
        // let's animate for 30 seconds
        let time_dilation = 30_000.0 / (self.end - self.start) as f64;
        let random_id = rand::random(); // so we can include several logs in the same webpage
        self.root
            .write_edges_svg(&mut svg_file, &Vec::new(), &Vec::new())?;
        let mut tasks_number: usize = 0;
        self.root
            .write_tasks_svg(&mut svg_file, time_dilation, random_id, &mut tasks_number)?;
        self.write_idle_gantt_diagram(&mut svg_file, time_dilation, random_id, &mut tasks_number)?;
        write_javascript_code(&mut svg_file, random_id)?;
        writeln!(&mut svg_file, "</svg>")?;
        Ok(())
    }
}

fn write_javascript_code<W: Write>(writer: &mut W, random_id: u64) -> std::io::Result<()> {
    // this part will allow to get more info on tasks by hovering over them
    writeln!(
        writer,
        "
   <style>
      .task-highlight {{
        fill: #ec008c;
        opacity: 1;
      }}
    </style>
  <script><![CDATA[

    displayTips();

    function displayTips() {{
        let tasks = document.getElementsByClassName('task{id}');
        for (let i = 0; i < tasks.length ; i++) {{
          let task = tasks[i];
          let tip_id = task.id;
          let tip = document.getElementById('tip_{id}_'+tip_id);
          tip.style.display='none';
          tasks[i].tip = tip;
          tasks[i].addEventListener('mouseover', mouseOverEffect);
          tasks[i].addEventListener('mouseout', mouseOutEffect);
        }}
    }}

    function mouseOverEffect() {{
      this.classList.add(\"task-highlight\");
      this.tip.style.display='block';
    }}

    function mouseOutEffect() {{
      this.classList.remove(\"task-highlight\");
      this.tip.style.display='none';
    }}
  ]]></script>",
        id = random_id
    )
}

fn write_task_hover<W: Write>(
    writer: &mut W,
    random_id: u64,
    task_id: usize,
    task: &super::graph::Task,
) -> std::io::Result<()> {
    let label = format!(
        "start {} end {}\nduration {}\nlabel {}",
        task.start,
        task.end,
        time_string(task.end - task.start),
        task.label
    );
    writeln!(writer, "<g id=\"tip_{}_{}\">", random_id, task_id)?;
    let x = SVG_WIDTH - 400;
    let height = label.lines().count() as u32 * 20;
    let mut y = SVG_HEIGHT as u32 - height - 40;
    writeln!(
        writer,
        "<rect x=\"{}\" y=\"{}\" width=\"300\" height=\"{}\" fill=\"white\" stroke=\"black\"/>",
        x,
        y,
        height + 10
    )?;
    for line in label.lines() {
        y += 20;
        writeln!(writer, "<text x=\"{}\" y=\"{}\">{}</text>", x + 5, y, line)?;
    }
    writeln!(writer, "</g>")
}

impl Graph {
    fn write_idle_gantt_diagram<W: Write>(
        &self,
        writer: &mut W,
        time_dilation: f64,
        random_id: u64,
        tasks_number: &mut usize,
    ) -> std::io::Result<()> {
        let mut tasks_per_threads: Vec<_> = std::iter::repeat_with(Vec::new)
            .take(self.threads_number)
            .collect();
        for task in self.root.tasks() {
            tasks_per_threads[task.thread].push(task);
        }
        for (thread, thread_tasks) in tasks_per_threads.iter_mut().enumerate() {
            thread_tasks.sort_unstable_by_key(|t| t.start);
            std::iter::once((0, 0))
                .chain(thread_tasks.iter().map(|t| (t.start, t.end)))
                .chain(std::iter::once((self.end, self.end)))
                .tuple_windows()
                .filter_map(|((_, end), (start, _))| {
                    if end == start {
                        None
                    } else {
                        Some((end, start))
                    }
                })
                .try_fold(0, |x, (idle_start, idle_end)| -> std::io::Result<u128> {
                    let width = (idle_end - idle_start) as f64 / self.x_scale;
                    let height = 1.0 / self.y_scale;
                    let y = SVG_HEIGHT as f64 - thread as f64 / self.y_scale;
                    write_task_svg(
                        writer,
                        time_dilation,
                        random_id,
                        thread,
                        x as f64 / self.x_scale,
                        y,
                        width,
                        height,
                        idle_start,
                        idle_end,
                        *tasks_number,
                    )?;

                    let task = Task {
                        start: idle_start,
                        end: idle_end,
                        thread,
                        label: "idle",
                    };
                    write_task_hover(writer, random_id, *tasks_number, &task)?;
                    *tasks_number += 1;
                    Ok(x + (idle_end - idle_start))
                })?;
        }
        Ok(())
    }
}

fn write_task_svg<W: Write>(
    writer: &mut W,
    time_dilation: f64,
    random_id: u64,
    thread_id: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start: u128,
    end: u128,
    tasks_number: usize,
) -> std::io::Result<()> {
    writeln!(
                writer,
                "<rect width='{}' height='{}' x='{}' y='{}' fill='black'/>
<rect class=\"task{}\" id=\"{}\" width='0' height='{}' x='{}' y='{}' fill='{}'>
<animate attributeType=\"XML\" attributeName=\"width\" from=\"0\" to=\"{}\" begin=\"{}ms\" dur=\"{}ms\" fill=\"freeze\"/>
</rect>",
                width,
                height * 0.5,
                x,
                y + height * 0.25,
                random_id,
                tasks_number,
                height * 0.5,
                x,
                y + height * 0.25,
                COLORS[thread_id % COLORS.len()],
                width,
                start as f64 * time_dilation,
                (end-start) as f64 * time_dilation,
            )
}

impl Node {
    fn tasks(&self) -> impl Iterator<Item = &Task> {
        let mut stack = Vec::new();
        stack.push(self);
        std::iter::from_fn(move || {
            while let Some(next_node) = stack.pop() {
                match &next_node.children {
                    Either::Left(children) => stack.extend(children),
                    Either::Right(task) => return Some(task),
                }
            }
            None
        })
    }
    fn write_tasks_svg<W: Write>(
        &self,
        writer: &mut W,
        time_dilation: f64,
        random_id: u64,
        tasks_number: &mut usize,
    ) -> std::io::Result<()> {
        match &self.children {
            Either::Left(children) => children.iter().try_for_each(|child| {
                child.write_tasks_svg(writer, time_dilation, random_id, tasks_number)
            })?,
            //TODO: call write_task_svg
            Either::Right(task) => {
                writeln!(
                writer,
                "<rect width='{}' height='{}' x='{}' y='{}' fill='black'/>
<rect class=\"task{}\" id=\"{}\" width='0' height='{}' x='{}' y='{}' fill='{}'>
<animate attributeType=\"XML\" attributeName=\"width\" from=\"0\" to=\"{}\" begin=\"{}ms\" dur=\"{}ms\" fill=\"freeze\"/>
</rect>",
                self.scaled_size[0],
                self.scaled_size[1] * 0.5,
                self.position[0],
                self.position[1] + self.height() * 0.25,
                random_id,
                *tasks_number,
                self.scaled_size[1] * 0.5,
                self.position[0],
                self.position[1] + self.height() * 0.25,
                COLORS[task.thread % COLORS.len()],
                self.scaled_size[0],
                task.start as f64 * time_dilation,
                (task.end-task.start) as f64 * time_dilation,
            )?;
                write_task_hover(writer, random_id, *tasks_number, &task)?;
                *tasks_number += 1;
            }
        };
        Ok(())
    }
    fn write_edges_svg<W: Write>(
        &self,
        writer: &mut W,
        entry_points: &[(f64, f64)],
        exit_points: &[(f64, f64)],
    ) -> std::io::Result<()> {
        match &self.children {
            Either::Left(children) => {
                if self.is_parallel {
                    children.iter().try_for_each(|child| {
                        child.write_edges_svg(writer, entry_points, exit_points)
                    })?
                } else {
                    entry_points
                        .iter()
                        .cartesian_product(&children.first().unwrap().entry_points())
                        .chain(
                            children
                                .last()
                                .unwrap()
                                .exit_points()
                                .iter()
                                .cartesian_product(exit_points),
                        )
                        .try_for_each(|(start, end)| write_edge_svg(writer, start, end))?;
                    children.iter().tuple_windows().try_for_each(|(a, b, c)| {
                        b.write_edges_svg(writer, &a.exit_points(), &c.entry_points())
                    })?
                }
            }
            _ => (),
        }
        Ok(())
    }
    // TODO: good exercise to write an iterator instead
    fn entry_points(&self) -> Vec<(f64, f64)> {
        match &self.children {
            Either::Left(children) => {
                if self.is_parallel {
                    children
                        .iter()
                        .flat_map(|child| child.entry_points())
                        .collect()
                } else {
                    children.first().unwrap().entry_points()
                }
            }
            Either::Right(_) => vec![(
                self.position[0] + self.width() / 2.0,
                self.position[1] + self.height() * 0.25,
            )],
        }
    }
    fn exit_points(&self) -> Vec<(f64, f64)> {
        match &self.children {
            Either::Left(children) => {
                if self.is_parallel {
                    children
                        .iter()
                        .flat_map(|child| child.exit_points())
                        .collect()
                } else {
                    children.last().unwrap().exit_points()
                }
            }
            Either::Right(_) => vec![(
                self.position[0] + self.width() / 2.0,
                self.position[1] + self.height() * 0.75,
            )],
        }
    }
}

fn write_edge_svg<W: Write>(
    writer: &mut W,
    entry_point: &(f64, f64),
    exit_point: &(f64, f64),
) -> std::io::Result<()> {
    writeln!(
        writer,
        "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='black' stroke-width='3'/>",
        entry_point.0, entry_point.1, exit_point.0, exit_point.1
    )
}

/// Convert nano seconds to human readable string.
fn time_string(nano: u128) -> String {
    match nano {
        n if n < 1_000 => format!("{}ns", n),
        n if n < 1_000_000 => format!("{:.2}us", time_float(n, 1_000)),
        n if n < 1_000_000_000 => format!("{:.2}ms", time_float(n, 1_000_000)),
        n if n < 60_000_000_000 => format!("{:.2}s", time_float(n, 1_000_000_000)),
        n => format!("{}m{}s", n / 60_000_000_000, n % 60_000_000_000),
    }
}

fn time_float(time: u128, limit: u128) -> f64 {
    (time / limit) as f64 + ((time % limit) * 100 / limit) as f64 / 100.0
}
