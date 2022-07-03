//! Emergency debugging console

use super::font::*;
use crate::{drawing::*, system::System};
use core::fmt;

pub struct EmConsole {
    x: usize,
    y: usize,
    fg_color: Color,
    bg_color: Color,
    font: &'static FixedFontDriver<'static>,
}

impl EmConsole {
    const DEFAULT_FG_COLOR: Color = Color::LIGHT_GRAY;
    const DEFAULT_BG_COLOR: Color = Color::from_rgb(0x000000);
    const PADDING: isize = 8;

    #[inline]
    pub const fn new(font: &'static FixedFontDriver<'static>) -> Self {
        Self {
            x: 0,
            y: 0,
            fg_color: Self::DEFAULT_FG_COLOR,
            bg_color: Self::DEFAULT_BG_COLOR,
            font,
        }
    }

    fn dims(&self) -> (isize, isize) {
        let font = self.font;
        let font_size = Size::new(font.width(), font.line_height());
        let bitmap = System::main_screen();
        let cols = (bitmap.width() as isize - Self::PADDING * 2) / font_size.width();
        let rows = (bitmap.height() as isize - Self::PADDING * 2) / font_size.height();
        (cols, rows)
    }

    pub fn write_char(&mut self, c: char) {
        let font = self.font;
        let font_size = Size::new(font.width(), font.line_height());
        let mut bitmap = System::main_screen();
        let bitmap = &mut bitmap;

        // check bounds
        let (cols, rows) = self.dims();
        let cols = cols as usize;
        let rows = rows as usize;
        if self.x >= cols {
            self.x = 0;
            self.y += 1;
        }
        if self.y >= rows {
            self.y = rows - 1;
            let sh = font_size.height() * self.y as isize;
            let mut rect = bitmap.bounds();
            rect.origin.y += font_size.height() + Self::PADDING;
            rect.size.height = sh;
            bitmap.blt_itself(Point::new(0, Self::PADDING), rect);
            bitmap.fill_rect(
                Rect::new(0, sh + Self::PADDING, rect.width(), font_size.height()),
                self.bg_color.into(),
            );
        }

        match c {
            '\x08' => {
                if self.x > 0 {
                    self.x -= 1;
                }
            }
            '\r' => {
                self.x = 0;
            }
            '\n' => {
                self.x = 0;
                self.y += 1;
            }
            _ => {
                let origin = Point::new(
                    self.x as isize * font_size.width + Self::PADDING,
                    self.y as isize * font_size.height + Self::PADDING,
                );
                bitmap.fill_rect(
                    Rect {
                        origin,
                        size: font_size,
                    },
                    self.bg_color.into(),
                );
                font.draw_char(c, bitmap, origin, font.base_height(), self.fg_color.into());

                self.x += 1;
            }
        }
    }
}

impl fmt::Write for EmConsole {
    #[inline]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

// impl TtyWrite for EmConsole {
//     fn reset(&mut self) -> Result<(), super::tty::TtyError> {
//         let mut bitmap = System::main_screen();
//         let bitmap = &mut bitmap;
//         bitmap.fill_rect(bitmap.bounds(), self.bg_color.into());
//         Ok(())
//     }

//     fn dims(&self) -> (isize, isize) {
//         let font = self.font;
//         let font_size = Size::new(font.width(), font.line_height());
//         let bitmap = System::main_screen();
//         let cols = (bitmap.width() as isize - Self::PADDING * 2) / font_size.width();
//         let rows = (bitmap.height() as isize - Self::PADDING * 2) / font_size.height();
//         (cols, rows)
//     }

//     fn cursor_position(&self) -> (isize, isize) {
//         (self.x as isize, self.y as isize)
//     }

//     fn set_cursor_position(&mut self, x: isize, y: isize) {
//         self.x = x as usize;
//         self.y = y as usize;
//     }

//     fn is_cursor_enabled(&self) -> bool {
//         false
//     }

//     fn set_cursor_enabled(&mut self, _enabled: bool) -> bool {
//         false
//     }

//     fn set_attribute(&mut self, attribute: u8) {
//         if attribute > 0 {
//             self.fg_color = IndexedColor(attribute & 0x0F).into();
//             let bg_color = attribute >> 4;
//             if bg_color > 0 {
//                 self.bg_color = IndexedColor(bg_color).into();
//             } else {
//                 self.bg_color = Color::from_rgb(0x000000);
//             }
//         } else {
//             self.fg_color = Self::DEFAULT_FG_COLOR;
//             self.bg_color = Self::DEFAULT_BG_COLOR;
//         }
//     }
// }
