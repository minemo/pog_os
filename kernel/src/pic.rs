use x86_64::instructions::port::Port;

const ICW1_ICW4: u8 = 0x01;
const ICW1_INIT: u8 = 0x10;
const PIC_EOI: u8 = 0x20;
const ICW4_8086: u8 = 0x01;

struct Pic {
  base: u8,
  command: Port<u8>,
  data: Port<u8>,
}

impl Pic {
  fn should_handle(&self, interrupt_id:u8) -> bool {
    self.base <= interrupt_id && interrupt_id < self.base + 8
  }

  unsafe fn eoi(&mut self) {
    self.command.write(PIC_EOI);
  }

  unsafe fn read_mask(&mut self) -> u8 {
    self.data.read()
  }

  unsafe fn write_mask(&mut self, mask: u8) {
    self.data.write(mask)
  }
}

pub struct ChainedPics {
  pics: [Pic; 2],
}

impl ChainedPics {
  pub const unsafe fn new(b1: u8, b2: u8) -> ChainedPics {
    ChainedPics { 
      pics: [
        Pic {
          base: b1,
          command: Port::new(0x20),
          data: Port::new(0x21)
        },
        Pic {
          base: b2,
          command: Port::new(0xA0),
          data: Port::new(0xA1)
        }
      ]
    }
  }

  pub unsafe fn init(&mut self) {
    // since we cant use timers yet, writing to port 0x80 to spend some time
    let mut wait_port: Port<u8> = Port::new(0x80);
    let mut wait = || wait_port.write(0); // write some data

    // backup masks before init
    let saved_masks = self.read_masks();

    // start init on both PICs
    self.pics[0].data.write(ICW1_INIT | ICW1_ICW4);
    wait();
    self.pics[1].data.write(ICW1_INIT | ICW1_ICW4);
    wait();

    // Send 3-Byte init sequence to each
    self.pics[0].data.write(self.pics[0].base);
    wait();
    self.pics[1].data.write(self.pics[1].base);
    wait();

    self.pics[0].data.write(4);
    wait();
    self.pics[1].data.write(2);
    wait();

    self.pics[0].data.write(ICW4_8086);
    wait();
    self.pics[1].data.write(ICW4_8086);
    wait();

    // restore masks
    self.write_masks(saved_masks[0], saved_masks[1]);
  }

  pub unsafe fn read_masks(&mut self) -> [u8; 2] {
    [self.pics[0].read_mask(), self.pics[1].read_mask()]
  }

  pub unsafe fn write_masks(&mut self, m1: u8, m2: u8) {
    self.pics[0].write_mask(m1);
    self.pics[1].write_mask(m2);
  }

  pub unsafe fn disable(&mut self) {
    self.write_masks(0xFF, 0xFF)
  }

  // does any PIC handle the interrupt?
  pub fn should_handle(&self, interrupt_id: u8) -> bool {
    self.pics.iter().any(|p| p.should_handle(interrupt_id))
  }

  pub unsafe fn send_eoi(&mut self, interrupt_id: u8) {
    if self.should_handle(interrupt_id) {
      if self.pics[1].should_handle(interrupt_id) {
        self.pics[1].eoi();
      }
      self.pics[0].eoi();
    }
  }

}