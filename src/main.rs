#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{board::Board, display::blocking::Display, hal::Timer};

#[entry]
fn main() -> ! {
    let Some(board) = Board::take() else {
        panic!("Couldn't take ownership of the board!");
    };

    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);

    let mut frame = [
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 1, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ];

    loop {
        display.show(&mut timer, frame, 1000);
        
        for row in &mut frame {
            for col in row {
                *col = 1 - *col;
            }
        }
    }
}