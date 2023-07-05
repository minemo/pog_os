#![no_std] // no rust standard lib
#![no_main] // no rust entry points

use core::{fmt::Write, any::Any};

use crate::framebuffer::FrameBufferWriter;

mod framebuffer;

bootloader_api::entry_point!(kmain);

fn kmain(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    let possible_fb = boot_info.framebuffer.as_mut();
    match possible_fb {
        Some(fb) => {
            let info = fb.info();
            let mut logger: FrameBufferWriter = framebuffer::FrameBufferWriter::new(fb.buffer_mut(), info);
            writeln!(logger,"there is an impostor among us!").unwrap();
            writeln!(logger,"or is there?").unwrap();
        },
        None => panic!(),
    }
    loop {}
}

struct _PanicPayload<'a, 'b> {
    logger: &'a FrameBufferWriter,
    message: &'b dyn Any
}

// handles panic (duh)
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

