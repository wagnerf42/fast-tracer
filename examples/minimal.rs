use fast_tracer::FastSubscriber;
use tracing::{span, Level};
fn main() {
    let span = span!(Level::TRACE, "start_task");
    let my_subscriber = FastSubscriber::new();
    tracing::subscriber::set_global_default(my_subscriber).expect("setting tracing default failed");
    let _enter = span.enter();
    std::thread::sleep(std::time::Duration::from_secs(2));
}
