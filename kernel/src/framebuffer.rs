use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use core::{borrow::BorrowMut, fmt, ptr};
use font_constants::BACKUP_CHAR;
use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};
use spin::{mutex::Mutex, once::Once};

const LINE_SPACING: usize = 2;
const LETTER_SPACING: usize = 0;
const BORDER_PADDING: usize = 1;

pub enum PixelValue {
    Mono(u8),
    Rgb(u8, u8, u8),
    Bgr(u8, u8, u8),
}

impl Into<u8> for PixelValue {
    fn into(self) -> u8 {
        match self {
            PixelValue::Mono(val) => return val,
            PixelValue::Rgb(r, g, b) => {
                return (r / 5) + ((g / 2) + (g / 5)) + (b / 14);
            }
            PixelValue::Bgr(b, g, r) => {
                return (r / 5) + ((g / 2) + (g / 5)) + (b / 14);
            }
        }
    }
}

impl PixelValue {
    pub fn to_mono(&self) -> PixelValue {
        match self {
            PixelValue::Mono(val) => return PixelValue::Mono(*val),
            PixelValue::Rgb(r, g, b) => {
                return PixelValue::Mono(((r << 1) + r + (g << 2) + b) >> 3);
            }
            PixelValue::Bgr(b, g, r) => {
                return PixelValue::Mono(((r << 1) + r + (g << 2) + b) >> 3);
            }
        }
    }

    pub fn to_rgb(&self) -> PixelValue {
        match self {
            PixelValue::Mono(val) => {
                return PixelValue::Rgb(*val, *val, *val);
            }
            PixelValue::Rgb(r, g, b) => return PixelValue::Rgb(*r, *g, *b),
            PixelValue::Bgr(b, g, r) => return PixelValue::Rgb(*r, *g, *b),
        }
    }

    pub fn to_bgr(&self) -> PixelValue {
        match self {
            PixelValue::Mono(val) => {
                return PixelValue::Bgr(*val, *val, *val);
            }
            PixelValue::Rgb(r, g, b) => return PixelValue::Bgr(*b, *g, *r),
            PixelValue::Bgr(b, g, r) => return PixelValue::Bgr(*b, *g, *r),
        }
    }

    pub fn to_array(&self, thresh: bool) -> [u8; 4] {
        match self {
            PixelValue::Mono(val) => {
                if thresh {
                    let t = if *val > 127 { 0xf } else { 0 };
                    return [t, t, t, 4];
                } else {
                    return [*val, *val, *val, 0];
                }
            }
            PixelValue::Rgb(r, g, b) => return [*r, *g, *b, 0],
            PixelValue::Bgr(b, g, r) => return [*b, *g, *r, 0],
        }
    }
}

/// Constants for the usage of the [`noto_sans_mono_bitmap`] crate.
mod font_constants {
    use super::*;
    /// Height of each char raster. The font size is ~0.84% of this. Thus, this is the line height that
    /// enables multiple characters to be side-by-side and appear optically in one line in a natural way.
    pub const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;
    /// The width of each single symbol of the mono space font.
    pub const CHAR_RASTER_WIDTH: usize = get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);
    /// Backup character if a desired symbol is not available by the font.
    pub const BACKUP_CHAR: char = '?';
    pub const FONT_WEIGHT: FontWeight = FontWeight::Regular;
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

pub struct FrameBufferWriter {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    pub x_pos: usize,
    pub y_pos: usize,
}

impl FrameBufferWriter {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
        };
        logger.clear();
        logger
    }

    fn newline(&mut self) {
        self.y_pos += font_constants::CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        self.carriage_return()
    }

    fn carriage_return(&mut self) {
        self.x_pos = BORDER_PADDING;
    }

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

    fn get_color(&mut self, value: PixelValue, thresholding: bool) -> [u8; 4] {
        match self.info.pixel_format {
            PixelFormat::Rgb => value.to_rgb().to_array(thresholding),
            PixelFormat::Bgr => value.to_bgr().to_array(thresholding),
            PixelFormat::U8 => value.to_mono().to_array(thresholding),
            other => {
                // set a supported (but invalid) pixel format before panicking to avoid a double
                // panic; it might not be readable though
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported in logger", other)
            }
        }
    }

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

    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, PixelValue::Mono(*byte));
            }
        }
        self.x_pos += rendered_char.width() + LETTER_SPACING;
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, val: PixelValue) {
        let pixel_offset = y * self.info.stride + x;
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let color = self.get_color(val, false);
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, val: PixelValue) {
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let color = self.get_color(val, false);
        for i in x..w {
            for j in y..h {
                let px_offset = j * self.info.stride + i;
                let byte_offset = px_offset * bytes_per_pixel;
                self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
                    .copy_from_slice(&color[..bytes_per_pixel]);
                let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
            }
        }
    }

    pub fn draw_image_gray(&mut self, x: usize, y: usize, w: usize, h: usize, img_data: &[u8]) {
        let bytes_per_pixel = self.info.bytes_per_pixel;
        for i in 0..w {
            let i_off = x + i;
            for j in 0..h {
                let j_off = y + j;
                let px_offset = j_off * self.info.stride + i_off;
                let byte_offset = px_offset * bytes_per_pixel;
                let color = self.get_color(PixelValue::Mono(img_data[j * w + i]), false);
                self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
                    .copy_from_slice(&color[..bytes_per_pixel]);
                let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
            }
        }
    }

    pub fn draw_image(&mut self, x: usize, y: usize, w: usize, h: usize, img_data: &[&[u8]]) {
        let bytes_per_pixel = self.info.bytes_per_pixel;
        for i in 0..w {
            let i_off = x + i;
            for j in 0..h {
                let j_off = y + j;
                let px_offset = j_off * self.info.stride + i_off;
                let byte_offset = px_offset * bytes_per_pixel;
                let color = self.get_color(
                    PixelValue::Rgb(
                        img_data[0][j * w + i],
                        img_data[1][j * w + i],
                        img_data[2][j * w + i],
                    ),
                    false,
                );
                self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
                    .copy_from_slice(&color[..bytes_per_pixel]);
                let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
            }
        }
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

pub static FBWRITER: Once<Mutex<FrameBufferWriter>> = Once::new();

pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
    let possible_fb = boot_info.borrow_mut().framebuffer.as_mut();
    match possible_fb {
        Some(fb) => {
            let info = fb.info();
            FBWRITER.call_once(|| Mutex::new(FrameBufferWriter::new(fb.buffer_mut(), info)));
        }
        None => panic!(),
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        FBWRITER.get().unwrap().lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::framebuffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
