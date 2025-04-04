use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;
use x86_64::instructions::port::{
    Port, PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess,
};

use crate::println;

bitflags! {
    struct StatusFlags: u8 {
        const ERR = 0b00000001;
        const IDX = 0b00000010;
        const CORR = 0b00000100;
        const DRQ = 0b00001000;
        const SRV = 0b00010000;
        const DF = 0b00100000;
        const RDY = 0b01000000;
        const BSY = 0b10000000;
    }
}

pub struct ATAIOBus {
    data: PortGeneric<u16, ReadWriteAccess>,
    error: PortGeneric<u16, ReadOnlyAccess>,
    features: PortGeneric<u16, WriteOnlyAccess>,
    sector_count: PortGeneric<u16, ReadWriteAccess>,
    sector_num: PortGeneric<u16, ReadWriteAccess>,
    cylinder_low: PortGeneric<u16, ReadWriteAccess>,
    cylinder_high: PortGeneric<u16, ReadWriteAccess>,
    head: PortGeneric<u8, ReadWriteAccess>,
    status: PortGeneric<u8, ReadOnlyAccess>,
    command: PortGeneric<u8, WriteOnlyAccess>,
}

pub struct ATACtrlBus {
    alternate_status: PortGeneric<u8, ReadOnlyAccess>,
    device_control: PortGeneric<u8, WriteOnlyAccess>,
    drive_addr: PortGeneric<u8, ReadOnlyAccess>,
}

pub struct PIOBus {
    io: ATAIOBus,
    ctrl: ATACtrlBus,
    base: u16,
    is_secondary: bool,
    info: Option<[u16; 256]>,
}

impl ATAIOBus {
    pub fn new(bus_base: u16) -> Self {
        ATAIOBus {
            data: PortGeneric::new(bus_base),
            error: PortGeneric::new(bus_base + 1),
            features: PortGeneric::new(bus_base + 1),
            sector_count: PortGeneric::new(bus_base + 2),
            sector_num: PortGeneric::new(bus_base + 3),
            cylinder_low: PortGeneric::new(bus_base + 4),
            cylinder_high: PortGeneric::new(bus_base + 5),
            head: PortGeneric::new(bus_base + 6),
            status: PortGeneric::new(bus_base + 7),
            command: PortGeneric::new(bus_base + 7),
        }
    }

    /// Checks if [`ATAIOBus`] is in Float state.
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn check_float(&mut self) -> bool {
        self.status.read() == 0xff
    }

    /// Send Identify command to Bus and return info if successful.
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn identify(&mut self, secondary: bool) -> Option<[u16; 256]> {
        self.head.write(0xa0 + ((secondary as u8) << 4));
        self.sector_count.write(0);
        self.sector_num.write(0);
        self.cylinder_low.write(0);
        self.cylinder_high.write(0);
        self.command.write(0xec);
        let mut flags: u8 = self.status.read();
        println!("{:b}", 0x80);
        println!("{:b}", flags);
        loop {
            // println!("{:b}", flags);
            if (flags & 0x80) != 0 {
                if (flags & 1) != 0
                    || (self.sector_num.read() != 0
                        || self.cylinder_low.read() != 0
                        || self.cylinder_high.read() != 0)
                {
                    return None;
                } else if (flags & 8) != 0 {
                    println!("{:b}", flags);

                    break;
                }
            } else {
                flags = self.status.read();
            }
        }
        let mut ident: [u16; 256] = [0; 256];
        for i in ident.iter_mut() {
            *i = self.data.read();
        }
        Some(ident)
    }
}

impl ATACtrlBus {
    pub fn new(bus_base: u16) -> Self {
        ATACtrlBus {
            alternate_status: PortGeneric::new(bus_base),
            device_control: PortGeneric::new(bus_base),
            drive_addr: PortGeneric::new(bus_base + 1),
        }
    }

    /// Disable interrupts of this [`ATACtrlBus`].
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn disable_interrupts(&mut self) {
        self.device_control.write(2);
    }

    /// Enable interrupts of this [`ATACtrlBus`].
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn enable_interrupts(&mut self) {
        self.device_control.write(0);
    }

    /// Perform software-reset .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn reset(&mut self) {
        self.device_control.write(4);
        self.device_control.write(0);
    }
}

impl PIOBus {
    pub fn new(bus_base: u16, secondary: bool) -> Self {
        let mut bus = PIOBus {
            io: ATAIOBus::new(bus_base),
            ctrl: ATACtrlBus::new(bus_base + 0x206),
            base: bus_base,
            is_secondary: secondary,
            info: None,
        };
        unsafe { println!("Bus Floating: {}", bus.io.check_float()) };
        bus.info = unsafe { bus.io.identify(bus.is_secondary) };
        // unsafe { bus.ctrl.device_control.write(0x02) };
        bus
    }

