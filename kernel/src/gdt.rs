use core::ptr::addr_of;

use generic_once_cell::Lazy;
use spinning_top::RawSpinlock;
use x86_64::instructions::tables::{load_tss, sgdt};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static TSS: Lazy<RawSpinlock, TaskStateSegment> = Lazy::new(|| {
    let mut tss = TaskStateSegment::new();
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
        const STCK_SIZE: usize = 5 * 4096;
        static mut STACK: [u8; STCK_SIZE] = [0; STCK_SIZE];

        let stack_start = VirtAddr::from_ptr(unsafe { addr_of!(STACK) });
        let stack_end = stack_start + STCK_SIZE.try_into().unwrap();
        stack_end
    };
    tss
});

static GDT: Lazy<RawSpinlock, (GlobalDescriptorTable, Selectors)> = Lazy::new(|| {
    // let prev_gdt_ptr = sgdt();
    // println!("Previous GDT: (Ptr: 0x{:x},Size:{})", prev_gdt_ptr.base.as_u64(),prev_gdt_ptr.limit);
    let mut gdt = GlobalDescriptorTable::new();
    let code_selector = gdt.append(Descriptor::kernel_code_segment());
    let data_selector = gdt.append(Descriptor::kernel_data_segment());
    let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
    (
        gdt,
        Selectors {
            code_selector,
            data_selector,
            tss_selector,
        },
    )
});

pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS, DS};

    GDT.0.load();
    unsafe {
        DS::set_reg(GDT.1.data_selector);
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
