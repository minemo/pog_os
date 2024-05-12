#![no_std]
#![feature(abi_x86_interrupt)]

use core::borrow::{Borrow, BorrowMut};

pub mod framebuffer;
pub mod interrupts;
pub mod serial;
pub mod gdt;
pub mod apic;
pub mod alloc;

pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
  // Set up initial framebuffer logic
  framebuffer::init(boot_info);

  //TODO find APCI (not APIC) address using RSDP
  
  // Init gdt and idt
  gdt::init();
  interrupts::init_idt();
  unsafe { 
    serial_println!("{:#?}", interrupts::PICS.lock().read_masks());
    interrupts::PICS.lock().initialize();
  };
  x86_64::instructions::interrupts::enable();
}