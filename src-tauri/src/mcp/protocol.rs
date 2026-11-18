use serde_json::{json, Value};

pub const SERVER_NAME: &str = "snapmcp";
pub const SERVER_TITLE: &str = "SnapMCP";
pub const PROTOCOL_MODERN: &str = "2026-07-28";
pub const SUPPORTED_LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
pub const SUPPORTED_VERSIONS: &[&str] = &[
    PROTOCOL_MODERN,
    "2025-11-25",
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
];

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn server_info() -> Value {
    json!({
        "name": SERVER_NAME,
        "title": SERVER_TITLE,
        "version": version()
    })
}

pub fn instructions() -> &'static str {
    "SnapMCP captures the desktop for the user and can draw annotations on screenshots.\n\
     Workflow: call list_displays when you need monitor geometry; call take_screenshot to \
     inspect the screen; then call annotate_screenshot with coordinates measured on the \
     returned image. Use capture_interactive when the user should choose or confirm a region.\n\
     Coordinate rules: list_displays and take_screenshot.region use absolute physical \
     virtual-desktop pixels. Annotation coordinates and sizes use pixels relative to the \
     captured image, with (0,0) at its top-left. All returned image paths are PNG files."
}

pub fn metrics() -> Value {
    json!({
        "ttlMs": 60000,
        "cacheScope": "private"
    })
}

fn result_meta() -> Value {
    json!({ "io.modelcontextprotocol/serverInfo": server_info() })
}

pub fn add_modern_result(mut result: Value) -> Value {
    if let Some(obj) = result.as_object_mut() {
        obj.insert("resultType".into(), json!("complete"));
        obj.insert("_meta".into(), result_meta());
    }
    result
}

fn point_schema() -> Value {
    json!({
        "type": "array",
        "items": { "type": "number" },
        "minItems": 2,
        "maxItems": 2,
        "description": "Point as [x, y] in image pixels."
    })
}

fn annotation_schema() -> Value {
    let stroke = json!({
        "type": "number",
        "minimum": 0.5,
        "maximum": 80,
        "description": "Stroke width in image pixels. Default 4."
    });
    json!({
        "oneOf": [
            {
                "type": "object",
                "description": "Rectangle highlight around a UI element.",
                "properties": {
                    "type": { "enum": ["rect"] },
                    "x": { "type": "number" }, "y": { "type": "number" },
                    "w": { "type": "number", "minimum": 1 }, "h": { "type": "number", "minimum": 1 },
                    "color": { "type": "string", "description": "Border color as #RGB, #RRGGBB or #RRGGBBAA." },
                    "strokeWidth": stroke.clone(),
                    "fill": { "type": "string", "description": "Optional translucent fill color." }
                },
                "required": ["type", "x", "y", "w", "h"]
            },
            {
                "type": "object",
                "description": "Ellipse or circle around a UI element.",
                "properties": {
                    "type": { "enum": ["ellipse"] },
                    "x": { "type": "number" }, "y": { "type": "number" },
                    "w": { "type": "number", "minimum": 1 }, "h": { "type": "number", "minimum": 1 },
                    "color": { "type": "string" },
                    "strokeWidth": stroke.clone(),
                    "fill": { "type": "string" }
                },
                "required": ["type", "x", "y", "w", "h"]
            },
            {
                "type": "object",
                "description": "Arrow pointing at a button, input or feature. Preferred annotation.",
                "properties": {
                    "type": { "enum": ["arrow"] },
                    "from": point_schema(), "to": point_schema(),
                    "color": { "type": "string" },
                    "strokeWidth": stroke.clone(),
                    "headSize": { "type": "number", "minimum": 4, "maximum": 200 }
                },
                "required": ["type", "from", "to"]
            },
            {
                "type": "object",
                "description": "Straight line.",
                "properties": {
                    "type": { "enum": ["line"] },
                    "from": point_schema(), "to": point_schema(),
                    "color": { "type": "string" },
                    "strokeWidth": stroke.clone()
                },
                "required": ["type", "from", "to"]
            },
            {
                "type": "object",
                "description": "Freehand pen stroke through a list of points.",
                "properties": {
                    "type": { "enum": ["pen"] },
                    "points": { "type": "array", "minItems": 2, "maxItems": 5000, "items": point_schema() },
                    "color": { "type": "string" },
                    "strokeWidth": stroke.clone()
                },
                "required": ["type", "points"]
            },
            {
                "type": "object",
                "description": "Text label such as '① Click Login'. Supports Chinese and line breaks with \\n.",
                "properties": {
                    "type": { "enum": ["text"] },
                    "x": { "type": "number" }, "y": { "type": "number" },
                    "text": { "type": "string", "maxLength": 2000 },
                    "color": { "type": "string", "description": "Text color. Default white." },
                    "fontSize": { "type": "number", "minimum": 10, "maximum": 200, "description": "Font size in image pixels. Default 28." },
                    "bgColor": { "type": "string", "description": "Background chip color. Default #0B1220CC. Use 'none' for no background." },
                    "padding": { "type": "number", "minimum": 0, "maximum": 48 }
                },
                "required": ["type", "x", "y", "text"]
            },
            {
                "type": "object",
                "description": "Translucent highlighter box.",
                "properties": {
                    "type": { "enum": ["highlight"] },
                    "x": { "type": "number" }, "y": { "type": "number" },
                    "w": { "type": "number", "minimum": 1 }, "h": { "type": "number", "minimum": 1 },
                    "color": { "type": "string", "description": "Default translucent yellow #FFD60A59." }
                },
                "required": ["type", "x", "y", "w", "h"]
            },
            {
                "type": "object",
                "description": "Pixelates sensitive information such as emails or tokens.",
                "properties": {
                    "type": { "enum": ["mosaic"] },
                    "x": { "type": "number" }, "y": { "type": "number" },
                    "w": { "type": "number", "minimum": 1 }, "h": { "type": "number", "minimum": 1 },
                    "blockSize": { "type": "integer", "minimum": 2, "maximum": 64, "description": "Pixel block size. Default 16." }
                },
                "required": ["type", "x", "y", "w", "h"]
            },
            {
                "type": "object",
                "description": "Numbered circle for step-by-step instructions.",
                "properties": {
                    "type": { "enum": ["step"] },
                    "x": { "type": "number", "description": "Center X in image pixels." },
                    "y": { "type": "number", "description": "Center Y in image pixels." },
                    "index": { "type": "integer", "minimum": 1, "maximum": 999 },
                    "color": { "type": "string" },
                    "size": { "type": "number", "minimum": 18, "maximum": 160, "description": "Circle diameter. Default 32." }
                },
                "required": ["type", "x", "y", "index"]
            }
        ]
    })
}

