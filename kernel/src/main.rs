#![no_std] // no rust standard lib
#![no_main] // no rust entry points
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::fmt::{Write, self};

use crate::framebuffer::FrameBufferWriter;

mod framebuffer;

mod serial;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

// create a global logger to make stuff easier 
static mut LOGGER: Option<FrameBufferWriter> = None;

pub static BOOTLOADER_CFG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

bootloader_api::entry_point!(kmain, config = &BOOTLOADER_CFG);

fn kmain(boot_info: &'static mut bootloader_api::BootInfo) -> ! {

    // Set up initial framebuffer logic
    let possible_fb = boot_info.framebuffer.as_mut();
    match possible_fb {
        Some(fb) => {
            let info = fb.info();
            unsafe {
                LOGGER = Some(FrameBufferWriter::new(fb.buffer_mut(), info));
            }
        },
        None => panic!(),
    }
    // if framebuffer setup failed, we won't even reach here

    println!("Hello World{}", "!");
        
    loop {}
    
}

fn get_logger() -> &'static mut FrameBufferWriter {
    unsafe {
        LOGGER.as_mut().unwrap()
    }
}

// handles panic (duh)
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    get_logger().write_fmt(args).unwrap();
}


//* TESTS
//TODO fix tests not working due to https://github.com/rust-osdev/bootloader/issues/366
pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }

    exit_qemu(QemuExitCode::Success);
}

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion... ");
    assert_eq!(1, 1);
    println!("[ok]");
}
