use crate::error::{AppError, Result};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tracing::{info, warn};
use twox_hash::XxHash64;

use glfw::Monitor as GlfwMonitor;

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub id_hash: u64,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width_px: u32,
    pub height_px: u32,
    pub width_mm: u32,
    pub height_mm: u32,
    pub ppi: f64,
}

static MONITOR_INFO_CACHE: Lazy<Mutex<Option<Vec<MonitorInfo>>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("GLFW initialization failed: {0:?}")]
    GlfwInit(glfw::InitError),
    #[error("Failed to lock monitor cache")]
    CacheLock,
    #[error("Monitor cache init error")]
    CacheInit,
    #[error("No monitors available")]
    MonitorNotFound,
}

/// Initializes GLFW, fetches monitor information, calculates PPI, caches it, and terminates GLFW.
/// Must be called once at startup from the main thread.
pub fn initialize_monitor_info() -> std::result::Result<(), PlatformError> {
    info!("Initializing GLFW for monitor detection...");

    let mut glfw = glfw::init(glfw::fail_on_errors).map_err(PlatformError::GlfwInit)?;

    let monitors = glfw.with_connected_monitors(|_glfw, monitors| {
        let mut info_list = Vec::new();
        for (index, monitor) in monitors.iter().enumerate() {
            if let Some(info) = get_info_for_monitor(monitor, index) {
                info_list.push(info);
            }
        }
        info_list
    });

    info!("Detected {} monitors.", monitors.len());
    for monitor in &monitors {
        info!(
            "  Monitor '{}' [{}x{} @ ({},{}) - {:.1} PPI]",
            monitor.name, monitor.width_px, monitor.height_px, monitor.x, monitor.y, monitor.ppi
        );
    }

    let mut cache = MONITOR_INFO_CACHE
        .lock()
        .map_err(|_| PlatformError::CacheLock)?;
    *cache = Some(monitors);

    info!("Monitor information cached successfully. GLFW terminated.");
    Ok(())
}

fn get_info_for_monitor(monitor: &GlfwMonitor, index: usize) -> Option<MonitorInfo> {
    monitor.get_video_mode().map(|mode| {
        let (width_mm, height_mm) = monitor.get_physical_size();
        let (x, y) = monitor.get_pos();
        let name = monitor
            .get_name()
            .unwrap_or_else(|| format!("Monitor {}", index));

        let ppi = if width_mm > 0 && height_mm > 0 {
            let width_in = width_mm as f64 / 25.4;
            let height_in = height_mm as f64 / 25.4;
            let ppi_x = mode.width as f64 / width_in;
            let ppi_y = mode.height as f64 / height_in;
            (ppi_x + ppi_y) / 2.0
        } else {
            warn!(
                "Monitor '{}' reported 0 physical size, assuming default PPI.",
                name
            );
            96.0 // default ppi
        };

        MonitorInfo {
            id_hash: hash_name_xxhash64(&name),
            name,
            x,
            y,
            width_px: mode.width,
            height_px: mode.height,
            width_mm: width_mm as u32,
            height_mm: height_mm as u32,
            ppi,
        }
    })
}

pub fn get_cached_monitor_info() -> Result<Vec<MonitorInfo>> {
    let cache = MONITOR_INFO_CACHE
        .lock()
        .map_err(|_| AppError::Platform(PlatformError::CacheLock))?;

    cache
        .as_ref()
        .cloned()
        .ok_or_else(|| AppError::Platform(PlatformError::CacheInit))
}

/// Finds the monitor containing the given screen coordinates using the cached monitor info.
/// Defaults to the primary/first monitor if coordinates are outside known bounds.
pub fn get_monitor_for_point(x: i32, y: i32) -> Result<MonitorInfo> {
    let monitors = get_cached_monitor_info()?;
    if monitors.is_empty() {
        return Err(AppError::Platform(PlatformError::MonitorNotFound));
    }

    let found_monitor = monitors.iter().find(|m| {
        x >= m.x && x < (m.x + m.width_px as i32) && y >= m.y && y < (m.y + m.height_px as i32)
    });

    match found_monitor {
        Some(monitor) => Ok(monitor.clone()),
        None => {
            warn!(
                "Coordinates ({}, {}) outside known monitor bounds, using first monitor as default.",
                x, y
            );
            monitors.first().cloned().ok_or_else(|| AppError::Platform(PlatformError::MonitorNotFound))
        }
    }
}

fn hash_name_xxhash64(name: &str) -> u64 {
    XxHash64::oneshot(42, name.as_bytes())
}
