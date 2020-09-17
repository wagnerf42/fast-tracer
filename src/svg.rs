use super::Node;
use super::{extract_spans, Graph};
use either::Either;
use std::io::Write;
use tracing::{span, Level};

pub(super) const SVG_WIDTH: u128 = 1920;
pub(super) const SVG_HEIGHT: u128 = 1080;
const COLORS: [&'static str; 3] = ["red", "green", "blue"];

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
        self.root.write_tasks_svg(&mut svg_file)?;
        self.root.write_edges_svg(&mut svg_file)?;
        writeln!(&mut svg_file, "</svg>")?;
        Ok(())
    }
}

impl Node {
    fn write_tasks_svg<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match &self.children {
            Either::Left(children) => children
                .iter()
                .try_for_each(|child| child.write_tasks_svg(writer)),
            Either::Right(task) => writeln!(
                writer,
                "<rect width='{}' height='{}' x='{}' y='{}' style='fill:{}'/>",
                self.scaled_size[0],
                self.scaled_size[1],
                self.position[0],
                self.position[1],
                COLORS[task.thread % COLORS.len()]
            ),
        }?;
        Ok(())
    }
    fn write_edges_svg<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
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
            Either::Right(_) => vec![(self.position[0] + self.width() / 2.0, self.position[1])],
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
                self.position[1] + self.height(),
            )],
        }
    }
}
