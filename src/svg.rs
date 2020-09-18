use super::Node;
use super::{extract_spans, Graph};
use either::Either;
use itertools::Itertools;
use std::io::Write;
use tracing::{span, Level};

pub(super) const SVG_WIDTH: u128 = 1920;
pub(super) const SVG_HEIGHT: u128 = 1080;

const COLORS: [&str; 7] = [
    "red", "blue", "green", "yellow", "purple", "brown", "orange",
];

pub fn svg<P: AsRef<std::path::Path>, R, F: FnOnce() -> R>(path: P, op: F) -> std::io::Result<R> {
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
        self.root.write_tasks_svg(&mut svg_file, time_dilation)?;
        self.root
            .write_edges_svg(&mut svg_file, &Vec::new(), &Vec::new())?;
        writeln!(&mut svg_file, "</svg>")?;
        Ok(())
    }
}

impl Node {
    fn write_tasks_svg<W: Write>(&self, writer: &mut W, time_dilation: f64) -> std::io::Result<()> {
        match &self.children {
            Either::Left(children) => children
                .iter()
                .try_for_each(|child| child.write_tasks_svg(writer, time_dilation)),
            Either::Right(task) => writeln!(
                writer,
                "<rect width='{}' height='{}' x='{}' y='{}' fill='black'/>
<rect width='0' height='{}' x='{}' y='{}' fill='{}'>
<animate attributeType=\"XML\" attributeName=\"width\" from=\"0\" to=\"{}\" begin=\"{}ms\" dur=\"{}ms\" fill=\"freeze\"/>
</rect>",
                self.scaled_size[0],
                self.scaled_size[1] * 0.5,
                self.position[0],
                self.position[1] + self.height() * 0.25,
                self.scaled_size[1] * 0.5,
                self.position[0],
                self.position[1] + self.height() * 0.25,
                COLORS[task.thread % COLORS.len()],
                self.scaled_size[0],
                task.start as f64 * time_dilation,
                (task.end-task.start) as f64 * time_dilation,
            ),
        }?;
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
