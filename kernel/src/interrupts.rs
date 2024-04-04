use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use generic_once_cell::Lazy;
use spinning_top::RawSpinlock;
use crate::println;

static IDT: Lazy<RawSpinlock,InterruptDescriptorTable> = Lazy::new(|| {
  let mut idt = InterruptDescriptorTable::new();
  idt.breakpoint.set_handler_fn(breakpoint_handler);
  idt
});

pub fn init_idt() {
  IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
  println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}