use crate::annotate::{self, Annotation};
use crate::capture;
use crate::models::{display_label, DisplaySpec, MonitorSnapshot, RectI};
use crate::state::{AppState, InteractiveOutcome};
use crate::{diag, export, overlay, paths, util};
use base64::Engine;
use image::RgbaImage;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;

pub struct ToolOutput {
    pub content: Vec<Value>,
    pub is_error: bool,
}

impl ToolOutput {
    fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![json!({ "type": "text", "text": text.into() })],
            is_error: false,
        }
    }

    fn with_image(mut self, image: &RgbaImage, include: bool) -> Self {
        if include {
            let mut cursor = std::io::Cursor::new(Vec::new());
            if image.write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
                let data = base64::engine::general_purpose::STANDARD.encode(cursor.into_inner());
                self.content.push(json!({
                    "type": "image",
                    "data": data,
                    "mimeType": "image/png"
                }));
            }
        }
        self
    }
}

fn arg_bool(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn parse_region(value: Option<&Value>) -> Result<Option<RectI>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let obj = value
        .as_object()
        .ok_or_else(|| "region must be an object with x, y, width and height".to_string())?;
    let number = |keys: &[&str]| -> Option<f64> {
        keys.iter()
            .find_map(|key| obj.get(*key).and_then(Value::as_f64))
    };
    let x = number(&["x"]).ok_or_else(|| "region.x is required".to_string())?;
    let y = number(&["y"]).ok_or_else(|| "region.y is required".to_string())?;
    let width = number(&["width", "w"]).ok_or_else(|| "region.width is required".to_string())?;
    let height = number(&["height", "h"]).ok_or_else(|| "region.height is required".to_string())?;
    if !(x.is_finite() && y.is_finite() && width.is_finite() && height.is_finite()) {
        return Err("region coordinates must be finite numbers".into());
    }
    if width < 1.0 || height < 1.0 || width > 100_000.0 || height > 100_000.0 {
        return Err("region width and height must be between 1 and 100000".into());
    }
    Ok(Some(RectI::new(
        x.round() as i32,
        y.round() as i32,
        width.round() as u32,
        height.round() as u32,
    )))
}

fn parse_annotations(args: &Value) -> Result<Vec<Annotation>, String> {
    let mut value = args
        .get("annotations")
        .cloned()
        .unwrap_or(Value::Array(vec![]));
    if value.is_null() {
        return Ok(Vec::new());
    }
    if let Some(raw) = value.as_str() {
        value = serde_json::from_str(raw)
            .map_err(|e| format!("annotations string is not valid JSON: {e}"))?;
    }
    let array = value
        .as_array()
        .ok_or_else(|| "annotations must be an array".to_string())?;
    if array.len() > 200 {
        return Err("At most 200 annotations are allowed".into());
    }
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            serde_json::from_value::<Annotation>(item.clone())
                .map_err(|e| format!("annotations[{index}] is invalid: {e}"))
        })
        .collect()
}

fn resolve_source_path(
    app: &AppHandle,
    state: &AppState,
    requested: Option<String>,
) -> Result<PathBuf, String> {
    let path = match requested {
        Some(raw) => {
            let mut p = util::expand_tilde(&raw);
            if p.is_relative() {
                p = export::capture_dir(app, state).join(p);
            }
            p
        }
        None => state
            .latest_capture
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .ok_or_else(|| {
                "No previous capture exists. Call take_screenshot first or provide sourcePath."
                    .to_string()
            })?,
    };
    if !path.is_file() {
        return Err(format!("Image not found: {}", path.display()));
    }
    let metadata = std::fs::metadata(&path).map_err(|e| format!("Cannot read image: {e}"))?;
    if metadata.len() > 120 * 1024 * 1024 {
        return Err("Image is larger than 120 MB".into());
    }
    Ok(path)
}

fn save_png_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    paths::ensure_parent(path).map_err(|e| format!("Cannot create output folder: {e}"))?;
    std::fs::write(path, bytes).map_err(|e| format!("Failed to save PNG: {e}"))
}

fn summary(value: Value) -> String {
    match serde_json::to_string_pretty(&value) {
        Ok(json) => format!("SnapMCP result:\n{json}"),
        Err(_) => "SnapMCP operation completed.".to_string(),
    }
}

