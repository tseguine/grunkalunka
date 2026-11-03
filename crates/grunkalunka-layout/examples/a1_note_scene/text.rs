//! Private A1 text service: a locally defined 5x7 printable-ASCII bitmap font.
//! Other Unicode scalars, including controls, get a visible replacement box.
//! No shaping, grapheme segmentation, word breaking or ambient font access.
//! Original labels remain in A1FRAME1; this module only produces bounded ink.

use grunkalunka_layout::paint::{Canvas, PaintError, Rgb};
use grunkalunka_layout::{MAX_VIEWPORT, Rect};

const MAX_TEXT_BYTES: usize = 4096;
const INSET: u32 = 2;
const GLYPH_WIDTH: u32 = 5;
const GLYPH_HEIGHT: u32 = 7;
const ADVANCE: u32 = 6;
const LINE_ADVANCE: u32 = 9;
const REPLACEMENT: [u8; 7] = [31, 17, 21, 17, 21, 17, 31];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextError {
    TextTooLarge,
    InvalidWidth,
    Geometry,
    Paint(PaintError),
}

/// Measured glyph rows are retained so painting uses exactly the same plan.
/// Fixed-cell wrapping preserves every scalar, including spaces.
#[derive(Debug)]
pub struct TextPlan {
    width: u32,
    columns: u32,
    height: u32,
    glyphs: Vec<[u8; 7]>,
}

impl TextPlan {
    pub fn new(text: &str, width: u32) -> Result<Self, TextError> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(TextError::TextTooLarge);
        }
        if !(INSET * 2 + GLYPH_WIDTH..=MAX_VIEWPORT).contains(&width) {
            return Err(TextError::InvalidWidth);
        }
        let columns = (width - INSET * 2 + ADVANCE - GLYPH_WIDTH) / ADVANCE;
        let glyphs: Vec<_> = text
            .chars()
            .map(|scalar| {
                if (' '..='~').contains(&scalar) {
                    ASCII[scalar as usize - ' ' as usize]
                } else {
                    REPLACEMENT
                }
            })
            .collect();
        // At most 4096 UTF-8 bytes implies at most 4096 scalar cells.
        let rows = (glyphs.len() as u32).div_ceil(columns);
        let height = if rows == 0 {
            0
        } else {
            INSET * 2 + (rows - 1) * LINE_ADVANCE + GLYPH_HEIGHT
        };
        Ok(Self {
            width,
            columns,
            height,
            glyphs,
        })
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Draw inside the measured content rectangle; Canvas clips to its viewport.
    /// A1 uses non-overlapping normal-flow rows without ancestor clipping.
    /// Validate the complete destination before modifying any pixel.
    pub fn paint(&self, canvas: &mut Canvas, content: Rect, ink: Rgb) -> Result<(), TextError> {
        if content.width != self.width
            || content.height < self.height
            || content.right().is_none()
            || content.bottom().is_none()
        {
            return Err(TextError::Geometry);
        }
        for (index, rows) in self.glyphs.iter().enumerate() {
            let index = index as u32;
            let x = content.x + INSET + (index % self.columns) * ADVANCE;
            let y = content.y + INSET + (index / self.columns) * LINE_ADVANCE;
            let mask = rows.map(|row| row << 3);
            canvas
                .blit_mono(x, y, GLYPH_WIDTH, GLYPH_HEIGHT, &mask, ink)
                .map_err(TextError::Paint)?;
        }
        Ok(())
    }
}

