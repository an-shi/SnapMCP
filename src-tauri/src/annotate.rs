use ab_glyph::{FontVec, PxScale};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_text_mut, text_size};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::models::RectI;

const DEFAULT_COLOR: &str = "#FF453A";
const DEFAULT_TEXT_COLOR: &str = "#FFFFFF";
const DEFAULT_TEXT_BG: &str = "#0B1220CC";
const DEFAULT_HIGHLIGHT: &str = "#FFD60A59";

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Serialize for Point {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        [self.x, self.y].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Point {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Tuple([f32; 2]),
            Object { x: f32, y: f32 },
        }
        match Repr::deserialize(deserializer)? {
            Repr::Tuple([x, y]) => Ok(Point { x, y }),
            Repr::Object { x, y } => Ok(Point { x, y }),
        }
    }
}

fn default_color() -> String {
    DEFAULT_COLOR.into()
}
fn default_stroke() -> f32 {
    4.0
}
fn default_font_size() -> f32 {
    28.0
}
fn default_text_color() -> String {
    DEFAULT_TEXT_COLOR.into()
}
fn default_text_bg() -> Option<String> {
    Some(DEFAULT_TEXT_BG.into())
}
fn default_padding() -> f32 {
    10.0
}
fn default_highlight() -> String {
    DEFAULT_HIGHLIGHT.into()
}
fn default_block() -> u32 {
    16
}
fn default_index() -> u32 {
    1
}
fn default_step_size() -> f32 {
    32.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Annotation {
    #[serde(alias = "rectangle", alias = "box")]
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default = "default_color")]
        color: String,
        #[serde(
            default = "default_stroke",
            alias = "stroke",
            alias = "thickness",
            alias = "lineWidth"
        )]
        stroke_width: f32,
        #[serde(default)]
        fill: Option<String>,
    },
    #[serde(alias = "circle", alias = "oval")]
    Ellipse {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default = "default_color")]
        color: String,
        #[serde(default = "default_stroke", alias = "stroke", alias = "thickness")]
        stroke_width: f32,
        #[serde(default)]
        fill: Option<String>,
    },
    Arrow {
        from: Point,
        to: Point,
        #[serde(default = "default_color")]
        color: String,
        #[serde(default = "default_stroke", alias = "stroke", alias = "thickness")]
        stroke_width: f32,
        #[serde(default, alias = "head")]
        head_size: Option<f32>,
    },
    Line {
        from: Point,
        to: Point,
        #[serde(default = "default_color")]
        color: String,
        #[serde(default = "default_stroke", alias = "stroke", alias = "thickness")]
        stroke_width: f32,
    },
    Pen {
        points: Vec<Point>,
        #[serde(default = "default_color")]
        color: String,
        #[serde(default = "default_stroke", alias = "stroke", alias = "thickness")]
        stroke_width: f32,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        #[serde(default = "default_text_color")]
        color: String,
        #[serde(default = "default_font_size", alias = "size")]
        font_size: f32,
        #[serde(default = "default_text_bg", alias = "background")]
        bg_color: Option<String>,
        #[serde(default = "default_padding")]
        padding: f32,
    },
    Highlight {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default = "default_highlight")]
        color: String,
    },
    #[serde(alias = "pixelate")]
    Mosaic {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default = "default_block", alias = "block")]
        block_size: u32,
    },
    Step {
        x: f32,
        y: f32,
        #[serde(default = "default_index")]
        index: u32,
        #[serde(default = "default_color")]
        color: String,
        #[serde(default = "default_step_size", alias = "radius")]
        size: f32,
    },
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderReport {
    pub applied: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

struct FontInfo {
    font: FontVec,
}

fn font_candidates() -> Vec<(String, u32)> {
    if let Ok(custom) = std::env::var("SNAPMCP_FONT") {
        let (path, index) = match custom.rsplit_once('#') {
            Some((p, i)) => (p.to_string(), i.parse::<u32>().unwrap_or(0)),
            None => (custom, 0),
        };
        return vec![(path, index)];
    }
    let mut out: Vec<(String, u32)> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let fonts = std::path::Path::new(&windir).join("Fonts");
        for name in [
            "msyh.ttc",
            "msyhbd.ttc",
            "Deng.ttf",
            "simhei.ttf",
            "simsun.ttc",
            "segoeui.ttf",
            "arial.ttf",
        ] {
            out.push((fonts.join(name).to_string_lossy().to_string(), 0));
        }
    }
    #[cfg(target_os = "macos")]
    {
        for (path, idx) in [
            ("/System/Library/Fonts/PingFang.ttc", 0u32),
            ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
            ("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", 0),
            ("/System/Library/Fonts/Helvetica.ttc", 0),
        ] {
            out.push((path.to_string(), idx));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for (path, idx) in [
            (
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                0u32,
            ),
            ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
            ("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc", 0),
            ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 0),
            ("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", 0),
        ] {
            out.push((path.to_string(), idx));
        }
    }
    out
}

