use crate::error::Result;
use crate::platform;
use tracing::warn;

/// Calculates the distance moved in inches, considering potential monitor changes and using cached PPI.
pub fn calculate_distance_inches(x1: i32, y1: i32, x2: i32, y2: i32) -> Result<f64> {
    if x1 == x2 && y1 == y2 {
        return Ok(0.0);
    }

    let monitor1 = platform::get_monitor_for_point(x1, y1)?;
    let monitor2 = platform::get_monitor_for_point(x2, y2)?;

    let ppi1 = if monitor1.ppi > 0.0 {
        monitor1.ppi
    } else {
        96.0
    };

    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    let pixel_distance = (dx * dx + dy * dy).sqrt();

    if monitor1.id_hash != monitor2.id_hash {
        warn!(
            "Cross-monitor movement detected ({} -> {}). Using simplified distance calculation based on starting monitor PPI.",
            monitor1.name, monitor2.name
        );
    }

    Ok(pixel_distance / ppi1)
}
