use conquer_once::spin::OnceCell;

use crate::{println, serial_println};

pub fn init_logger() {
    let logger = LOGGER.get_or_init(move || LockedLogger::new(true, true));

    log::set_logger(logger).expect("logger is already set");
    log::set_max_level(log::LevelFilter::Trace);
}

/// The global logger instance used for the `log` crate.
pub static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();

/// A logger instance protected by a spinlock.
pub struct LockedLogger {
    framebuffer: bool,
    serial: bool,
}

impl LockedLogger {
    /// Create a new instance that logs to the given framebuffer.
    pub fn new(frame_buffer_logger_status: bool, serial_logger_status: bool) -> Self {
        let framebuffer = frame_buffer_logger_status;
        let serial = serial_logger_status;

        LockedLogger {
            framebuffer,
            serial,
        }
    }
}

impl log::Log for LockedLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.framebuffer {
            println!("{:5}: {}", record.level(), record.args());
        }
        if self.serial {
            serial_println!("{:5}: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
