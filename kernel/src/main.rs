#![no_std] // no rust standard lib
#![no_main] // no rust entry points
//! See test TODO below
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader_api::BootInfo;
use kernel::{println, serial_println};

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

pub static BOOTLOADER_CFG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    // config.kernel_stack_size = 100 * 1024;
    config
};

bootloader_api::entry_point!(kmain, config = &BOOTLOADER_CFG);

fn kmain(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    use kernel::memory::active_level_4_table;
    use x86_64::VirtAddr;

    // Create a BootInfo pointer for the init function to use
    let bi_ptr: *mut BootInfo = &mut *boot_info;

    // Init kernel
    kernel::init(unsafe { &mut *bi_ptr });

    println!("Hello World{}", "!");

    match boot_info.physical_memory_offset.into_option() {
        Some(x) => {
            let phys_mem_offset = VirtAddr::new(x);
            let l4_table = unsafe { active_level_4_table(phys_mem_offset) };

            for (i, entry) in l4_table.iter().enumerate() {
                if !entry.is_unused() {
                    serial_println!("L4 Entry {}: {:?}", i, entry);
                }
            }
        }
        None => panic!("No memory mapping found!"),
    }


    kernel::hlt_loop();
}

// handles panic (duh)
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // dump the info to serial for now
    println!("{}", info);
    kernel::hlt_loop();
}

//* TESTS
//TODO fix tests not working due to https://github.com/rust-osdev/bootloader/issues/366
#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }

    exit_qemu(QemuExitCode::Success);
}

#[test_case]
fn trivial_assertion() {
    serial_print!("trivial assertion... ");
    assert_eq!(1, 1);
    println!("[ok]");
}
