#![no_std]
#![feature(abi_x86_interrupt)]

pub mod apic;
pub mod framebuffer;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
    // Set up initial framebuffer logic
    framebuffer::init(boot_info);

    //TODO find APCI (not APIC) address using RSDP

    // Init gdt and idt
    gdt::init();
    interrupts::init_idt();
    unsafe {
        interrupts::PICS.lock().initialize();
    };
    x86_64::instructions::interrupts::enable();
}

#[cfg(test)]
use bootloader_api::{entry_point, BootInfo};

#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(_boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init(boot_info);
}
