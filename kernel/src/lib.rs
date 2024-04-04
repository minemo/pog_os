#![no_std]
#![feature(abi_x86_interrupt)]

use framebuffer::{FrameBufferWriter, FBWRITER};
use spinning_top::Spinlock;
use x86_64::PhysAddr;

pub mod framebuffer;
pub mod interrupts;
pub mod serial;
pub mod gdt;
pub mod pic;
// pub mod apic;

pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
  
  // Set up initial framebuffer logic
  let possible_fb = boot_info.framebuffer.as_mut();
  match possible_fb {
      Some(fb) => {
          let info = fb.info();
          FBWRITER.get_or_init(||{
              Spinlock::new(FrameBufferWriter::new(fb.buffer_mut(), info))
          });
      },
      None => panic!(),
  }
  
  //TODO find APCI (not APIC) address using RSDP

  // Init gdt and idt
  gdt::init();
  interrupts::init_idt();
  unsafe { interrupts::PICS.lock().init()};
//   x86_64::instructions::interrupts::enable();
}

fn print_rsdp(boot_info: &'static mut bootloader_api::BootInfo) {
    match boot_info.rsdp_addr.take() {
        Some(a) => {serial_println!("{:#?}", PhysAddr::new(a))},
        _ => {}
    }
}