fn load_font() -> Option<FontInfo> {
    for (path, index) in font_candidates() {
        let p = std::path::Path::new(&path);
        if !p.is_file() {
            continue;
        }
        match std::fs::read(p) {
            Ok(bytes) => match FontVec::try_from_vec_and_index(bytes, index) {
                Ok(font) => {
                    crate::diag::info(format!("Using annotation font {path} (face {index})"));
                    return Some(FontInfo { font });
                }
                Err(err) => crate::diag::warn(format!(
                    "Font {path} (face {index}) could not be loaded: {err}"
                )),
            },
            Err(err) => crate::diag::warn(format!("Font {path} could not be read: {err}")),
        }
    }
    crate::diag::warn("No usable system font was found; text annotations will be skipped");
    None
}

fn parse_hex_pair(s: &str) -> Option<u8> {
    u8::from_str_radix(s, 16).ok()
}

pub fn parse_color(input: &str) -> Option<Rgba<u8>> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    let named = match lower.as_str() {
        "transparent" | "none" => Some(Rgba([0, 0, 0, 0])),
        "red" => Some(Rgba([255, 59, 48, 255])),
        "orange" => Some(Rgba([255, 149, 0, 255])),
        "yellow" => Some(Rgba([255, 214, 10, 255])),
        "green" => Some(Rgba([52, 199, 89, 255])),
        "blue" => Some(Rgba([10, 132, 255, 255])),
        "cyan" => Some(Rgba([50, 210, 224, 255])),
        "purple" => Some(Rgba([191, 90, 242, 255])),
        "pink" => Some(Rgba([255, 55, 95, 255])),
        "magenta" => Some(Rgba([255, 0, 255, 255])),
        "white" => Some(Rgba([255, 255, 255, 255])),
        "black" => Some(Rgba([0, 0, 0, 255])),
        "gray" | "grey" => Some(Rgba([142, 142, 147, 255])),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    if let Some(hex) = s.strip_prefix('#') {
        return match hex.len() {
            3 => {
                let r = parse_hex_pair(&hex[0..1].repeat(2))?;
                let g = parse_hex_pair(&hex[1..2].repeat(2))?;
                let b = parse_hex_pair(&hex[2..3].repeat(2))?;
                Some(Rgba([r, g, b, 255]))
            }
            4 => {
                let r = parse_hex_pair(&hex[0..1].repeat(2))?;
                let g = parse_hex_pair(&hex[1..2].repeat(2))?;
                let b = parse_hex_pair(&hex[2..3].repeat(2))?;
                let a = parse_hex_pair(&hex[3..4].repeat(2))?;
                Some(Rgba([r, g, b, a]))
            }
            6 => Some(Rgba([
                parse_hex_pair(&hex[0..2])?,
                parse_hex_pair(&hex[2..4])?,
                parse_hex_pair(&hex[4..6])?,
                255,
            ])),
            8 => Some(Rgba([
                parse_hex_pair(&hex[0..2])?,
                parse_hex_pair(&hex[2..4])?,
                parse_hex_pair(&hex[4..6])?,
                parse_hex_pair(&hex[6..8])?,
            ])),
            _ => None,
        };
    }
    if let Some(inner) = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))
        .and_then(|v| v.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 || parts.len() == 4 {
            let r = parts[0].parse::<f32>().ok()?.clamp(0.0, 255.0) as u8;
            let g = parts[1].parse::<f32>().ok()?.clamp(0.0, 255.0) as u8;
            let b = parts[2].parse::<f32>().ok()?.clamp(0.0, 255.0) as u8;
            let a = if parts.len() == 4 {
                let raw = parts[3].trim();
                if let Some(pct) = raw.strip_suffix('%') {
                    (pct.parse::<f32>().ok()? / 100.0 * 255.0).clamp(0.0, 255.0) as u8
                } else {
                    let value = raw.parse::<f32>().ok()?;
                    if value <= 1.0 {
                        (value * 255.0).clamp(0.0, 255.0) as u8
                    } else {
                        value.clamp(0.0, 255.0) as u8
                    }
                }
            } else {
                255
            };
            return Some(Rgba([r, g, b, a]));
        }
    }
    None
}

