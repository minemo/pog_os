#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod allocator;
pub mod framebuffer;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod ata;
pub mod serial;
pub mod task;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
    // Set up initial framebuffer logic
    framebuffer::init(boot_info);

    // Init gdt and idt
    gdt::init();
    interrupts::init_idt();
    unsafe {
        interrupts::init_apic(0);
    };
    x86_64::instructions::interrupts::enable();
}

#[cfg(test)]
use bootloader_api::{entry_point, BootInfo};

#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    init(_boot_info);
    hlt_loop();
}
