use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::capture::{Frame, RectPx};

use super::bitmap::Bitmap;

/// How RGB pixels become one brightness value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Perceived brightness (Rec. 601). Good for any bright text.
    Luma,
    /// Darkest of R, G, B: high only for white-ish pixels, so white text
    /// separates from saturated backgrounds (e.g. digits on orange tiles).
    Min,
    /// Darker of R and G: high for white and yellow text, low for green
    /// foliage and brown map art behind the notification stack.
    MinRG,
}

/// 8-bit single-channel image. Serialized as rows of hex bytes so glyph
/// templates in calibration files stay compact and diffable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "GrayRows", try_from = "GrayRows")]
pub struct Gray {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Gray {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        assert_eq!(data.len(), width as usize * height as usize, "gray buffer size");
        Self { width, height, data }
    }

    /// Converts RGBA8 rows (`stride` bytes apart) using `channel`.
    pub fn from_rgba(width: u32, height: u32, stride: usize, rgba: &[u8], channel: Channel) -> Self {
        let mut data = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height as usize {
            let row = &rgba[y * stride..];
            for x in 0..width as usize {
                let [r, g, b] = [row[x * 4], row[x * 4 + 1], row[x * 4 + 2]].map(u32::from);
                let v = match channel {
                    Channel::Luma => (299 * r + 587 * g + 114 * b) / 1000,
                    Channel::Min => r.min(g).min(b),
                    Channel::MinRG => r.min(g),
                };
                data.push(v as u8);
            }
        }
        Self { width, height, data }
    }

    pub fn from_frame(frame: &Frame, channel: Channel) -> Self {
        Self::from_rgba(frame.width, frame.height, frame.stride, &frame.rgba, channel)
    }

    /// Ink pixels become `ink`, the rest `background`.
    pub fn from_bitmap(b: &Bitmap, ink: u8, background: u8) -> Self {
        let data = (0..b.height())
            .flat_map(|y| (0..b.width()).map(move |x| (x, y)))
            .map(|(x, y)| if b.get(x, y) { ink } else { background })
            .collect();
        Self { width: b.width(), height: b.height(), data }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.data[(y * self.width + x) as usize]
    }

    /// Sub-image, clamped to the image bounds.
    pub fn crop(&self, r: RectPx) -> Gray {
        let x0 = r.x.min(self.width);
        let y0 = r.y.min(self.height);
        let x1 = (r.x + r.w).min(self.width);
        let y1 = (r.y + r.h).min(self.height);
        let mut data = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
        for y in y0..y1 {
            let start = (y * self.width + x0) as usize;
            data.extend_from_slice(&self.data[start..start + (x1 - x0) as usize]);
        }
        Gray { width: x1 - x0, height: y1 - y0, data }
    }

    pub fn crop_columns(&self, cols: Range<u32>) -> Gray {
        self.crop(RectPx { x: cols.start, y: 0, w: cols.end - cols.start, h: self.height })
    }

    pub fn min(&self) -> u8 {
        self.data.iter().copied().min().unwrap_or(0)
    }

    /// Pixels brighter than `threshold` become ink.
    pub fn binarize(&self, threshold: u8) -> Bitmap {
        let mut b = Bitmap::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) > threshold {
                    b.set(x, y, true);
                }
            }
        }
        b
    }

    pub fn max(&self) -> u8 {
        self.data.iter().copied().max().unwrap_or(0)
    }

    /// Otsu's threshold: the split that best separates dark background from
    /// bright text. Works per crop, so it adapts to each region's background.
    /// When a range of thresholds separates equally well (an empty stretch of
    /// the histogram), the middle of that range is used.
    pub fn otsu_threshold(&self) -> u8 {
        self.otsu_threshold_above(0)
    }

    /// Otsu's threshold among pixels brighter than `floor` only. Running it
    /// above a first Otsu split separates the brightest class from a mid-tone
    /// one, e.g. white digits from the orange tiles behind them. Returns
    /// `floor` when those pixels are all one tone (nothing to split).
    pub fn otsu_threshold_above(&self, floor: u8) -> u8 {
        let mut hist = [0u64; 256];
        for &v in self.data.iter().filter(|&&v| v > floor || floor == 0) {
            hist[v as usize] += 1;
        }
        let total: f64 = hist.iter().sum::<u64>() as f64;
        let sum_all: f64 = hist.iter().enumerate().map(|(i, &n)| i as f64 * n as f64).sum();
        let (mut weight_bg, mut sum_bg) = (0.0, 0.0);
        let (mut best, mut best_end, mut best_var) = (0u8, 0u8, -1.0);
        for (t, &n) in hist.iter().enumerate() {
            weight_bg += n as f64;
            sum_bg += t as f64 * n as f64;
            let weight_fg = total - weight_bg;
            if weight_bg == 0.0 || weight_fg == 0.0 {
                continue;
            }
            let mean_bg = sum_bg / weight_bg;
            let mean_fg = (sum_all - sum_bg) / weight_fg;
            let var = weight_bg * weight_fg * (mean_bg - mean_fg).powi(2);
            if var > best_var * (1.0 + 1e-9) {
                best_var = var;
                best = t as u8;
                best_end = t as u8;
            } else if var >= best_var * (1.0 - 1e-9) && best_end as usize + 1 == t {
                best_end = t as u8;
            }
        }
        if best_var <= 0.0 {
            return floor;
        }
        best + (best_end - best) / 2
    }

    /// Mean brightness of the pixels no brighter than `t`, 0 if none.
    pub fn mean_at_most(&self, t: u8) -> u8 {
        let (sum, n) = self.data.iter().filter(|&&v| v <= t).fold((0u64, 0u64), |(s, n), &v| (s + u64::from(v), n + 1));
        sum.checked_div(n).map_or(0, |m| m as u8)
    }

    /// Mean brightness of the pixels that are ink in `mask`, 0 if none.
    pub fn mean_under(&self, mask: &Bitmap) -> u8 {
        let (mut sum, mut n) = (0u64, 0u64);
        for y in 0..self.height.min(mask.height()) {
            for x in 0..self.width.min(mask.width()) {
                if mask.get(x, y) {
                    sum += u64::from(self.get(x, y));
                    n += 1;
                }
            }
        }
        sum.checked_div(n).map_or(0, |mean| mean as u8)
    }
}