fn color_or(value: &str, fallback: Rgba<u8>) -> Rgba<u8> {
    parse_color(value).unwrap_or(fallback)
}

fn blend_pixel(image: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>, coverage: f32) {
    if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 || coverage <= 0.0
    {
        return;
    }
    let alpha = (color.0[3] as f32 / 255.0) * coverage.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let dst = image.get_pixel_mut(x as u32, y as u32);
    let da = dst.0[3] as f32 / 255.0;
    let out_a = alpha + da * (1.0 - alpha);
    if out_a <= 0.0 {
        return;
    }
    for c in 0..3 {
        let src = color.0[c] as f32 / 255.0;
        let dst_c = dst.0[c] as f32 / 255.0;
        let out = (src * alpha + dst_c * da * (1.0 - alpha)) / out_a;
        dst.0[c] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst.0[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn fill_rect(image: &mut RgbaImage, rect: RectI, color: Rgba<u8>) {
    if rect.is_empty() {
        return;
    }
    let x0 = rect.x.max(0) as u32;
    let y0 = rect.y.max(0) as u32;
    let x1 = rect.right().min(image.width() as i64).max(0) as u32;
    let y1 = rect.bottom().min(image.height() as i64).max(0) as u32;
    for y in y0..y1 {
        for x in x0..x1 {
            blend_pixel(image, x as i32, y as i32, color, 1.0);
        }
    }
}

fn fill_round_rect(image: &mut RgbaImage, rect: RectI, radius: f32, color: Rgba<u8>) {
    if rect.is_empty() {
        return;
    }
    let radius = radius
        .max(0.0)
        .min(rect.width as f32 / 2.0)
        .min(rect.height as f32 / 2.0);
    if radius < 0.5 {
        fill_rect(image, rect, color);
        return;
    }
    let x1 = rect.right().min(image.width() as i64) as i32;
    let y1 = rect.bottom().min(image.height() as i64) as i32;
    let cx = rect.x as f32 + rect.width as f32 / 2.0;
    let cy = rect.y as f32 + rect.height as f32 / 2.0;
    let hw = rect.width as f32 / 2.0 - radius;
    let hh = rect.height as f32 / 2.0 - radius;
    for y in rect.y.max(0)..y1 {
        for x in rect.x.max(0)..x1 {
            let qx = (x as f32 + 0.5 - cx).abs() - hw;
            let qy = (y as f32 + 0.5 - cy).abs() - hh;
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
            let inside = qx.max(qy).min(0.0);
            let dist = outside + inside - radius;
            let coverage = (0.5 - dist).clamp(0.0, 1.0);
            if coverage > 0.0 {
                blend_pixel(image, x, y, color, coverage);
            }
        }
    }
}

fn fill_circle(image: &mut RgbaImage, cx: f32, cy: f32, radius: f32, color: Rgba<u8>) {
    if radius <= 0.0 {
        return;
    }
    let x0 = (cx - radius - 1.0).floor() as i32;
    let y0 = (cy - radius - 1.0).floor() as i32;
    let x1 = (cx + radius + 1.0).ceil() as i32;
    let y1 = (cy + radius + 1.0).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let coverage = (radius + 0.5 - d).clamp(0.0, 1.0);
            if coverage > 0.0 {
                blend_pixel(image, x, y, color, coverage);
            }
        }
    }
}

fn stroke_circle(
    image: &mut RgbaImage,
    cx: f32,
    cy: f32,
    radius: f32,
    width: f32,
    color: Rgba<u8>,
) {
    if radius <= 0.0 || width <= 0.0 {
        return;
    }
    let outer = radius + width / 2.0;
    let x0 = (cx - outer - 1.0).floor() as i32;
    let y0 = (cy - outer - 1.0).floor() as i32;
    let x1 = (cx + outer + 1.0).ceil() as i32;
    let y1 = (cy + outer + 1.0).ceil() as i32;
    let half = width / 2.0;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let coverage = (half + 0.5 - (d - radius).abs()).clamp(0.0, 1.0);
            if coverage > 0.0 {
                blend_pixel(image, x, y, color, coverage);
            }
        }
    }
}

