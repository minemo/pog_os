use alloc::vec::Vec;
use alloc::{string::String, vec};
use x86_64::instructions::port::{PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess};

use crate::{println, serial_println};

#[derive(Debug)]
struct ATAError;

pub struct ATAIOBus {
    data: PortGeneric<u16, ReadWriteAccess>,
    error: PortGeneric<u8, ReadOnlyAccess>,
    features: PortGeneric<u8, WriteOnlyAccess>,
    sector_count: PortGeneric<u8, ReadWriteAccess>,
    sector_num: PortGeneric<u8, ReadWriteAccess>,
    cylinder_low: PortGeneric<u8, ReadWriteAccess>,
    cylinder_high: PortGeneric<u8, ReadWriteAccess>,
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

    /// Polls the [`ATAIOBus`] until it is ready for next command or data read (status bit 7 clears).
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn poll_til_ready(&mut self) -> u8 {
        let mut flags: u8;
        loop {
            flags = self.status.read();
            if flags & 1 != 0 {
                panic!("Error {:08b} during operation", self.error.read());
            }
            if (flags >> 7) & 1 == 0 {
                return flags;
            }
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
        self.poll_til_ready();
        self.head.write(0xa0 | ((secondary as u8) << 4));
        _ = self.poll_til_ready();
        self.sector_count.write(0);
        self.sector_num.write(0);
        self.cylinder_low.write(0);
        self.cylinder_high.write(0);
        self.command.write(0xec);
        _ = self.poll_til_ready();
        let mut flags: u8;
        loop {
            flags = self.poll_til_ready();
            if (flags & 1) == 1 {
                println!("Identify error: {:b}", self.error.read());
                return None;
            } else if (flags >> 3) & 1 == 1 {
                break;
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
        let mut atabus = ATACtrlBus {
            alternate_status: PortGeneric::new(bus_base),
            device_control: PortGeneric::new(bus_base),
            drive_addr: PortGeneric::new(bus_base + 1),
        };
        unsafe {
            atabus.device_control.write(0);
        }
        atabus
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
        self.device_control.write(4_u8);
        self.device_control.write(0_u8);
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
        unsafe {
            bus.ctrl.reset();
            bus.info = bus.io.identify(bus.is_secondary);
        };
        bus
    }

    /// Flush cache of this [`PIOBus`].
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn flush_cache(&mut self) {
        self.io.command.write(0xea);
        _ = self.io.poll_til_ready();
    }

    /// Read data from this [`PIOBus`].
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn read(&mut self, lba: u32, num_sectors: u16) -> Option<Vec<u16>> {
        self.io.sector_count.write((num_sectors >> 8) as u8);
        self.io.sector_num.write((lba >> 24) as u8);
        self.io.cylinder_low.write(0);
        self.io.cylinder_high.write(0);
        self.io.sector_count.write((num_sectors << 8 >> 8) as u8);
        self.io.sector_num.write((lba & 0xff) as u8);
        self.io.cylinder_low.write(((lba & 0x0000ff00) >> 8) as u8);
        self.io
            .cylinder_high
            .write(((lba & 0x00ff0000) >> 16) as u8);
        self.io.head.write(0x40 | ((self.is_secondary as u8) << 4));
        self.io.command.write(0x24);
        let mut status: u8;
        let mut data: Vec<u16> = Vec::new();
        let real_secnum: u32 = if num_sectors == 0 {
            65536
        } else {
            num_sectors as u32
        };
        for _ in 0..real_secnum {
            status = self.io.poll_til_ready();
            if status & 1 == 1 || (status >> 5) & 1 == 1 {
                println!("Read error: {:08b}", self.io.error.read());
                return None;
            }
            for _ in 0..256 {
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
    pub unsafe fn write(&mut self, lba: u32, data: Vec<u16>) {
        let num_sectors = data.chunks(256).len();
        self.io.sector_count.write((num_sectors >> 8) as u8);
        self.io.sector_num.write((lba >> 24) as u8);
        self.io.cylinder_low.write(0);
        self.io.cylinder_high.write(0);
        self.io.sector_count.write((num_sectors << 8 >> 8) as u8);
        self.io.sector_num.write((lba & 0xff) as u8);
        self.io.cylinder_low.write(((lba & 0x0000ff00) >> 8) as u8);
        self.io
            .cylinder_high
            .write(((lba & 0x00ff0000) >> 16) as u8);
        self.io.head.write(0x40 | ((self.is_secondary as u8) << 4));
        self.io.command.write(0x34);
        let mut status: u8;
        for c in data.chunks(256) {
            status = self.io.poll_til_ready();
            if status & 1 == 1 || (status >> 5) & 1 == 1 {
                println!("Write error: {:08b}", self.io.error.read());
                return;
            }
            for b in c.iter() {
                self.io.data.write(*b);
            }
            if c.len() < 256 {
                for _ in 0..256 - c.len() {
                    self.io.data.write(0x0000);
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

    pub fn get_selected_drive(&mut self) -> u8 {
        unsafe { !self.ctrl.drive_addr.read() & 3 }
    }

    pub fn get_error(&mut self) -> u8 {
        unsafe { self.io.error.read() }
    }
}

pub struct PIOController {}

pub fn test_read() -> Option<Vec<u16>> {
    let mut prim_bus = PIOBus::new(0x1f0, true);
    println!(
        "Serial Number {}",
        prim_bus
            .info
            .unwrap()
            .iter()
            .skip(9)
            .take(8)
            .flat_map(|x| x.to_be_bytes())
            .skip(2)
            .map(|x| x as char)
            .collect::<String>()
            .trim()
    );

    println!(
        "Supports 48bit PIO: {}",
        prim_bus.get_info_vec()[83] >> 10 & 1
    );

    let data = unsafe { prim_bus.read(0, 1152).unwrap() };

    Some(data)
}

pub fn test_write() {
    let mut prim_bus = PIOBus::new(0x1f0, true);
    println!(
        "Serial Number {}",
        prim_bus
            .info
            .unwrap()
            .iter()
            .skip(9)
            .take(8)
            .flat_map(|x| x.to_be_bytes())
            .skip(2)
            .map(|x| x as char)
            .collect::<String>()
            .trim()
    );

    println!(
        "Supports 48bit PIO: {}",
        prim_bus.get_info_vec()[83] >> 10 & 1
    );

    let mut test_data: Vec<u16> = vec![0xaaaa, 0x0000];
    test_data.append(&mut Vec::from([0xffff; 254]));

    unsafe {
        prim_bus.write(0, test_data);
    }
}
