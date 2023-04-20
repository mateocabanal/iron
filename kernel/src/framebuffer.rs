use bootloader_api::{
    info::{FrameBufferInfo, PixelFormat},
    BootInfo,
};
use core::{fmt, ptr};
use font_constants::BACKUP_CHAR;
use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};

use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::framebuffer::font_constants::CHAR_RASTER_WIDTH;

use self::font_constants::CHAR_RASTER_HEIGHT;
pub static FBWRITER: OnceCell<Mutex<FrameBufferWriter>> = OnceCell::uninit();

/// Additional vertical space between lines
pub const LINE_SPACING: usize = 2;
/// Additional horizontal space between characters.
pub const LETTER_SPACING: usize = 0;

/// Padding from the border. Prevent that font is too close to border.
const BORDER_PADDING: usize = 1;

/// Constants for the usage of the [`noto_sans_mono_bitmap`] crate.
pub mod font_constants {
    use super::*;

    /// Height of each char raster. The font size is ~0.84% of this. Thus, this is the line height that
    /// enables multiple characters to be side-by-side and appear optically in one line in a natural way.
    pub const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;

    /// The width of each single symbol of the mono space font.
    pub const CHAR_RASTER_WIDTH: usize = get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);

    /// Backup character if a desired symbol is not available by the font.
    /// The '�' character requires the feature "unicode-specials".
    pub const BACKUP_CHAR: char = '�';

    pub const FONT_WEIGHT: FontWeight = FontWeight::Regular;
}

pub fn init_global_fb(boot_info: &'static BootInfo) {
    let boot_info = boot_info as *const BootInfo as *mut BootInfo;
    let boot_info_mut = unsafe { &mut *(boot_info) };
    let fb = boot_info_mut.framebuffer.as_mut().unwrap();
    let fb_info = fb.info().clone();
    FBWRITER.init_once(move || Mutex::new(FrameBufferWriter::new(fb.buffer_mut(), fb_info)));
}

/// Returns the raster of the given char or the raster of [`font_constants::BACKUP_CHAR`].
fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        get_raster(
            c,
            font_constants::FONT_WEIGHT,
            font_constants::CHAR_RASTER_HEIGHT,
        )
    }
    get(c).unwrap_or_else(|| get(BACKUP_CHAR).expect("Should get raster of backup char."))
}

#[derive(Debug, PartialEq)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Blue,
    White
}

impl Color {
    const fn get_color_rgb(&self) -> [u8; 3] {
        match self {
            Color::Red => [0xf4, 0x43, 0x36],
            Color::Yellow => [0xff, 0xc1, 0x07],
            Color::Green => [0x4c, 0xaf, 0x50],
            Color::Blue => [0x03, 0xa9, 0xf4],
            Color::White => [0xff, 0xff, 0xff],
        }
    }
    fn get_color_pixel(&self, pixel_format: PixelFormat, intensity: u8) -> [u8; 4] {
        let [r, g, b] = self
            .get_color_rgb()
            .map(|x| (x as u32 * intensity as u32 / 0xff) as u8);
        match pixel_format {
            PixelFormat::Rgb => [r, g, b, 0],
            PixelFormat::Bgr => [b, g, r, 0],
            PixelFormat::U8 => [intensity >> 4, 0, 0, 0],
            _ => panic!("Unknown pixel format: {:?}", pixel_format),
        }
    }
}

/// Allows logging text to a pixel-based framebuffer.
pub struct FrameBufferWriter {
    pub framebuffer: &'static mut [u8],
    pub info: FrameBufferInfo,
    pub x_pos: usize,
    pub y_pos: usize,
    pub color: Color
}

impl FrameBufferWriter {
    /// Creates a new logger that uses the given framebuffer.
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
            color: Color::White
        };
        logger.clear();
        logger
    }

    pub fn back_space(&mut self) {
        if self.x_pos > 0 {
            self.x_pos -= CHAR_RASTER_WIDTH;
        }

        if self.x_pos <= 0 {
            self.x_pos = self.width();
            if self.y_pos > 0 {
                self.y_pos -= 1;
            }
        }
        for y in 0..CHAR_RASTER_HEIGHT as usize {
            for x in 0..CHAR_RASTER_WIDTH {
                self.write_pixel(self.x_pos + x, self.y_pos + y, 0);
            }
        }
    }

    fn newline(&mut self) {
        self.y_pos += font_constants::CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        self.carriage_return()
    }

    fn carriage_return(&mut self) {
        self.x_pos = BORDER_PADDING;
    }

    /// Erases all text on the screen. Resets `self.x_pos` and `self.y_pos`.
    pub fn clear(&mut self) {
        self.x_pos = BORDER_PADDING;
        self.y_pos = BORDER_PADDING;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.width
    }

    fn height(&self) -> usize {
        self.info.height
    }

    /// Writes a single char to the framebuffer. Takes care of special control characters, such as
    /// newlines and carriage returns.
    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let new_xpos = self.x_pos + font_constants::CHAR_RASTER_WIDTH;
                if new_xpos >= self.width() {
                    self.newline();
                }
                let new_ypos =
                    self.y_pos + font_constants::CHAR_RASTER_HEIGHT.val() + BORDER_PADDING;
                if new_ypos >= self.height() {
                    self.clear();
                }
                self.write_rendered_char(get_char_raster(c));
            }
        }
    }

    /// Prints a rendered char into the framebuffer.
    /// Updates `self.x_pos`.
    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width() + LETTER_SPACING;
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.stride + x;
        let color = self.color.get_color_pixel(self.info.pixel_format, intensity);
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

unsafe impl Send for FrameBufferWriter {}
unsafe impl Sync for FrameBufferWriter {}

impl fmt::Write for FrameBufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| {
        if let Ok(fb) = FBWRITER.try_get() {
            fb.lock()
                .write_fmt(args)
                .expect("Printing to serial failed");
        }
    });
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::framebuffer::_print(format_args!($($arg)*))
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}