// Original geometric pixel patterns, indexed by U+0020 through U+007E.
// Five low bits describe each row from left to right; painting shifts them
// into the existing MSB-first bitmap interface. Space deliberately has no ink.
const ASCII: [[u8; 7]; 95] = [
    [0, 0, 0, 0, 0, 0, 0],        // U+0020
    [4, 4, 4, 4, 4, 0, 4],        // U+0021
    [10, 10, 10, 0, 0, 0, 0],     // U+0022
    [10, 31, 10, 10, 31, 10, 0],  // U+0023
    [4, 15, 20, 14, 5, 30, 4],    // U+0024
    [24, 25, 2, 4, 8, 19, 3],     // U+0025
    [12, 18, 20, 8, 21, 18, 13],  // U+0026
    [4, 4, 8, 0, 0, 0, 0],        // U+0027
    [2, 4, 8, 8, 8, 4, 2],        // U+0028
    [8, 4, 2, 2, 2, 4, 8],        // U+0029
    [0, 4, 21, 14, 21, 4, 0],     // U+002A
    [0, 4, 4, 31, 4, 4, 0],       // U+002B
    [0, 0, 0, 0, 4, 4, 8],        // U+002C
    [0, 0, 0, 31, 0, 0, 0],       // U+002D
    [0, 0, 0, 0, 0, 12, 12],      // U+002E
    [1, 2, 2, 4, 8, 8, 16],       // U+002F
    [14, 17, 19, 21, 25, 17, 14], // U+0030
    [4, 12, 4, 4, 4, 4, 14],      // U+0031
    [14, 17, 1, 2, 4, 8, 31],     // U+0032
    [30, 1, 1, 14, 1, 1, 30],     // U+0033
    [2, 6, 10, 18, 31, 2, 2],     // U+0034
    [31, 16, 16, 30, 1, 1, 30],   // U+0035
    [6, 8, 16, 30, 17, 17, 14],   // U+0036
    [31, 1, 2, 4, 8, 8, 8],       // U+0037
    [14, 17, 17, 14, 17, 17, 14], // U+0038
    [14, 17, 17, 15, 1, 2, 12],   // U+0039
    [0, 12, 12, 0, 12, 12, 0],    // U+003A
    [0, 12, 12, 0, 4, 4, 8],      // U+003B
    [2, 4, 8, 16, 8, 4, 2],       // U+003C
    [0, 0, 31, 0, 31, 0, 0],      // U+003D
    [8, 4, 2, 1, 2, 4, 8],        // U+003E
    [14, 17, 1, 2, 4, 0, 4],      // U+003F
    [14, 17, 23, 21, 23, 16, 14], // U+0040
    [14, 17, 17, 31, 17, 17, 17], // U+0041
    [30, 17, 17, 30, 17, 17, 30], // U+0042
    [14, 17, 16, 16, 16, 17, 14], // U+0043
    [30, 17, 17, 17, 17, 17, 30], // U+0044
    [31, 16, 16, 30, 16, 16, 31], // U+0045
    [31, 16, 16, 30, 16, 16, 16], // U+0046
    [14, 17, 16, 23, 17, 17, 15], // U+0047
    [17, 17, 17, 31, 17, 17, 17], // U+0048
    [31, 4, 4, 4, 4, 4, 31],      // U+0049
    [7, 2, 2, 2, 18, 18, 12],     // U+004A
    [17, 18, 20, 24, 20, 18, 17], // U+004B
    [16, 16, 16, 16, 16, 16, 31], // U+004C
    [17, 27, 21, 21, 17, 17, 17], // U+004D
    [17, 25, 21, 19, 17, 17, 17], // U+004E
    [14, 17, 17, 17, 17, 17, 14], // U+004F
    [30, 17, 17, 30, 16, 16, 16], // U+0050
    [14, 17, 17, 17, 21, 18, 13], // U+0051
    [30, 17, 17, 30, 20, 18, 17], // U+0052
    [15, 16, 16, 14, 1, 1, 30],   // U+0053
    [31, 4, 4, 4, 4, 4, 4],       // U+0054
    [17, 17, 17, 17, 17, 17, 14], // U+0055
    [17, 17, 17, 17, 17, 10, 4],  // U+0056
    [17, 17, 17, 21, 21, 27, 17], // U+0057
    [17, 17, 10, 4, 10, 17, 17],  // U+0058
    [17, 17, 10, 4, 4, 4, 4],     // U+0059
    [31, 1, 2, 4, 8, 16, 31],     // U+005A
    [14, 8, 8, 8, 8, 8, 14],      // U+005B
    [16, 8, 8, 4, 2, 2, 1],       // U+005C
    [14, 2, 2, 2, 2, 2, 14],      // U+005D
    [4, 10, 17, 0, 0, 0, 0],      // U+005E
    [0, 0, 0, 0, 0, 0, 31],       // U+005F
    [8, 4, 2, 0, 0, 0, 0],        // U+0060
    [0, 0, 14, 1, 15, 17, 15],    // U+0061
    [16, 16, 30, 17, 17, 17, 30], // U+0062
    [0, 0, 15, 16, 16, 16, 15],   // U+0063
    [1, 1, 15, 17, 17, 17, 15],   // U+0064
    [0, 0, 14, 17, 31, 16, 14],   // U+0065
    [6, 9, 8, 28, 8, 8, 8],       // U+0066
    [0, 0, 15, 17, 15, 1, 14],    // U+0067
    [16, 16, 30, 17, 17, 17, 17], // U+0068
    [4, 0, 12, 4, 4, 4, 14],      // U+0069
    [2, 0, 6, 2, 2, 18, 12],      // U+006A
    [16, 16, 18, 20, 24, 20, 18], // U+006B
    [12, 4, 4, 4, 4, 4, 14],      // U+006C
    [0, 0, 26, 21, 21, 21, 21],   // U+006D
    [0, 0, 30, 17, 17, 17, 17],   // U+006E
    [0, 0, 14, 17, 17, 17, 14],   // U+006F
    [0, 0, 30, 17, 30, 16, 16],   // U+0070
    [0, 0, 15, 17, 15, 1, 1],     // U+0071
    [0, 0, 22, 25, 16, 16, 16],   // U+0072
    [0, 0, 15, 16, 14, 1, 30],    // U+0073
    [8, 8, 28, 8, 8, 9, 6],       // U+0074
    [0, 0, 17, 17, 17, 19, 13],   // U+0075
    [0, 0, 17, 17, 17, 10, 4],    // U+0076
    [0, 0, 17, 17, 21, 21, 10],   // U+0077
    [0, 0, 17, 10, 4, 10, 17],    // U+0078
    [0, 0, 17, 17, 15, 1, 14],    // U+0079
    [0, 0, 31, 2, 4, 8, 31],      // U+007A
    [3, 4, 4, 8, 4, 4, 3],        // U+007B
    [4, 4, 4, 4, 4, 4, 4],        // U+007C
    [24, 4, 4, 2, 4, 4, 24],      // U+007D
    [0, 0, 8, 21, 2, 0, 0],       // U+007E
];
