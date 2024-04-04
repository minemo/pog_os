use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use generic_once_cell::Lazy;
use spinning_top::{RawSpinlock,Spinlock};
use crate::println;
use crate::gdt;
use crate::pic::ChainedPics;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Spinlock<ChainedPics> = Spinlock::new(unsafe {
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
});

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
}

impl InterruptIndex {
  fn as_u8(self) -> u8 {
    self as u8
  }

  fn as_usize(self) -> usize {
    usize::from(self.as_u8())
  }
}

static IDT: Lazy<RawSpinlock,InterruptDescriptorTable> = Lazy::new(|| {
  let mut idt = InterruptDescriptorTable::new();
  idt.breakpoint.set_handler_fn(breakpoint_handler);
  unsafe {
    idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
  }
  idt
});

pub fn init_idt() {
  IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
  println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
  panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}