#[derive(Serialize, Deserialize)]
struct GrayRows {
    width: u32,
    height: u32,
    rows: Vec<String>,
}

impl From<Gray> for GrayRows {
    fn from(g: Gray) -> Self {
        let rows = g
            .data
            .chunks(g.width.max(1) as usize)
            .take(g.height as usize)
            .map(|row| row.iter().map(|v| format!("{v:02x}")).collect())
            .collect();
        Self { width: g.width, height: g.height, rows }
    }
}

impl TryFrom<GrayRows> for Gray {
    type Error = String;

    fn try_from(r: GrayRows) -> Result<Self, Self::Error> {
        let mut data = Vec::with_capacity(r.width as usize * r.height as usize);
        for row in &r.rows {
            if row.len() != r.width as usize * 2 {
                return Err(format!("gray row length does not match width {}", r.width));
            }
            for i in (0..row.len()).step_by(2) {
                data.push(u8::from_str_radix(&row[i..i + 2], 16).map_err(|e| e.to_string())?);
            }
        }
        if r.rows.len() != r.height as usize {
            return Err(format!("expected {} gray rows", r.height));
        }
        Ok(Gray { width: r.width, height: r.height, data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_crop_and_otsu() {
        // 4x2: dark background (30) with a bright stroke (220) and one mid pixel.
        let rgba: Vec<u8> = [30, 220, 220, 30, 30, 30, 120, 30]
            .iter()
            .flat_map(|&v| [v, v, v, 255])
            .collect();
        let g = Gray::from_rgba(4, 2, 16, &rgba, Channel::Luma);
        assert_eq!(g.get(1, 0), 220);
        let t = g.otsu_threshold();
        assert!((30..120).contains(&t), "threshold {t}");
        // Three tones in level-crop proportions: mostly background, then
        // orange tile, then white digit strokes.
        let data = [vec![10; 20], vec![100; 8], vec![230; 3]].concat();
        let three = Gray::new(31, 1, data);
        let first = three.otsu_threshold();
        let second = three.otsu_threshold_above(first);
        assert!((10..100).contains(&first) && (100..230).contains(&second), "{first} {second}");
        assert_eq!(Gray::new(2, 1, vec![10, 200]).otsu_threshold_above(100), 100);
        // Two clusters (30, 220): the middle of the empty stretch between them.
        let two = Gray::new(4, 1, vec![30, 30, 220, 220]);
        assert_eq!(two.otsu_threshold(), 30 + (219 - 30) / 2);
        assert_eq!(g.binarize(t).to_ascii(), vec![".##.", "..#."]);
        assert_eq!(g.binarize(150).to_ascii(), vec![".##.", "...."]);
        assert_eq!(g.crop(RectPx { x: 1, y: 0, w: 9, h: 1 }).width(), 3);
        assert_eq!(g.mean_under(&Bitmap::from_ascii(&[".##.", "..#."])), ((220 + 220 + 120) / 3) as u8);
    }

    #[test]
    fn serializes_as_hex_rows() {
        let g = Gray::new(2, 2, vec![0, 255, 16, 1]);
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(json, r#"{"width":2,"height":2,"rows":["00ff","1001"]}"#);
        assert_eq!(serde_json::from_str::<Gray>(&json).unwrap(), g);
        assert!(serde_json::from_str::<Gray>(r#"{"width":2,"height":1,"rows":["00"]}"#).is_err());
    }

    #[test]
    fn honours_stride_padding() {
        let mut rgba = vec![0u8; 2 * 12];
        rgba[12..16].copy_from_slice(&[200, 200, 200, 255]);
        let g = Gray::from_rgba(2, 2, 12, &rgba, Channel::Luma);
        assert_eq!((g.get(0, 0), g.get(0, 1)), (0, 200));
    }

    #[test]
    fn min_channel_rejects_saturated_colours() {
        let orange_then_white = [230, 120, 40, 255, 240, 240, 230, 255];
        let luma = Gray::from_rgba(2, 1, 8, &orange_then_white, Channel::Luma);
        let min = Gray::from_rgba(2, 1, 8, &orange_then_white, Channel::Min);
        assert!(luma.get(0, 0) > 130);
        assert_eq!((min.get(0, 0), min.get(1, 0)), (40, 230));
        // Yellow text vs green leaves.
        let yellow_then_green = [220, 215, 90, 255, 120, 200, 80, 255];
        let rg = Gray::from_rgba(2, 1, 8, &yellow_then_green, Channel::MinRG);
        assert_eq!((rg.get(0, 0), rg.get(1, 0)), (215, 120));
    }
}
