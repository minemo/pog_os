use crossbeam_queue::ArrayQueue;
use spin::{once::Once,mutex::Mutex};

static SCANCODE_QUEUE: Once<ArrayQueue<u8>> = Once::new();