fn annotation_warning(report: &annotate::RenderReport) -> Vec<String> {
    report.warnings.clone()
}

fn interactive_text(outcome: &InteractiveOutcome) -> String {
    summary(json!({
        "ok": true,
        "path": outcome.path,
        "width": outcome.width,
        "height": outcome.height,
        "region": outcome.region,
        "display": outcome.display,
        "annotations": outcome.annotations,
        "hint": "The user selected and confirmed this capture. Annotation coordinates are relative to this image."
    }))
}

async fn take_screenshot(
    app: &AppHandle,
    state: Arc<AppState>,
    args: Value,
) -> Result<ToolOutput, String> {
    let guard = state
        .acquire_busy()
        .ok_or_else(|| "Another screenshot is already in progress".to_string())?;
    let spec = DisplaySpec::parse(args.get("display"))?;
    let region = parse_region(args.get("region"))?;
    let annotations = parse_annotations(&args)?;
    let include_image = arg_bool(&args, "includeImage", true);
    let copy_to_clipboard = arg_bool(&args, "copyToClipboard", false);
    let requested_path = arg_str(&args, "savePath");

    let capture_result =
        tauri::async_runtime::spawn_blocking(move || capture::capture_for_mcp(spec, region))
            .await
            .map_err(|e| format!("Capture task failed: {e}"))??;

    let mut image = capture_result.image;
    let report = if annotations.is_empty() {
        annotate::RenderReport::default()
    } else {
        let annotations_for_task = annotations.clone();
        let (annotated, report) = tauri::async_runtime::spawn_blocking(move || {
            let mut target = image;
            let report = annotate::apply(&mut target, &annotations_for_task);
            (target, report)
        })
        .await
        .map_err(|e| format!("Annotation task failed: {e}"))?;
        image = annotated;
        report
    };

    let png = util::encode_png(&image)?;
    let dir = export::capture_dir(app, &state);
    let path = util::resolve_save_path(requested_path.as_deref(), &dir, "SnapMCP")?;
    save_png_bytes(&path, &png)?;
    state.set_latest_capture(path.clone());

    if copy_to_clipboard {
        export::copy_image(app, &image)?;
    }

    let applied = if annotations.is_empty() {
        0
    } else {
        annotations.len()
    };
    let text = summary(json!({
        "ok": true,
        "path": path.to_string_lossy(),
        "width": image.width(),
        "height": image.height(),
        "region": capture_result.region,
        "display": capture_result.display,
        "requestedDisplay": display_label(spec),
        "annotationsRequested": applied,
        "annotationsApplied": report.applied,
        "annotationsSkipped": report.skipped,
        "warnings": annotation_warning(&report),
        "copiedToClipboard": copy_to_clipboard,
        "hint": "Annotation coordinates are relative to this image (0,0 = top-left). Call annotate_screenshot to add or refine annotations using coordinates measured on this image."
    }));
    drop(guard);
    Ok(ToolOutput::text(text).with_image(&image, include_image))
}

async fn annotate_existing(
    app: &AppHandle,
    state: Arc<AppState>,
    args: Value,
) -> Result<ToolOutput, String> {
    let guard = state
        .acquire_busy()
        .ok_or_else(|| "Another capture is already in progress".to_string())?;
    let source = resolve_source_path(app, &state, arg_str(&args, "sourcePath"))?;
    let annotations = parse_annotations(&args)?;
    if annotations.is_empty() {
        return Err("annotations must contain at least one annotation".into());
    }
    let include_image = arg_bool(&args, "includeImage", true);
    let copy_to_clipboard = arg_bool(&args, "copyToClipboard", false);
    let requested_path = arg_str(&args, "savePath");

    let source_for_task = source.clone();
    let annotations_for_task = annotations.clone();
    let (image, report) = tauri::async_runtime::spawn_blocking(move || {
        let mut image = image::open(&source_for_task)
            .map_err(|e| format!("Failed to open image: {e}"))?
            .to_rgba8();
        let report = annotate::apply(&mut image, &annotations_for_task);
        Ok::<_, String>((image, report))
    })
    .await
    .map_err(|e| format!("Annotation task failed: {e}"))??;

    let png = util::encode_png(&image)?;
    let dir = export::capture_dir(app, &state);
    let path = util::resolve_save_path(requested_path.as_deref(), &dir, "SnapMCP")?;
    save_png_bytes(&path, &png)?;
    state.set_latest_capture(path.clone());

    if copy_to_clipboard {
        export::copy_image(app, &image)?;
    }

    let text = summary(json!({
        "ok": true,
        "path": path.to_string_lossy(),
        "sourcePath": source.to_string_lossy(),
        "width": image.width(),
        "height": image.height(),
        "annotationsRequested": annotations.len(),
        "annotationsApplied": report.applied,
        "annotationsSkipped": report.skipped,
        "warnings": report.warnings,
        "copiedToClipboard": copy_to_clipboard,
        "hint": "The returned image is the annotated result. Use its pixel coordinates for further annotations."
    }));
    drop(guard);
    Ok(ToolOutput::text(text).with_image(&image, include_image))
}

