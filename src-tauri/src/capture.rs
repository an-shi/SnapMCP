use crate::models::{DisplaySpec, MonitorSnapshot, RectI};
use image::RgbaImage;
use xcap::Monitor;

#[derive(Debug, Clone)]
pub struct CapturedImage {
    pub image: RgbaImage,
    /// Absolute virtual-desktop region represented by `image`.
    pub region: RectI,
    pub monitors: Vec<MonitorSnapshot>,
    pub display: Option<MonitorSnapshot>,
}

fn monitor_snapshot(monitor: &Monitor, index: usize) -> MonitorSnapshot {
    MonitorSnapshot {
        index,
        name: monitor
            .friendly_name()
            .unwrap_or_else(|_| format!("Display {}", index + 1)),
        x: monitor.x().unwrap_or(0),
        y: monitor.y().unwrap_or(0),
        width: monitor.width().unwrap_or(0),
        height: monitor.height().unwrap_or(0),
        scale_factor: monitor.scale_factor().map(|v| v as f64).unwrap_or(1.0),
        is_primary: monitor.is_primary().unwrap_or(false),
    }
}

fn normalize(monitor: &Monitor, index: usize) -> MonitorSnapshot {
    let mut snap = monitor_snapshot(monitor, index);
    if snap.width == 0 || snap.height == 0 {
        snap.width = 1;
        snap.height = 1;
    }
    snap
}

pub fn list_monitors() -> Result<Vec<MonitorSnapshot>, String> {
    let monitors = Monitor::all().map_err(|e| format!("Cannot enumerate monitors: {e}"))?;
    if monitors.is_empty() {
        return Err("No displays were found".into());
    }
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(i, m)| normalize(m, i))
        .collect())
}

