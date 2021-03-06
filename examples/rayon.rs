use fast_tracer::svg;
use rayon::prelude::*;

fn main() {
    svg("filter_collect.svg", || {
        let v = (0..10_000_000)
            .into_par_iter()
            .filter(|&e| e % 2 == 0)
            .fold(Vec::new, |mut v, e| {
                v.push(e);
                v
            })
            .reduce(Vec::new, |mut v1, mut v2| {
                v1.append(&mut v2);
                v1
            });
        assert!(v.len() > 0)
    })
    .expect("failed saving svg file")
}
