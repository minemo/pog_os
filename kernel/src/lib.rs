#![no_std]
#![feature(abi_x86_interrupt)]

use framebuffer::{FrameBufferWriter, FBWRITER};
use spinning_top::Spinlock;

pub mod framebuffer;
pub mod interrupts;
pub mod serial;

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

  // Init interrupt descriptor table
  interrupts::init_idt();
}