use crate::framebuffer::FBWRITER;
use crate::println;
use crate::{ata::pio::test_read, print};
use alloc::string::String;
use alloc::vec::Vec;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{stream::Stream, task::AtomicWaker, StreamExt};
use spin::{mutex::Mutex, once::Once};
use x86_64::registers;

static COMMAND_QUEUE: Once<Mutex<ArrayQueue<String>>> = Once::new();
static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_console_char(character: char) {
    if let Some(queue) = COMMAND_QUEUE.get() {
        let mut newpartial = String::from(character);
        if let Some(partial_command) = queue.lock().pop() {
            newpartial.insert_str(0, partial_command.as_str());
        }
        if queue.lock().push(newpartial).is_err() {
            println!("WARNING: input-queue full, dropping input");
        } else if character == '\n' {
            WAKER.wake();
        }
    } else {
        println!("WARNING: input-queue uninitialized");
    }
}

pub struct ConsoleStream {
    _private: (),
}

impl ConsoleStream {
    pub fn new() -> Self {
        COMMAND_QUEUE.call_once(|| Mutex::new(ArrayQueue::new(100)));
        ConsoleStream { _private: () }
    }
}

impl Stream for ConsoleStream {
    type Item = String;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = COMMAND_QUEUE.get().expect("not initialized");

        if let Some(command) = queue.lock().pop() {
            return Poll::Ready(Some(command));
        }

        WAKER.register(cx.waker());
        match queue.lock().pop() {
            Some(command) => {
                WAKER.take();
                Poll::Ready(Some(command))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn run_console() {
    let mut commands = ConsoleStream::new();

    while let Some(command) = commands.next().await {
        match command.trim() {
            "image" => {
                if let Some(x) = test_read() {
                    let data: Vec<u8> = x.iter().flat_map(|v| v.to_le_bytes()).collect();
                    let imginfo = data[0..0x10].iter().map(|&v| v as char).collect::<String>();
                    let mut imglines = imginfo.split('\n');
                    let imgsize = imglines
                        .nth(1)
                        .unwrap()
                        .split(" ")
                        .map(|v| v.parse::<u16>().unwrap())
                        .collect::<Vec<u16>>();
                    // let r: Vec<u8> = data[0x0f..data.len()].iter().step_by(3).copied().collect();
                    // let g: Vec<u8> = data[0x0f..data.len()]
                    //     .iter()
                    //     .skip(1)
                    //     .step_by(3)
                    //     .copied()
                    //     .collect();
                    // let b: Vec<u8> = data[0x0f..data.len()]
                    //     .iter()
                    //     .skip(2)
                    //     .step_by(3)
                    //     .copied()
                    //     .collect();

                    // let yoff = FBWRITER.get().unwrap().lock().y_pos;
                    // FBWRITER.get().unwrap().lock().draw_image(
                    //     0,
                    //     yoff,
                    //     imgsize[0] as usize,
                    //     imgsize[1] as usize,
                    //     &[r.as_slice(), g.as_slice(), b.as_slice()],
                    // );
                }
            }
            "dbg" => {
                println!(
                    "Register info:\nRFLAGS: {:?}\nCR0: {:?}\nCR2: {:?}\nCR3: {:?}\nCR4: {:?}",
                    registers::rflags::read(),
                    registers::control::Cr0::read(),
                    registers::control::Cr2::read(),
                    registers::control::Cr3::read(),
                    registers::control::Cr4::read()
                );
            }
            "clear" => {
                FBWRITER.get().unwrap().lock().clear();
            }
            "help" => {
                println!("TODO");
            }
            "exit" => {
                use x86_64::instructions::port::Port;

                unsafe {
                    let mut port = Port::new(0xf4);
                    port.write(0x10_u32);
                }
            }
            _ => {}
        }
        print!(">");
    }
}
