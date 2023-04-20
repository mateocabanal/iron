const PROMPT: &'static str = "--> ";

use x86_64::instructions::interrupts::without_interrupts;

use crate::{
    framebuffer::{
        font_constants::{CHAR_RASTER_HEIGHT, CHAR_RASTER_WIDTH},
        FBWRITER,
    },
    print,
};

pub struct Shell {
    cursor_x: usize,
    cursor_y: usize,
}

impl Shell {
    pub fn init() -> Shell {
        without_interrupts(|| {
            let mut fb = FBWRITER.get().unwrap().lock();

            fb.clear();
        });

        print!("{PROMPT}");

        Shell {
            cursor_x: PROMPT.len() + CHAR_RASTER_WIDTH as usize,
            cursor_y: 0usize,
        }
    }

    pub fn update() {
        without_interrupts(|| {
            let mut fb = FBWRITER.get().unwrap().lock();
            let x_pos = fb.x_pos;
            let y_pos = fb.y_pos;
            let is_color_white = fb.color == crate::framebuffer::Color::White; 
            if is_color_white {
                fb.color = crate::framebuffer::Color::Blue;
            } else {
                fb.color = crate::framebuffer::Color::White;
            };        
            for y in 0..CHAR_RASTER_HEIGHT as usize {
                for x in 0..CHAR_RASTER_WIDTH {
                    fb.write_pixel(x_pos + x, y_pos + y, 127);
                }
            };
        });
    }
}
