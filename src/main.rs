#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use cortex_m_rt::entry;
use microbit::{board::Board, display::blocking::Display, hal::Timer};


fn random_automaton() -> [[u8; 5]; 5] {
    static mut SEED: u16 = 39333;
    let mut result = [[0; 5]; 5];

    unsafe {
        let square = SEED as u32 * SEED as u32;
        SEED = ((square << 8) >> 16) as u16;

        let mut mask = 1;
        for r in 0..5 {
            for c in 0..5 {
                result[r][c] = (square & mask) as u8;
                mask <<= 1;
            }
        }
        
        result
    }
}

fn update_automata<F>(automata: &mut [[u8; 5]; 5], transition_function: F)
where
    F: Fn(u8, [u8; 8]) -> u8
{
    let dummy = *automata;

    for row in 0..5 {
        for col in 0..5 {
            automata[row][col] = transition_function(
                dummy[row][col],
                [
                    dummy[(row + 4) % 5][(col + 4) % 5],     // Top left
                    dummy[(row + 4) % 5][col],               // Top middle
                    dummy[(row + 4) % 5][(col + 1) % 5],     // Top right
                    
                    dummy[row][(col + 4) % 5],               // Middle left
                    dummy[row][(col + 1) % 5],               // Middle right
                    
                    dummy[(row + 1) % 5][(col + 4) % 5],     // Bottom left
                    dummy[(row + 1) % 5][col],               // Bottom middle
                    dummy[(row + 1) % 5][(col + 1) % 5],     // Bottom right
                ]
            );
        }
    }
}

fn conway_transitions(center_cell: u8, neighbors: [u8; 8]) -> u8 {
    let live_neighbor_count = neighbors.iter().filter(|n| **n != 0).count();

    match (center_cell, live_neighbor_count) {
        (0, 3) => 1,
        (_, 2..=3) => 1,
        _ => 0,
    }
}

#[entry]
fn main() -> ! {
    let Some(board) = Board::take() else {
        panic!("Couldn't take ownership of the board!");
    };

    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    let mut automata = random_automaton();

    loop {
        display.show(&mut timer, automata, 1000);

        update_automata(&mut automata, conway_transitions);
    }
}