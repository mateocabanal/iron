use core::fmt::{self, Write};

use spin::Mutex;
use spin::Lazy;

pub struct SerialPort {
    port: uart_16550::SerialPort,
}

impl SerialPort {
    /// # Safety
    ///
    /// unsafe because this function must only be called once
    pub unsafe fn init() -> Self {
        let mut port = unsafe { uart_16550::SerialPort::new(0x3F8) };
        port.init();
        Self { port }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.port.write_str(s).unwrap();
        Ok(())
    }
}

pub static SERIAL1: Lazy<Mutex<SerialPort>> = Lazy::new(|| {
    let serial_port = unsafe { SerialPort::init() };
    Mutex::new(serial_port)
});

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