#[cfg(windows)]
fn cursor_position() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut point = POINT::default();
    unsafe {
        if GetCursorPos(&mut point).is_ok() {
            Some((point.x, point.y))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn cursor_position() -> Option<(i32, i32)> {
    None
}

pub fn cursor_display_index(monitors: &[MonitorSnapshot]) -> Option<usize> {
    let (x, y) = cursor_position()?;
    monitors
        .iter()
        .find(|m| {
            x >= m.x
                && y >= m.y
                && x < m.x.saturating_add(m.width as i32)
                && y < m.y.saturating_add(m.height as i32)
        })
        .map(|m| m.index)
}

fn choose_monitor_index(monitors: &[MonitorSnapshot], spec: DisplaySpec) -> Result<usize, String> {
    match spec {
        DisplaySpec::Primary => monitors
            .iter()
            .find(|m| m.is_primary)
            .map(|m| m.index)
            .or_else(|| monitors.first().map(|m| m.index))
            .ok_or_else(|| "No displays were found".to_string()),
        DisplaySpec::Cursor => cursor_display_index(monitors)
            .or_else(|| monitors.iter().find(|m| m.is_primary).map(|m| m.index))
            .or_else(|| monitors.first().map(|m| m.index))
            .ok_or_else(|| "No displays were found".to_string()),
        DisplaySpec::Index(i) => {
            if i < monitors.len() {
                Ok(i)
            } else {
                Err(format!(
                    "Display index {i} does not exist (0..{})",
                    monitors.len().saturating_sub(1)
                ))
            }
        }
        DisplaySpec::All => Err("'all' is not a single display".into()),
    }
}

fn capture_single_monitor(index: usize) -> Result<(CapturedImage, MonitorSnapshot), String> {
    let monitors = list_monitors()?;
    let snap = monitors
        .get(index)
        .cloned()
        .ok_or_else(|| format!("Display index {index} does not exist"))?;
    let raw = Monitor::all().map_err(|e| format!("Cannot enumerate monitors: {e}"))?;
    let monitor = raw
        .into_iter()
        .nth(index)
        .ok_or_else(|| "Display disappeared while capturing".to_string())?;
    let mut image = monitor
        .capture_image()
        .map_err(|e| format!("Screen capture failed: {e}"))?;
    if image.width() != snap.width || image.height() != snap.height {
        crate::diag::warn(format!(
            "capture size {}x{} differs from display bounds {}x{} (index {})",
            image.width(),
            image.height(),
            snap.width,
            snap.height,
            index
        ));
        image = image::imageops::resize(
            &image,
            snap.width.max(1),
            snap.height.max(1),
            image::imageops::FilterType::Lanczos3,
        );
    }
    let captured = CapturedImage {
        image,
        region: snap.bounds(),
        monitors,
        display: Some(snap.clone()),
    };
    Ok((captured, snap))
}

pub fn capture_display(spec: DisplaySpec) -> Result<CapturedImage, String> {
    match spec {
        DisplaySpec::All => capture_all(),
        other => {
            let monitors = list_monitors()?;
            let index = choose_monitor_index(&monitors, other)?;
            capture_single_monitor(index).map(|(captured, _)| captured)
        }
    }
}

pub fn capture_primary() -> Result<(RgbaImage, MonitorSnapshot), String> {
    let monitors = list_monitors()?;
    let index = choose_monitor_index(&monitors, DisplaySpec::Primary)?;
    let (captured, snap) = capture_single_monitor(index)?;
    Ok((captured.image, snap))
}

pub fn capture_all() -> Result<CapturedImage, String> {
    let monitors = list_monitors()?;
    if monitors.is_empty() {
        return Err("No displays were found".into());
    }
    let raw = Monitor::all().map_err(|e| format!("Cannot enumerate monitors: {e}"))?;
    let min_x = monitors.iter().map(|m| m.x).min().unwrap_or(0);
    let min_y = monitors.iter().map(|m| m.y).min().unwrap_or(0);
    let max_x = monitors
        .iter()
        .map(|m| m.x as i64 + m.width as i64)
        .max()
        .unwrap_or(0);
    let max_y = monitors
        .iter()
        .map(|m| m.y as i64 + m.height as i64)
        .max()
        .unwrap_or(0);
    let width = (max_x - min_x as i64).max(1);
    let height = (max_y - min_y as i64).max(1);
    if width * height > 160_000_000 {
        return Err("Virtual desktop is too large to capture".into());
    }
    let mut canvas = RgbaImage::new(width as u32, height as u32);
    for (index, snap) in monitors.iter().enumerate() {
        let Some(monitor) = raw.get(index) else {
            continue;
        };
        let mut image = match monitor.capture_image() {
            Ok(image) => image,
            Err(err) => {
                crate::diag::warn(format!(
                    "Skipping display {} because capture failed: {err}",
                    snap.index
                ));
                continue;
            }
        };
        if image.width() != snap.width || image.height() != snap.height {
            image = image::imageops::resize(
                &image,
                snap.width.max(1),
                snap.height.max(1),
                image::imageops::FilterType::Lanczos3,
            );
        }
        let ox = (snap.x - min_x) as i64;
        let oy = (snap.y - min_y) as i64;
        image::imageops::overlay(&mut canvas, &image, ox, oy);
    }
    let raw_width = canvas.width();
    let raw_height = canvas.height();
    Ok(CapturedImage {
        image: canvas,
        region: RectI::new(min_x, min_y, raw_width, raw_height),
        monitors,
        display: None,
    })
}

pub fn crop(image: &RgbaImage, rect: RectI) -> Result<RgbaImage, String> {
    if rect.width == 0 || rect.height == 0 {
        return Err("The capture region is empty".into());
    }
    if rect.x < 0
        || rect.y < 0
        || rect.right() > image.width() as i64
        || rect.bottom() > image.height() as i64
    {
        return Err("The capture region is outside the image".into());
    }
    Ok(
        image::imageops::crop_imm(image, rect.x as u32, rect.y as u32, rect.width, rect.height)
            .to_image(),
    )
}

/// Capture the region requested by an MCP client. Coordinates are absolute
/// virtual-desktop pixels, matching `list_displays`.
pub fn capture_for_mcp(
    spec: DisplaySpec,
    requested_region: Option<RectI>,
) -> Result<CapturedImage, String> {
    if let Some(region) = requested_region {
        let all = capture_all()?;
        let wanted = region
            .intersect(all.region)
            .ok_or_else(|| "The requested region is outside the virtual desktop".to_string())?;
        let cropped = crop(
            &all.image,
            RectI::new(
                wanted.x - all.region.x,
                wanted.y - all.region.y,
                wanted.width,
                wanted.height,
            ),
        )?;
        return Ok(CapturedImage {
            image: cropped,
            region: wanted,
            monitors: all.monitors,
            display: all.display,
        });
    }
    capture_display(spec)
}