fn stroke_rect(image: &mut RgbaImage, rect: RectI, width: f32, color: Rgba<u8>) {
    if rect.is_empty() || width <= 0.0 {
        return;
    }
    let w = width.round().max(1.0) as u32;
    fill_rect(image, RectI::new(rect.x, rect.y, rect.width, w), color);
    fill_rect(
        image,
        RectI::new(rect.x, rect.bottom() as i32 - w as i32, rect.width, w),
        color,
    );
    fill_rect(image, RectI::new(rect.x, rect.y, w, rect.height), color);
    fill_rect(
        image,
        RectI::new(rect.right() as i32 - w as i32, rect.y, w, rect.height),
        color,
    );
}

fn thick_line(image: &mut RgbaImage, from: Point, to: Point, width: f32, color: Rgba<u8>) {
    if width <= 0.0 {
        return;
    }
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len_sq = dx * dx + dy * dy;
    let half = width / 2.0;
    let x0 = (from.x.min(to.x) - half - 1.0).floor() as i32;
    let y0 = (from.y.min(to.y) - half - 1.0).floor() as i32;
    let x1 = (from.x.max(to.x) + half + 1.0).ceil() as i32;
    let y1 = (from.y.max(to.y) + half + 1.0).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = if len_sq <= f32::EPSILON {
                0.0
            } else {
                (((px - from.x) * dx + (py - from.y) * dy) / len_sq).clamp(0.0, 1.0)
            };
            let qx = from.x + t * dx;
            let qy = from.y + t * dy;
            let d = ((px - qx).powi(2) + (py - qy).powi(2)).sqrt();
            let coverage = (half + 0.5 - d).clamp(0.0, 1.0);
            if coverage > 0.0 {
                blend_pixel(image, x, y, color, coverage);
            }
        }
    }
}

fn stroke_ellipse(
    image: &mut RgbaImage,
    rect: RectI,
    width: f32,
    color: Rgba<u8>,
    fill: Option<Rgba<u8>>,
) {
    if rect.is_empty() {
        return;
    }
    let cx = rect.x as f32 + rect.width as f32 / 2.0;
    let cy = rect.y as f32 + rect.height as f32 / 2.0;
    let rx = (rect.width as f32 / 2.0).max(1.0);
    let ry = (rect.height as f32 / 2.0).max(1.0);
    let half = (width / 2.0).max(0.0);
    let scale = rx.min(ry);
    let ext = half + 2.0;
    let x0 = (cx - rx - ext).floor() as i32;
    let y0 = (cy - ry - ext).floor() as i32;
    let x1 = (cx + rx + ext).ceil() as i32;
    let y1 = (cy + ry + ext).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let nx = (x as f32 + 0.5 - cx) / rx;
            let ny = (y as f32 + 0.5 - cy) / ry;
            let d = (nx * nx + ny * ny).sqrt();
            if let Some(fill_color) = fill {
                let inside = (1.0 - d * d).max(0.0);
                let coverage = (inside * 30.0).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    blend_pixel(image, x, y, fill_color, coverage);
                }
            }
            if half > 0.0 {
                let edge_dist = (d - 1.0).abs() * scale;
                let coverage = (half + 0.5 - edge_dist).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    blend_pixel(image, x, y, color, coverage);
                }
            }
        }
    }
}