    /// Flush cache of this [`PIOBus`].
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn flush_cache(&mut self) {
        self.io.command.write(0xe7);
        let mut status = StatusFlags::from_bits_retain(self.io.status.read());
        loop {
            if !status.contains(StatusFlags::BSY) {
                return;
            } else {
                status = StatusFlags::from_bits_retain(self.io.status.read());
            }
        }
    }

    /// Read data from this [`PIOBus`].
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn read(&mut self, lba: [u8; 6], num_sectors: u16) -> Option<Vec<u16>> {
        self.io.head.write(0x40 | (self.is_secondary as u8) << 4);
        self.io.sector_count.write(num_sectors & 0xf0);
        self.io.sector_num.write(lba[3] as u16);
        self.io.cylinder_low.write(lba[4] as u16);
        self.io.cylinder_high.write(lba[5] as u16);
        self.io.sector_count.write(num_sectors & 0x0f);
        self.io.sector_num.write(lba[0] as u16);
        self.io.cylinder_low.write(lba[1] as u16);
        self.io.cylinder_high.write(lba[2] as u16);
        self.io.command.write(0x24);
        let mut status = StatusFlags::from_bits_retain(self.ctrl.alternate_status.read());
        let mut data: Vec<u16> = Vec::new();
        let real_secnum: u32 = if num_sectors == 0 {
            65536
        } else {
            num_sectors as u32
        };
        for _ in 0..real_secnum {
            loop {
                if !status.contains(StatusFlags::BSY) && status.contains(StatusFlags::DRQ) {
                    break;
                } else if status.contains(StatusFlags::ERR) || status.contains(StatusFlags::DF) {
                    return None;
                } else {
                    status = StatusFlags::from_bits_retain(self.ctrl.alternate_status.read());
                }
            }
            for _ in 0..255 {
                data.push(self.io.data.read());
            }
        }
        Some(data)
    }

    /// Write data into this [`PIOBus`].
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn write(&mut self, lba: [u8; 6], num_sectors: u16, is_slave: bool, data: Vec<u16>) {
        self.io.head.write(0x40 | (is_slave as u8) << 4);
        self.io.sector_count.write(num_sectors & 0xf0);
        self.io.sector_num.write(lba[3] as u16);
        self.io.cylinder_low.write(lba[4] as u16);
        self.io.cylinder_high.write(lba[5] as u16);
        self.io.sector_count.write(num_sectors & 0x0f);
        self.io.sector_num.write(lba[0] as u16);
        self.io.cylinder_low.write(lba[1] as u16);
        self.io.cylinder_high.write(lba[2] as u16);
        self.io.command.write(0x24);
        let mut status = StatusFlags::from_bits_retain(self.ctrl.alternate_status.read());
        let mut writehead = 0;
        let real_secnum: u16 = if num_sectors == 0 {
            256
        } else {
            num_sectors as u16
        };
        for _ in 0..real_secnum {
            loop {
                if !status.contains(StatusFlags::BSY) && status.contains(StatusFlags::DRQ) {
                    break;
                } else if status.contains(StatusFlags::ERR) || status.contains(StatusFlags::DF) {
                    return;
                } else {
                    status = StatusFlags::from_bits_retain(self.ctrl.alternate_status.read());
                }
            }
            for _ in 0..255 {
                if writehead < data.len() {
                    self.io.data.write(*data.get(writehead).unwrap());
                    writehead += 1;
                } else {
                    self.io.data.write(0x0);
                }
            }
        }
        self.flush_cache();
    }

    pub fn get_info_vec(&mut self) -> Vec<u16> {
        let mut out: Vec<u16> = vec![0; 256];
        out.clone_from_slice(&self.info.unwrap());
        out
    }
}

pub struct PIOController {}

pub fn asd() -> Option<Vec<u16>> {
    let mut prim_bus = PIOBus::new(0x1f0, false);

    // unsafe { println!("{:x}", prim_bus.io.identify(false).unwrap()[0]) };

    // let test_data: Vec<u16> = vec![0xde, 0xad, 0xbe, 0xef];
    //
    // unsafe {
    //     prim_bus.write(0, 1, test_data);
    // }

    None
    // Some(unsafe { prim_bus.read([0, 0, 0, 0, 0, 0], 1).unwrap() })
}
