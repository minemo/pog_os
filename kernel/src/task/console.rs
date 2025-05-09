use crate::framebuffer::print_image;
use crate::{ata::pio::test_read, print};
use crate::{clear, println};
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
use x86_64::registers::debug::DebugAddressRegister;

static COMMAND_QUEUE: Once<Mutex<ArrayQueue<String>>> = Once::new();
static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_console_char(character: char) {
    print!("{}", character);
    if let Some(queue) = COMMAND_QUEUE.get() {
        //TODO handle backspace '\u{8}'
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
        //TODO make commands serializable/implement propper lexer
        let mut args = command.trim().split(" ");
        match args.next().unwrap() {
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
                    let r: Vec<u8> = data[0x0f..data.len()].iter().step_by(3).copied().collect();
                    let g: Vec<u8> = data[0x0f..data.len()]
                        .iter()
                        .skip(1)
                        .step_by(3)
                        .copied()
                        .collect();
                    let b: Vec<u8> = data[0x0f..data.len()]
                        .iter()
                        .skip(2)
                        .step_by(3)
                        .copied()
                        .collect();

                    print_image(
                        imgsize[0] as usize,
                        imgsize[1] as usize,
                        &[r.as_slice(), g.as_slice(), b.as_slice()],
                    );
                }
            }
            "dbg" => match args.next().unwrap_or("") {
                "all" => {
                    print!(
                        "RFLAGS: {:?}\nCR0: {:?}\nCR2: {:?}\nCR3: {:?}\nCR4: {:?}",
                        registers::rflags::read(),
                        registers::control::Cr0::read(),
                        registers::control::Cr2::read(),
                        registers::control::Cr3::read(),
                        registers::control::Cr4::read(),
                    );
                    print!(
                        "DR0: {:?}\nDR1: {:?}\nDR2: {:?}\nDR3: {:?}\nDR6: {:?}\nDR7: {:?}\n",
                        registers::debug::Dr0::read(),
                        registers::debug::Dr1::read(),
                        registers::debug::Dr2::read(),
                        registers::debug::Dr3::read(),
                        registers::debug::Dr6::read(),
                        registers::debug::Dr7::read()
                    );
                }
                "rflags" => {
                    print!("RFLAGS: {:?}", registers::rflags::read());
                }
                "cr" => {
                    print!(
                        "CR0: {:?}\nCR2: {:?}\nCR3: {:?}\nCR4: {:?}",
                        registers::control::Cr0::read(),
                        registers::control::Cr2::read(),
                        registers::control::Cr3::read(),
                        registers::control::Cr4::read()
                    );
                }
                "dr" => {
                    print!(
                        "DR0: {:?}\nDR1: {:?}\nDR2: {:?}\nDR3: {:?}\nDR6: {:?}\nDR7: {:?}",
                        registers::debug::Dr0::read(),
                        registers::debug::Dr1::read(),
                        registers::debug::Dr2::read(),
                        registers::debug::Dr3::read(),
                        registers::debug::Dr6::read(),
                        registers::debug::Dr7::read()
                    );
                }
                _ => {
                    println!("Unknown debug target!\nUsage: dbg [all,rflags,cr,dr]");
                }
            },
            "clear" => {
                clear!();
            }
            "help" => {
                println!("clear - Clear the screen\ndbg - Print debug info\nhelp - Print this help message\nimage - Draw an image to screen\nqexit - Exit QEMU");
            }
            "qexit" => {
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