async fn capture_interactive(
    app: &AppHandle,
    state: Arc<AppState>,
    args: Value,
) -> Result<ToolOutput, String> {
    let hint = arg_str(&args, "hint").map(|h| {
        if h.chars().count() > 200 {
            h.chars().take(200).collect::<String>()
        } else {
            h
        }
    });
    let include_image = arg_bool(&args, "includeImage", true);
    let timeout_seconds = args
        .get("timeoutSeconds")
        .and_then(Value::as_u64)
        .unwrap_or(180)
        .clamp(5, 600);

    let rx = overlay::start_interactive(app.clone(), Arc::clone(&state), hint)
        .await
        .map_err(|e| format!("Could not open the interactive capture overlay: {e}"))?;

    let outcome =
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_seconds), rx).await {
            Err(_) => {
                overlay::cancel_all(app, &state, "Timed out waiting for the user");
                return Err(format!(
                "The user did not finish the interactive capture within {timeout_seconds} seconds."
            ));
            }
            Ok(Err(_)) => {
                return Err("The interactive capture was closed before it completed.".into());
            }
            Ok(Ok(Err(reason))) => {
                return Err(format!("The interactive capture was canceled: {reason}"));
            }
            Ok(Ok(Ok(outcome))) => outcome,
        };

    let bytes = std::fs::read(&outcome.path)
        .map_err(|e| format!("The confirmed screenshot could not be read: {e}"))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|e| format!("The confirmed screenshot is invalid: {e}"))?
        .to_rgba8();
    Ok(ToolOutput::text(interactive_text(&outcome)).with_image(&image, include_image))
}

async fn list_displays() -> Result<ToolOutput, String> {
    let monitors = tauri::async_runtime::spawn_blocking(capture::list_monitors)
        .await
        .map_err(|e| format!("Display task failed: {e}"))??;
    let cursor_index = capture::cursor_display_index(&monitors);
    let text = summary(json!({
        "ok": true,
        "coordinateSpace": "absolute physical virtual-desktop pixels",
        "monitors": monitors.iter().map(|m| {
            json!({
                "index": m.index,
                "name": m.name,
                "x": m.x,
                "y": m.y,
                "width": m.width,
                "height": m.height,
                "scaleFactor": m.scale_factor,
                "isPrimary": m.is_primary,
                "isCursor": Some(m.index) == cursor_index,
            })
        }).collect::<Vec<_>>(),
        "cursorIndex": cursor_index,
        "hint": "Pass a monitor index, 'primary', 'cursor' or 'all' as take_screenshot.display. Region coordinates are absolute, matching these x/y/width/height values."
    }));
    Ok(ToolOutput::text(text))
}

pub async fn execute(
    app: &AppHandle,
    state: Arc<AppState>,
    name: &str,
    args: Value,
) -> Result<ToolOutput, String> {
    let args = if args.is_null() { json!({}) } else { args };
    diag::info(format!("MCP tool call: {name}"));
    match name {
        "take_screenshot" => take_screenshot(app, state, args).await,
        "annotate_screenshot" => annotate_existing(app, state, args).await,
        "capture_interactive" => capture_interactive(app, state, args).await,
        "list_displays" => list_displays().await,
        _ => Err(format!("Unknown tool: {name}")),
    }
}

#[allow(dead_code)]
fn _monitor_json(monitor: &MonitorSnapshot) -> Value {
    serde_json::to_value(monitor).unwrap_or(Value::Null)
}