fn fill_triangle(image: &mut RgbaImage, a: Point, b: Point, c: Point, color: Rgba<u8>) {
    let min_x = a.x.min(b.x).min(c.x).floor() as i32 - 1;
    let max_x = a.x.max(b.x).max(c.x).ceil() as i32 + 1;
    let min_y = a.y.min(b.y).min(c.y).floor() as i32 - 1;
    let max_y = a.y.max(b.y).max(c.y).ceil() as i32 + 1;

    let edge = |p: Point, q: Point, x: f32, y: f32| -> f32 {
        (x - p.x) * (q.y - p.y) - (y - p.y) * (q.x - p.x)
    };
    let area = edge(a, b, c.x, c.y);
    if area.abs() < f32::EPSILON {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let mut inside = 0.0;
            for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let px = x as f32 + sx;
                let py = y as f32 + sy;
                let w0 = edge(b, c, px, py);
                let w1 = edge(c, a, px, py);
                let w2 = edge(a, b, px, py);
                if (area > 0.0 && w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0)
                    || (area < 0.0 && w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0)
                {
                    inside += 1.0;
                }
            }
            if inside > 0.0 {
                blend_pixel(image, x, y, color, inside / 4.0);
            }
        }
    }
}

fn draw_arrow(
    image: &mut RgbaImage,
    from: Point,
    to: Point,
    width: f32,
    head_size: Option<f32>,
    color: Rgba<u8>,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 2.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let head = head_size
        .unwrap_or(width * 5.0)
        .clamp(8.0, (len * 0.7).max(8.0));
    let head_w = head * 0.52;
    let base = Point {
        x: to.x - ux * head,
        y: to.y - uy * head,
    };
    // Shaft, stopping at the head base.
    thick_line(
        image,
        from,
        Point {
            x: base.x + ux * 2.0,
            y: base.y + uy * 2.0,
        },
        width,
        color,
    );
    let perp = Point {
        x: -uy * head_w,
        y: ux * head_w,
    };
    fill_triangle(
        image,
        Point {
            x: base.x + perp.x,
            y: base.y + perp.y,
        },
        Point {
            x: base.x - perp.x,
            y: base.y - perp.y,
        },
        to,
        color,
    );
}

fn draw_text_block(
    image: &mut RgbaImage,
    x: f32,
    y: f32,
    text: &str,
    font: &FontVec,
    font_size: f32,
    color: Rgba<u8>,
    bg_color: Option<Rgba<u8>>,
    padding: f32,
) -> bool {
    let text = text.trim_end_matches(['\r', '\n']);
    if text.trim().is_empty() {
        return false;
    }
    let font_size = font_size.clamp(10.0, 300.0);
    let padding = padding.clamp(0.0, 48.0);
    let scale = PxScale::from(font_size);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut max_w = 0u32;
    let mut line_h = font_size * 1.35;
    for line in &lines {
        let (w, h) = text_size(scale, font, line);
        max_w = max_w.max(w);
        line_h = line_h.max(h as f32);
    }
    let block_w = max_w as f32 + padding * 2.0;
    let block_h = line_h * lines.len() as f32 + padding * 2.0;
    if block_w < 2.0 || block_h < 2.0 {
        return false;
    }
    let max_x = image.width() as f32 - 1.0;
    let max_y = image.height() as f32 - 1.0;
    let left = x.min(max_x).max(0.0);
    let top = y.min(max_y).max(0.0);
    let rect = RectI::new(
        left.round() as i32,
        top.round() as i32,
        block_w.ceil().min(max_x + 1.0) as u32,
        block_h.ceil().min(max_y + 1.0) as u32,
    );
    if let Some(bg) = bg_color {
        if bg.0[3] > 0 {
            fill_round_rect(image, rect, 8.0_f32.min(padding.max(4.0)), bg);
        }
    }
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        draw_text_mut(
            image,
            color,
            (left + padding).round() as i32,
            (top + padding + line_h * i as f32).round() as i32,
            scale,
            font,
            line,
        );
    }
    true
}

