#![no_std]

use x86_64::instructions::port::Port;
use x86_64::registers::model_specific::CetFlags;

const APIC_BASE_MSR: u32 = 0x1B;
const APIC_BASE_MSR_BSP: u32 = 0x100;
const APIC_BASE_MSR_ENABLE: u32 = 0x800;

//TODO Implement APIC for better IRQs
//TODO Use MADT to discover all APICs