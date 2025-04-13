use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct IntervalMetrics {
    pub keypresses: AtomicUsize,
    pub mouse_clicks: AtomicUsize,
    pub scroll_steps: AtomicUsize,
    pub mouse_distance_in: Mutex<f64>,
}

impl IntervalMetrics {
    /// Resets the interval metrics to zero, returning the values captured during the interval.
    pub async fn reset(&self) -> (usize, usize, usize, f64) {
        let keys = self.keypresses.swap(0, Ordering::Relaxed);
        let clicks = self.mouse_clicks.swap(0, Ordering::Relaxed);
        let scrolls = self.scroll_steps.swap(0, Ordering::Relaxed);

        let distance = {
            let mut dist_lock = self.mouse_distance_in.lock().await;
            let current_dist = *dist_lock;
            *dist_lock = 0.0;
            current_dist
        };
        (keys, clicks, scrolls, distance)
    }
}

#[derive(Debug, Default)]
pub struct TotalMetrics {
    pub keypresses: AtomicUsize,
    pub mouse_clicks: AtomicUsize,
    pub scroll_steps: AtomicUsize,
    pub mouse_distance_in: Mutex<f64>,
}

impl TotalMetrics {
    /// Adds the values from a completed interval to the running totals.
    pub async fn add_interval(&self, keys: usize, clicks: usize, scrolls: usize, distance: f64) {
        self.keypresses.fetch_add(keys, Ordering::Relaxed);
        self.mouse_clicks.fetch_add(clicks, Ordering::Relaxed);
        self.scroll_steps.fetch_add(scrolls, Ordering::Relaxed);
        if distance > 0.0 {
            let mut total_dist_lock = self.mouse_distance_in.lock().await;
            *total_dist_lock += distance;
        }
    }
}

#[derive(Debug, Default)]
pub struct MetricsState {
    pub interval: IntervalMetrics,
    pub total: TotalMetrics,
    pub latest_mouse_x: AtomicI32,
    pub latest_mouse_y: AtomicI32,
    pub last_calc_mouse_x: AtomicI32,
    pub last_calc_mouse_y: AtomicI32,
}
