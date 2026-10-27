//! Transfer caller-rasterized 1-bit images into the opaque diagnostic RGB canvas.
//!
//! No font lookup, shaping, glyph metrics, blending or text interpretation lives
//! here. The caller owns every bitmap and chooses its ink color and position.

use super::{Canvas, MAX_CANVAS_PIXELS, PaintError, Rgb};
use crate::MAX_VIEWPORT;

impl Canvas {
    /// Draw a tightly packed, MSB-first monochrome bitmap using opaque ink.
    ///
    /// Rows start at byte boundaries: each row occupies ceil(width / 8) bytes,
    /// with unused low bits of its final byte ignored. A one bit overwrites the
    /// corresponding pixel, while a zero bit leaves existing paint untouched.
    /// Only right/bottom viewport clipping is needed because positions are
    /// nonnegative integer pixels; masks wholly outside the viewport are valid.
    ///
    /// Every input and coordinate is checked before any pixel is modified.
    /// Neither this function nor its caller owns a font or text rasterizer.
    pub fn blit_mono(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        mask: &[u8],
        ink: Rgb,
    ) -> Result<(), PaintError> {
        if width == 0 || height == 0 || width > MAX_VIEWPORT || height > MAX_VIEWPORT {
            return Err(PaintError::InvalidBitmap);
        }
        x.checked_add(width).ok_or(PaintError::Overflow)?;
        y.checked_add(height).ok_or(PaintError::Overflow)?;

        let area = (width as usize)
            .checked_mul(height as usize)
            .ok_or(PaintError::Overflow)?;
        if area > MAX_CANVAS_PIXELS {
            return Err(PaintError::TooManyPixels);
        }
        let stride = (width as usize).div_ceil(8);
        let required_len = stride
            .checked_mul(height as usize)
            .ok_or(PaintError::Overflow)?;
        if mask.len() != required_len {
            return Err(PaintError::InvalidBitmap);
        }

        let rows = height.min(self.height.saturating_sub(y));
        let columns = width.min(self.width.saturating_sub(x));
        for row in 0..rows {
            let mask_row = row as usize * stride;
            let destination_row = (y + row) as usize * self.width as usize;
            for column in 0..columns {
                let byte = mask[mask_row + (column as usize / 8)];
                if byte & (0x80 >> (column % 8)) != 0 {
                    self.pixels[destination_row + (x + column) as usize] = ink;
                }
            }
        }
        Ok(())
    }
}
