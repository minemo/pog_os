use crate::{gdt, hlt_loop, println};
use spin::{lazy::Lazy,mutex::Mutex};
use x2apic::{
    ioapic::{IoApic, IrqFlags, IrqMode, RedirectionTableEntry},
    lapic::{LocalApic, LocalApicBuilder},
};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const LAPIC_PHYS_ADDR: u64 = 0xFEE00000;
pub const LAPIC_VIRT_ADDR: u64 = 0xF0000000 + LAPIC_PHYS_ADDR;
pub const IOAPIC_PHYS_ADDR: u64 = 0xFEC00000;
pub const IOAPIC_VIRT_ADDR: u64 = 0xF0000000 + IOAPIC_PHYS_ADDR;

pub const INTERRUPT_BASE: u8 = 0x20;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = INTERRUPT_BASE,
    Keyboard,
    Mouse = INTERRUPT_BASE + 12,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        self as usize
    }
}

pub static LAPIC: Lazy<Mutex<LocalApic>> = Lazy::new(|| {
    let lapic = LocalApicBuilder::new()
        .timer_vector(InterruptIndex::Timer.as_usize())
        .error_vector(0x7)
        .spurious_vector(0xFF)
        .set_xapic_base(LAPIC_VIRT_ADDR)
        .build()
        .unwrap_or_else(|err| panic!("{}", err));
    Mutex::new(lapic)
});

pub static IOAPIC: Lazy<Mutex<IoApic>> = Lazy::new(|| unsafe {
    let ioapic = IoApic::new(IOAPIC_VIRT_ADDR);
    Mutex::new(ioapic)
});

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
    idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
    idt[InterruptIndex::Mouse.as_u8()].set_handler_fn(mouse_interrupt_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt
});

pub fn init_idt() {
    IDT.load();
}

pub unsafe fn redirect_interrupt(
    irq_idx: InterruptIndex,
    table_idx: u8,
    dest: u8,
    flags: IrqFlags,
) {
    let mut new_entry = RedirectionTableEntry::default();
    new_entry.set_mode(IrqMode::Fixed);
    new_entry.set_flags(flags);
    new_entry.set_dest(dest);
    new_entry.set_vector(irq_idx.as_u8());
    IOAPIC.lock().set_table_entry(table_idx, new_entry);
    IOAPIC.lock().enable_irq(table_idx);
}

pub unsafe fn init_apic(ioapic_offset: u8) {
    LAPIC.lock().enable();
    IOAPIC.lock().init(ioapic_offset);

    redirect_interrupt(InterruptIndex::Keyboard, 1, 0, IrqFlags::empty());
    redirect_interrupt(InterruptIndex::Mouse, 2, 0, IrqFlags::MASKED);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\n{:#?}\nERROR CODE:{:#?}",
        stack_frame, error_code
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // print!(".");

    unsafe { LAPIC.lock().end_of_interrupt() }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe { LAPIC.lock().end_of_interrupt() }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // print!("#");
    //TODO implement mouse input
    unsafe { LAPIC.lock().end_of_interrupt() }
}
