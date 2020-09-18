use fast_tracer::{svg, FastSubscriber};
use rayon::prelude::*;

fn main() {
    let my_subscriber = FastSubscriber::new();
    tracing::subscriber::set_global_default(my_subscriber).expect("setting tracing default failed");

    svg("filter_collect.svg", || {
        let v = (0..10_000_000)
            .into_par_iter()
            .filter(|&e| e % 2 == 0)
            .collect::<Vec<_>>();
        assert!(v.len() > 0)
    })
    .expect("failed saving svg file")
}
