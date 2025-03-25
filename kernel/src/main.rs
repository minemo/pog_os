#![no_std] // no rust standard lib
#![no_main] // no rust entry points
//! See test TODO below
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::ptr::NonNull;

use acpi::{AcpiHandler, AcpiTables, PhysicalMapping};
use bootloader_api::BootInfo;
use kernel::{
    allocator,
    ata::pio::asd,
    memory::{self, BootInfoFrameAllocator},
    println,
    task::{executor::Executor, keyboard, Task},
};
use x86_64::VirtAddr;

pub const VIRTUAL_OFFSET: usize = 0xF0000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

#[derive(Debug, Clone, Copy)]
struct TableHandler {}

impl AcpiHandler for TableHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let mapping = PhysicalMapping::<Self, T>::new(
            physical_address,
            NonNull::<T>::new((VIRTUAL_OFFSET + physical_address) as *mut T).unwrap(),
            size,
            size,
            *self,
        );
        mapping
    }

    fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {
        return;
    }
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub static BOOTLOADER_CFG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::FixedAddress(
        VIRTUAL_OFFSET as u64,
    )); //TODO make this accessible to other modules
        // config.kernel_stack_size = 100 * 1024;
    config
};

bootloader_api::entry_point!(kmain, config = &BOOTLOADER_CFG);

fn kmain(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    // Create a BootInfo pointer for the init function to use
    let bi_ptr: *mut BootInfo = &mut *boot_info;

    // ACPI parser
    let rdsp_addr = &boot_info.rsdp_addr.into_option().unwrap();
    let acpi = unsafe { AcpiTables::from_rsdp(TableHandler {}, *rdsp_addr as usize).unwrap() };

    // Init kernel
    kernel::init(unsafe { &mut *bi_ptr });

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.take().unwrap());
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_alloc = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_alloc).expect("heap initialization failed");

    // TODO DSDT AML Parser

    match acpi.platform_info().unwrap().interrupt_model {
        acpi::InterruptModel::Unknown => {}
        acpi::InterruptModel::Apic(apic) => {
            println!("[APIC] LAPIC found at 0x{:X}", apic.local_apic_address);
            println!("[APIC] IOAPICs:");
            for ioapic in apic.io_apics.iter() {
                println!("[APIC] {:?}", ioapic);
            }
        }
        _ => {}
    }

    match asd() {
        Some(x) => {
            println!("{:?}", x.get(0..64).unwrap());
        }
        None => {}
    }

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keys()));
    executor.run();
}

// handles panic (duh)
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // dump the info to serial for now
    println!("{}", info);
    kernel::hlt_loop();
}

//* TESTS
//TODO fix tests not working due to https://github.com/rust-osdev/bootloader/issues/366
#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    // serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }

    exit_qemu(QemuExitCode::Success);
}

// #[test_case]
// fn trivial_assertion() {
//     serial_print!("trivial assertion... ");
//     assert_eq!(1, 1);
//     println!("[ok]");
// }
