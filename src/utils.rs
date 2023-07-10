use futures::{Stream, StreamExt};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn rate_limit_stream<R, E>(
    str: impl Stream<Item = Result<R, E>>,
    interval: Option<Duration>,
) -> impl Stream<Item = Result<R, E>> {
    // Ref cells are ok here, because we are only ever accessing them from the same thread
    // additionally elements are processed one by one, not in parallel
    let stream_delay = Rc::new(RefCell::new(0.0));
    let current_item = Rc::new(RefCell::new(0));
    let started = std::time::Instant::now();

    str.then(move |res| {
        let stream_delay = stream_delay.clone();
        let current_item = current_item.clone();
        async move {
            if let Some(interval) = interval {
                const MAX_SLIPPAGE_INTERVALS: f64 = 10.0;
                const MAX_SLIPPAGE_CONST: f64 = 0.02;

                let target_time_point =
                    *current_item.borrow() as f64 * interval.as_secs_f64() + *stream_delay.borrow();
                current_item.replace_with(|&mut x| x + 1);

                let elapsed = started.elapsed();
                let delta = target_time_point - elapsed.as_secs_f64();
                let wait_time_seconds = if delta > 0.0 { delta } else { 0.0 };
                if delta < -(MAX_SLIPPAGE_INTERVALS * interval.as_secs_f64() + MAX_SLIPPAGE_CONST) {
                    // stream is falling behind, add the permanent delay
                    stream_delay.replace_with(|&mut val| val + (-delta));
                    log::warn!(
                        "Stream is falling behind, current delay {}s",
                        stream_delay.borrow()
                    );
                }
                if wait_time_seconds > 0.001 {
                    tokio::time::sleep(Duration::from_secs_f64(wait_time_seconds)).await;
                }
            }
            res
        }
    })
}
