//! Bounded glyph/line statistics. These are layout-risk signals, not OCR.
use pdf_extract::{ColorSpace, MediaBox, OutputDev, OutputError, Path, PathOp, Transform};
use std::collections::BTreeMap;
#[derive(Default)]
pub(super) struct PageGeometry {
    pub text: String,
    pub width: f64,
    rows: BTreeMap<i32, Vec<(f64, f64)>>,
    horizontal: usize,
    vertical: usize,
}
impl PageGeometry {
    pub fn columns(&self) -> usize {
        if self.width <= 0.0 {
            return 1;
        }
        let split_rows = self
            .rows
            .values()
            .filter(|row| {
                if row.len() < 16 {
                    return false;
                }
                let mut row = (*row).clone();
                row.sort_by(|a, b| a.0.total_cmp(&b.0));
                row.windows(2).enumerate().any(|(i, pair)| {
                    i >= 7 && i + 8 < row.len() && pair[1].0 - pair[0].1 > self.width * 0.12
                })
            })
            .count();
        if split_rows >= 3 { 2 } else { 1 }
    }
    pub fn has_grid(&self) -> bool {
        self.horizontal >= 4 && self.vertical >= 3
    }
}
impl OutputDev for PageGeometry {
    fn begin_page(
        &mut self,
        _: u32,
        media: &MediaBox,
        _: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        self.width = media.urx - media.llx;
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _: f64,
        _: f64,
        text: &str,
    ) -> Result<(), OutputError> {
        if self.text.len() + text.len() > 64 * 1024 {
            return Err(std::fmt::Error.into());
        }
        self.text.push_str(text);
        if !text.trim().is_empty() && trm.m31.is_finite() && trm.m32.is_finite() {
            let row = (trm.m32 / 3.0).round() as i32;
            self.rows
                .entry(row)
                .or_default()
                .push((trm.m31, trm.m31 + (width * trm.m11).abs()));
        }
        Ok(())
    }
    fn stroke(
        &mut self,
        _: &Transform,
        _: &ColorSpace,
        _: &[f64],
        path: &Path,
    ) -> Result<(), OutputError> {
        let mut position = None;
        for op in path.ops.iter().take(10_000) {
            match op {
                PathOp::MoveTo(x, y) => position = Some((*x, *y)),
                PathOp::LineTo(x, y) => {
                    if let Some((px, py)) = position {
                        if (py - y).abs() < 0.5 && (px - x).abs() > 10.0 {
                            self.horizontal += 1;
                        }
                        if (px - x).abs() < 0.5 && (py - y).abs() > 10.0 {
                            self.vertical += 1;
                        }
                    }
                    position = Some((*x, *y));
                }
                PathOp::Rect(_, _, w, h) if *w > 10.0 && *h > 10.0 => {
                    self.horizontal += 2;
                    self.vertical += 2;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
