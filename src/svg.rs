use super::FastSubscriber;
use super::Node;
use super::{extract_spans, reset_events, Graph};
use either::Either;
use itertools::Itertools;
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

impl Node {
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
