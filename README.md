# gertrudOS [^1]
[^1]: formerly known as pogOS

A **_very_** basic operating system made in Rust. It's named after my cat Gertrud since she's awesome and of great help when debugging.
<br/>
This is mostly intended as a learning excercise in both Rust and OS-development and will probably make people, who actually know about these things, very frustrated.
<br/>
So be warned if you want to actually try this or look at the code.

## Planned ""Features""

- [ ] I/O
  - [x] Serial
  - [x] Keyboard
  - [ ] Mouse
  - [ ] Storage
    - [x] ATA PIO
    - [ ] DMA
  - [ ] Network
  - [x] Framebuffer Video
  - [ ] PCI
  - [ ] USB
- [ ] Utility
  - [x] Interrupts using APIC
  - [x] Memory allocation
  - [x] Async
  - [ ] Multiprocessing
  - [ ] ACPI
- [ ] ...

## Resources

- Most of the pages on the OSDev [wiki](https://wiki.osdev.org) and [forum](https://forum.osdev.org/)
- [Writing an OS in Rust](https://os.phil-opp.com/) by Philipp Oppermann
