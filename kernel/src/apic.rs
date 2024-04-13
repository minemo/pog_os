#![no_std]

use x86_64::instructions::port::Port;
use x86_64::registers::model_specific::{CetFlags, Msr};

const APIC_BASE_MSR: u32 = 0x1B;
const APIC_BASE_MSR_BSP: u32 = 0x100;
const APIC_BASE_MSR_ENABLE: u32 = 0x800;

//TODO Implement APIC for better IRQs
//TODO Use MADT to discover all APICs

pub struct Apic {
  base: u64,
  register: Msr,
  svi: Port<u32>
}

impl Apic {
    pub unsafe fn new() -> Apic {
      let apic_msr = Msr::new(APIC_BASE_MSR);
      Apic { 
        base: apic_msr.read() & 0xfffff000, 
        register: apic_msr,
        svi: Port::new(0xF0),
      }
    }

    pub unsafe fn init(&mut self) {
      self.set_apic_base(self.base);
      let svi_val = self.svi.read();
      self.svi.write(svi_val | 0x100);
    }

    unsafe fn set_apic_base(&mut self, base: u64) {
      let val = (base & 0xfffff0000);
      self.register.write(val);
    }
}