fn draw_mosaic(image: &mut RgbaImage, rect: RectI, block_size: u32) {
    let Some(clipped) = rect.intersect(RectI::new(0, 0, image.width(), image.height())) else {
        return;
    };
    if clipped.width < 2 || clipped.height < 2 {
        return;
    }
    let block = block_size.clamp(2, 64);
    let small_w = (clipped.width / block).max(1);
    let small_h = (clipped.height / block).max(1);
    let source = image::imageops::crop_imm(
        image,
        clipped.x as u32,
        clipped.y as u32,
        clipped.width,
        clipped.height,
    )
    .to_image();
    let small = image::imageops::resize(
        &source,
        small_w,
        small_h,
        image::imageops::FilterType::Nearest,
    );
    let pixelated = image::imageops::resize(
        &small,
        clipped.width,
        clipped.height,
        image::imageops::FilterType::Nearest,
    );
    image::imageops::replace(image, &pixelated, clipped.x as i64, clipped.y as i64);
}

fn annotation_rect(x: f32, y: f32, w: f32, h: f32) -> RectI {
    let left = x.min(x + w).round();
    let top = y.min(y + h).round();
    let width = w.abs().round().max(1.0);
    let height = h.abs().round().max(1.0);
    RectI::new(left as i32, top as i32, width as u32, height as u32)
}

