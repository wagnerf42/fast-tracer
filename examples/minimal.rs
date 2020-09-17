use fast_tracer::{svg, FastSubscriber};
use tracing::{span, Level};

fn main() {
    let my_subscriber = FastSubscriber::new();
    tracing::subscriber::set_global_default(my_subscriber).expect("setting tracing default failed");

    svg("minimal.svg", || {
        let father = tracing::dispatcher::get_default(|dispatcher| dispatcher.current_span());
        let span2 = span!(parent: father.id().cloned(), Level::TRACE, "child_task");
        let _enter = span2.enter();
        std::thread::sleep(std::time::Duration::from_secs(1));
    })
    .expect("failed saving svg file")
}