fn region_schema() -> Value {
    json!({
        "type": "object",
        "description": "Absolute virtual-desktop rectangle in physical pixels.",
        "properties": {
            "x": { "type": "integer" },
            "y": { "type": "integer" },
            "width": { "type": "integer", "minimum": 1 },
            "height": { "type": "integer", "minimum": 1 }
        },
        "required": ["x", "y", "width", "height"]
    })
}

fn display_schema() -> Value {
    json!({
        "oneOf": [
            { "type": "integer", "minimum": 0 },
            { "type": "string", "enum": ["primary", "cursor", "all"] }
        ],
        "default": "primary",
        "description": "Monitor selector: 'primary', 'cursor', 'all' (virtual desktop), or a zero-based index from list_displays. Ignored when region is provided."
    })
}

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "take_screenshot",
            "title": "Take a screenshot",
            "description": "Capture the user's desktop (primary display by default) and optionally draw annotations. Returns the PNG path plus the image itself. Annotation coordinates are relative to the captured image; region is in absolute screen pixels. For precise UI pointing: first take a screenshot, inspect it, then call annotate_screenshot with coordinates from that image.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "display": display_schema(),
                    "region": region_schema(),
                    "annotations": {
                        "type": "array",
                        "maxItems": 200,
                        "items": { "$ref": "#/$defs/annotation" },
                        "description": "Optional annotations drawn onto the screenshot before it is returned."
                    },
                    "savePath": { "type": "string", "description": "Optional .png output path. Relative paths are saved inside the SnapMCP capture folder." },
                    "copyToClipboard": { "type": "boolean", "default": false },
                    "includeImage": { "type": "boolean", "default": true, "description": "Set false to receive only the file path and metadata." }
                },
                "$defs": { "annotation": annotation_schema() },
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false }
        }),
        json!({
            "name": "annotate_screenshot",
            "title": "Annotate an existing screenshot",
            "description": "Draw annotations on the most recent capture, or on an existing PNG/JPEG path. Use image-relative pixel coordinates. Returns the new PNG path and image.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sourcePath": { "type": "string", "description": "Image to annotate. Defaults to SnapMCP's latest capture." },
                    "annotations": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 200,
                        "items": { "$ref": "#/$defs/annotation" }
                    },
                    "savePath": { "type": "string" },
                    "copyToClipboard": { "type": "boolean", "default": false },
                    "includeImage": { "type": "boolean", "default": true }
                },
                "required": ["annotations"],
                "$defs": { "annotation": annotation_schema() },
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false }
        }),
        json!({
            "name": "capture_interactive",
            "title": "Ask the user to select and annotate a screenshot",
            "description": "Show the SnapMCP selection overlay to the human. The user selects a region, can annotate it, and confirms. Ideal when the target is ambiguous or when a human-in-the-loop confirmation is required. Waits until the user finishes or the timeout expires.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hint": { "type": "string", "maxLength": 200, "description": "Short instruction shown at the top of the capture overlay." },
                    "timeoutSeconds": { "type": "integer", "minimum": 5, "maximum": 600, "default": 180 },
                    "includeImage": { "type": "boolean", "default": true }
                },
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false }
        }),
        json!({
            "name": "list_displays",
            "title": "List displays",
            "description": "List monitors with absolute virtual-desktop bounds (physical pixels), scale factor, primary flag and cursor flag. Use these coordinates for take_screenshot.region.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false }
        }),
    ]
}

pub fn legacy_negotiate(requested: &str) -> String {
    if SUPPORTED_LEGACY.contains(&requested) {
        requested.to_string()
    } else {
        "2025-11-25".to_string()
    }
}