pub fn apply(image: &mut RgbaImage, annotations: &[Annotation]) -> RenderReport {
    let mut report = RenderReport::default();
    let needs_text = annotations
        .iter()
        .any(|a| matches!(a, Annotation::Text { .. } | Annotation::Step { .. }));
    let font_info = if needs_text { load_font() } else { None };
    if needs_text && font_info.is_none() {
        report
            .warnings
            .push("No system font available; text and step labels were skipped".into());
    }

    for annotation in annotations {
        match annotation {
            Annotation::Rect {
                x,
                y,
                w,
                h,
                color,
                stroke_width,
                fill,
            } => {
                let rect = annotation_rect(*x, *y, *w, *h);
                if let Some(fill) = fill {
                    fill_rect(image, rect, color_or(fill, Rgba([255, 69, 58, 48])));
                }
                stroke_rect(
                    image,
                    rect,
                    *stroke_width,
                    color_or(color, Rgba([255, 69, 58, 255])),
                );
                report.applied += 1;
            }
            Annotation::Ellipse {
                x,
                y,
                w,
                h,
                color,
                stroke_width,
                fill,
            } => {
                let rect = annotation_rect(*x, *y, *w, *h);
                stroke_ellipse(
                    image,
                    rect,
                    *stroke_width,
                    color_or(color, Rgba([255, 69, 58, 255])),
                    fill.as_deref()
                        .and_then(parse_color)
                        .map(|c| Rgba([c.0[0], c.0[1], c.0[2], c.0[3].min(90)])),
                );
                report.applied += 1;
            }
            Annotation::Arrow {
                from,
                to,
                color,
                stroke_width,
                head_size,
            } => {
                draw_arrow(
                    image,
                    *from,
                    *to,
                    stroke_width.max(1.0),
                    *head_size,
                    color_or(color, Rgba([255, 69, 58, 255])),
                );
                report.applied += 1;
            }
            Annotation::Line {
                from,
                to,
                color,
                stroke_width,
            } => {
                thick_line(
                    image,
                    *from,
                    *to,
                    stroke_width.max(1.0),
                    color_or(color, Rgba([255, 69, 58, 255])),
                );
                report.applied += 1;
            }
            Annotation::Pen {
                points,
                color,
                stroke_width,
            } => {
                if points.len() >= 2 {
                    let c = color_or(color, Rgba([255, 69, 58, 255]));
                    for pair in points.windows(2) {
                        thick_line(image, pair[0], pair[1], stroke_width.max(1.0), c);
                    }
                    report.applied += 1;
                } else {
                    report.skipped += 1;
                }
            }
            Annotation::Text {
                x,
                y,
                text,
                color,
                font_size,
                bg_color,
                padding,
            } => {
                if text.chars().count() > 2000 {
                    report.skipped += 1;
                    report
                        .warnings
                        .push("Text annotation exceeded 2000 characters".into());
                    continue;
                }
                if let Some(info) = &font_info {
                    let drawn = draw_text_block(
                        image,
                        *x,
                        *y,
                        text,
                        &info.font,
                        *font_size,
                        color_or(color, Rgba([255, 255, 255, 255])),
                        bg_color.as_deref().and_then(parse_color),
                        *padding,
                    );
                    if drawn {
                        report.applied += 1;
                    } else {
                        report.skipped += 1;
                    }
                } else {
                    report.skipped += 1;
                }
            }
            Annotation::Highlight { x, y, w, h, color } => {
                fill_round_rect(
                    image,
                    annotation_rect(*x, *y, *w, *h),
                    4.0,
                    color_or(color, Rgba([255, 214, 10, 89])),
                );
                report.applied += 1;
            }
            Annotation::Mosaic {
                x,
                y,
                w,
                h,
                block_size,
            } => {
                draw_mosaic(image, annotation_rect(*x, *y, *w, *h), *block_size);
                report.applied += 1;
            }
            Annotation::Step {
                x,
                y,
                index,
                color,
                size,
            } => {
                let d = size.clamp(18.0, 160.0);
                let radius = d / 2.0;
                let c = color_or(color, Rgba([255, 69, 58, 255]));
                // Subtle dark rim keeps the badge readable on light captures.
                stroke_circle(image, *x, *y, radius + 1.0, 3.0, Rgba([0, 0, 0, 60]));
                fill_circle(image, *x, *y, radius, c);
                stroke_circle(image, *x, *y, radius - 1.5, 2.0, Rgba([255, 255, 255, 235]));
                if let Some(info) = &font_info {
                    let text = index.to_string();
                    let font_size = (d * 0.5).clamp(10.0, 80.0);
                    let scale = PxScale::from(font_size);
                    let (tw, th) = text_size(scale, &info.font, &text);
                    draw_text_mut(
                        image,
                        Rgba([255, 255, 255, 255]),
                        (*x - tw as f32 / 2.0).round() as i32,
                        (*y - th as f32 / 2.0 - font_size * 0.06).round() as i32,
                        scale,
                        &info.font,
                        &text,
                    );
                }
                report.applied += 1;
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("#ff0000"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_color("#0af8"), Some(Rgba([0, 170, 255, 136])));
        assert_eq!(
            parse_color("rgba(10,20,30,0.5)"),
            Some(Rgba([10, 20, 30, 127]))
        );
        assert!(parse_color("definitely-not-a-color").is_none());
    }

    #[test]
    fn parses_annotations_from_ai_json() {
        let json = serde_json::json!([
            {"type":"rect","x":10,"y":20,"w":100,"h":50,"color":"#ff0000","strokeWidth":3},
            {"type":"arrow","from":[1,2],"to":{"x":30,"y":40},"strokeWidth":5},
            {"type":"text","x":10,"y":10,"text":"① 点击登录","fontSize":24,"bgColor":"#00000080"},
            {"type":"step","x":100,"y":100,"index":2}
        ]);
        let parsed: Vec<Annotation> = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.len(), 4);
        let json = serde_json::to_value(&parsed).unwrap();
        assert!(json[1]["from"].is_array());
    }

    #[test]
    fn renders_shapes_without_panicking() {
        let mut image = RgbaImage::from_pixel(200, 120, Rgba([20, 20, 20, 255]));
        let annotations = vec![
            Annotation::Rect {
                x: -10.0,
                y: -10.0,
                w: 120.0,
                h: 80.0,
                color: "#ff0000".into(),
                stroke_width: 3.0,
                fill: Some("#00ff0044".into()),
            },
            Annotation::Arrow {
                from: Point { x: 160.0, y: 10.0 },
                to: Point { x: 220.0, y: 100.0 },
                color: "#0af".into(),
                stroke_width: 4.0,
                head_size: None,
            },
            Annotation::Mosaic {
                x: 20.0,
                y: 20.0,
                w: 80.0,
                h: 40.0,
                block_size: 8,
            },
        ];
        let report = apply(&mut image, &annotations);
        assert_eq!(report.applied, 3);
        assert_ne!(image.get_pixel(1, 1).0[0], 20);
    }
}
