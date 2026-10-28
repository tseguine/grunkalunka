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
    /// Positions are document-space pixels, including on a scrolled Canvas.
    /// Clipping on all four viewport edges is permitted; wholly offscreen
    /// masks are valid and leave the canvas unchanged.
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

        let viewport_right = self
            .scroll_x
            .checked_add(self.width)
            .ok_or(PaintError::Overflow)?;
        let viewport_bottom = self
            .scroll_y
            .checked_add(self.height)
            .ok_or(PaintError::Overflow)?;
        let left = x.max(self.scroll_x);
        let top = y.max(self.scroll_y);
        let right = (x + width).min(viewport_right);
        let bottom = (y + height).min(viewport_bottom);
        if left >= right || top >= bottom {
            return Ok(());
        }
        for doc_y in top..bottom {
            let mask_row = (doc_y - y) as usize * stride;
            let destination_row = (doc_y - self.scroll_y) as usize * self.width as usize;
            for doc_x in left..right {
                let column = (doc_x - x) as usize;
                let byte = mask[mask_row + column / 8];
                if byte & (0x80 >> (column % 8)) != 0 {
                    self.pixels[destination_row + (doc_x - self.scroll_x) as usize] = ink;
                }
            }
        }
        Ok(())
    }
}
