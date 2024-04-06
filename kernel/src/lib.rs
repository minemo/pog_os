#![no_std]
#![feature(abi_x86_interrupt)]

use framebuffer::{FrameBufferWriter, FBWRITER};
use spinning_top::Spinlock;

pub mod framebuffer;
pub mod interrupts;
pub mod serial;
pub mod gdt;
pub mod pic;
// pub mod apic;

pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
  // Set up initial framebuffer logic
  framebuffer::init(boot_info);


  //TODO find APCI (not APIC) address using RSDP
  
  // Init gdt and idt
  gdt::init();
  interrupts::init_idt();
  unsafe { interrupts::PICS.lock().init()};
  // x86_64::instructions::interrupts::enable();
}