use alloc::vec::{self, Vec};
use bitflags::bitflags;
use x86_64::{
    addr::PhysAddr,
    instructions::port::{Port, PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess},
};

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

    pub unsafe fn check_float(&mut self) -> bool {
        self.status.read() == 0xff
    }

    pub unsafe fn identify(&mut self, secondary: bool) -> Option<[u16; 256]> {
        self.head.write(0xA0 + ((secondary as u8) << 4));
        self.sector_num.write(0);
        self.cylinder_low.write(0);
        self.cylinder_high.write(0);
        self.command.write(0xec);
        let mut flags: StatusFlags = StatusFlags::from_bits_retain(self.status.read());
        if flags.is_empty() {
            None
        } else {
            loop {
                if !flags.contains(StatusFlags::BSY) {
                    if flags.contains(StatusFlags::ERR)
                        || (self.sector_num.read() != 0
                            || self.cylinder_low.read() != 0
                            || self.cylinder_high.read() != 0)
                    {
                        return None;
                    } else if flags.contains(StatusFlags::DRQ) {
                        break;
                    }
                } else {
                    flags = StatusFlags::from_bits_retain(self.status.read());
                }
            }
            let mut ident: [u16; 256] = [0; 256];
            for i in ident.iter_mut() {
                *i = self.data.read();
            }
            Some(ident)
        }
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
        bus.info = unsafe { bus.io.identify(secondary) };
        unsafe { bus.ctrl.device_control.write(0x02) };
        bus
    }

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

    pub unsafe fn read(&mut self, lba: u32, num_sectors: u8) -> Option<Vec<u16>> {
        self.io
            .head
            .write(0xe0 | ((self.is_secondary as u8) << 4) | ((lba >> 24) & 0x0F) as u8);
        self.io.features.write(0x0);
        self.io.sector_count.write(num_sectors);
        self.io.sector_num.write(lba as u8);
        self.io.cylinder_low.write((lba >> 8) as u8);
        self.io.cylinder_high.write((lba >> 16) as u8);
        self.io.command.write(0x20);
        let mut status = StatusFlags::from_bits_retain(self.io.status.read());
        let mut data: Vec<u16> = Vec::new();
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
                    return None;
                } else {
                    status = StatusFlags::from_bits_retain(self.io.status.read());
                }
            }
            for _ in 0..255 {
                data.push(self.io.data.read());
            }
        }
        Some(data)
    }

    pub unsafe fn write(&mut self, lba: u32, num_sectors: u8, data: Vec<u16>) {
        self.io
            .head
            .write(0xe0 | ((self.is_secondary as u8) << 4) | ((lba >> 24) & 0x0F) as u8);
        self.io.features.write(0x0);
        self.io.sector_count.write(num_sectors);
        self.io.sector_num.write(lba as u8);
        self.io.cylinder_low.write((lba >> 8) as u8);
        self.io.cylinder_high.write((lba >> 16) as u8);
        self.io.command.write(0x30);
        let mut status = StatusFlags::from_bits_retain(self.io.status.read());
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
                    status = StatusFlags::from_bits_retain(self.io.status.read());
                }
            }
            for _ in 0..255 {
                if writehead < data.len() {
                    self.io.data.write(0xff);
                    writehead += 1;
                } else {
                    self.io.data.write(0x0);
                }
            }
        }
        self.flush_cache();
    }
}

pub struct PIOController {}

pub fn asd() -> Option<Vec<u16>> {
    let mut prim_bus = PIOBus::new(0x1f0, false);
    let sec_bus = PIOBus::new(0x170, true);

    let mut test_data: Vec<u16> = Vec::new();
    test_data.push(0xdea);
    test_data.push(0xdbe);
    test_data.push(0xef0);

    unsafe {
        prim_bus.write(1, 1, test_data);
    }

    Some(unsafe { prim_bus.read(1, 1).unwrap() })
}
