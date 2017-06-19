extern crate gpio;

use std::thread;
use std::time;

fn main() {
    let mut io = gpio::Gpio::from_gpiomem().unwrap();
    io.setup(21, gpio::Direction::Output, gpio::Pud::Up)
        .unwrap();
    for _ in 0..100 {
        io.output(21, gpio::Status::High).unwrap();
        thread::sleep(time::Duration::from_millis(100));
        io.output(21, gpio::Status::Low).unwrap();
        thread::sleep(time::Duration::from_millis(100));
    }
}
