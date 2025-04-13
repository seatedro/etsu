use crate::distance;
use crate::error::Result;
use crate::input::InputEvent;
use crate::state::MetricsState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time;
use tracing::{debug, instrument, warn};

#[instrument(skip(rx, state, processing_interval))]
pub async fn aggregate_metrics(
    mut rx: Receiver<InputEvent>,
    state: Arc<MetricsState>,
    processing_interval: Duration,
) -> Result<()> {
    debug!(
        "Starting metrics processing task with interval: {:?}",
        processing_interval
    );
    let mut interval_timer = time::interval(processing_interval);
    let initial_x = state.latest_mouse_x.load(Ordering::Relaxed);
    let initial_y = state.latest_mouse_y.load(Ordering::Relaxed);
    state.last_calc_mouse_x.store(initial_x, Ordering::Relaxed);
    state.last_calc_mouse_y.store(initial_y, Ordering::Relaxed);

    loop {
        tokio::select! {
            biased;
            Some(event) = rx.recv() => {
                 match event {
                    InputEvent::KeyPress => {
                        state.interval.keypresses.fetch_add(1, Ordering::Relaxed);
                    }
                    InputEvent::MouseClick => {
                        state.interval.mouse_clicks.fetch_add(1, Ordering::Relaxed);
                    }
                    InputEvent::Scroll(delta) => {
                        state.interval.scroll_steps.fetch_add(delta as usize, Ordering::Relaxed);
                    }
                    InputEvent::MouseMove(x, y) => {
                        state.latest_mouse_x.store(x, Ordering::Relaxed);
                        state.latest_mouse_y.store(y, Ordering::Relaxed);
                    }
                }
            }
            _ = interval_timer.tick() => {
                let current_x = state.latest_mouse_x.load(Ordering::Relaxed);
                let current_y = state.latest_mouse_y.load(Ordering::Relaxed);
                let last_x = state.last_calc_mouse_x.load(Ordering::Relaxed);
                let last_y = state.last_calc_mouse_y.load(Ordering::Relaxed);

                if current_x != last_x || current_y != last_y {
                    match distance::calculate_distance_inches(last_x, last_y, current_x, current_y) {
                        Ok(distance_moved) => {
                            if distance_moved > 0.0 {
                                let mut dist_lock = state.interval.mouse_distance_in.lock().await;
                                *dist_lock += distance_moved;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to calculate mouse distance: {}", e);
                        }
                    }
                    state.last_calc_mouse_x.store(current_x, Ordering::Relaxed);
                    state.last_calc_mouse_y.store(current_y, Ordering::Relaxed);
                }
            }
            else => {
                debug!("Input channel closed. Exiting processing task.");
                break;
            }
        }
    }
    Ok(())
}
