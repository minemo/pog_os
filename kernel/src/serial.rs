use uart_16550::SerialPort;
use spinning_top::{RawSpinlock, Spinlock};
use generic_once_cell::Lazy;

pub static SERIAL1: Lazy<RawSpinlock,Spinlock<SerialPort>> = Lazy::new(||{
  let mut serial_port = unsafe { SerialPort::new(0x3F8) };
  serial_port.init();
  Spinlock::new(serial_port)
});


#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
  use core::fmt::Write;
  use x86_64::instructions::interrupts;
  interrupts::without_interrupts(|| {
    SERIAL1.lock().write_fmt(args).expect("error writing to serial");
  });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}