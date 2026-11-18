use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A rectangle in physical screen pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl RectI {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> i64 {
        self.x as i64 + self.width as i64
    }

    pub fn bottom(&self) -> i64 {
        self.y as i64 + self.height as i64
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn intersect(&self, other: RectI) -> Option<RectI> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());
        if x2 <= x1 as i64 || y2 <= y1 as i64 {
            return None;
        }
        Some(RectI::new(
            x1,
            y1,
            (x2 - x1 as i64) as u32,
            (y2 - y1 as i64) as u32,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

impl MonitorSnapshot {
    pub fn bounds(&self) -> RectI {
        RectI::new(self.x, self.y, self.width, self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplaySpec {
    Primary,
    Cursor,
    All,
    Index(usize),
}

impl DisplaySpec {
    pub fn parse(value: Option<&Value>) -> Result<Self, String> {
        match value {
            None | Some(Value::Null) => Ok(DisplaySpec::Primary),
            Some(Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
                "" | "primary" => Ok(DisplaySpec::Primary),
                "cursor" => Ok(DisplaySpec::Cursor),
                "all" | "virtual" | "desktop" => Ok(DisplaySpec::All),
                other => other.parse::<usize>().map(DisplaySpec::Index).map_err(|_| {
                    format!(
                        "Invalid display '{other}'. Use 'primary', 'cursor', 'all', or a zero-based monitor index."
                    )
                }),
            },
            Some(Value::Number(n)) => n
                .as_u64()
                .map(|v| DisplaySpec::Index(v as usize))
                .ok_or_else(|| "display must be a non-negative integer".to_string()),
            _ => Err("display must be a string or integer".to_string()),
        }
    }
}

pub fn display_label(spec: DisplaySpec) -> String {
    match spec {
        DisplaySpec::Primary => "primary".into(),
        DisplaySpec::Cursor => "cursor".into(),
        DisplaySpec::All => "all".into(),
        DisplaySpec::Index(i) => i.to_string(),
    }
}
