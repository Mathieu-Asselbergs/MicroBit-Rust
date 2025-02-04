#![no_std]
#![no_main]

use core::cell::RefCell;

use cortex_m::interrupt::Mutex;
use defmt_rtt as _;
use panic_halt as _;

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use microbit::{
    board::Board,
    display::nonblocking::{Display, GreyscaleImage},
    hal::{
        clocks::Clocks,
        rtc::{Rtc, RtcInterrupt},
    },
    pac::{self, interrupt, RTC0, TIMER1},
};


enum State {
    Running,
    Paused,
}


static DISPLAY: Mutex<RefCell<Option<Display<TIMER1>>>> = Mutex::new(RefCell::new(None));
static ANIM_TIMER: Mutex<RefCell<Option<Rtc<RTC0>>>> = Mutex::new(RefCell::new(None));

static mut IMAGE: [[u8; 5]; 5] = [[0; 5]; 5];


fn random_automata() -> [[u8; 5]; 5] {
    static mut SEED: u16 = 39333;
    let mut result = [[0; 5]; 5];

    unsafe {
        let square = SEED as u32 * SEED as u32;
        SEED = ((square << 8) >> 16) as u16;

        let mut mask = 1;
        for r in 0..5 {
            for c in 0..5 {
                result[r][c] = ((square & mask) >> (5 * r + c)) as u8;
                mask <<= 1;
            }
        }
        
        result
    }
}

fn update_automata<F>(automata: [[u8; 5]; 5], transition_function: F) -> [[u8; 5]; 5]
where
    F: Fn(u8, [u8; 8]) -> u8
{
    let mut result = automata;

    for row in 0..5 {
        for col in 0..5 {
            result[row][col] = transition_function(
                automata[row][col],
                [
                    automata[(row + 4) % 5][(col + 4) % 5],     // Top left
                    automata[(row + 4) % 5][col],               // Top middle
                    automata[(row + 4) % 5][(col + 1) % 5],     // Top right
                    
                    automata[row][(col + 4) % 5],               // Middle left
                    automata[row][(col + 1) % 5],               // Middle right
                    
                    automata[(row + 1) % 5][(col + 4) % 5],     // Bottom left
                    automata[(row + 1) % 5][col],               // Bottom middle
                    automata[(row + 1) % 5][(col + 1) % 5],     // Bottom right
                ]
            );
        }
    }

    result
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
    let Some(mut board) = Board::take() else {
        panic!("Couldn't take ownership of the board!");
    };

    Clocks::new(board.CLOCK).start_lfclk();

    let mut rtc0 = Rtc::new(board.RTC0, 2047).unwrap();
    rtc0.enable_event(RtcInterrupt::Tick);
    rtc0.enable_interrupt(RtcInterrupt::Tick, None);
    rtc0.enable_counter();

    let display = Display::new(board.TIMER1, board.display_pins);

    cortex_m::interrupt::free(move |cs| {
        *DISPLAY.borrow(cs).borrow_mut() = Some(display);
        *ANIM_TIMER.borrow(cs).borrow_mut() = Some(rtc0);
    });

    unsafe {
        board.NVIC.set_priority(pac::interrupt::RTC0, 64);
        board.NVIC.set_priority(pac::interrupt::TIMER1, 128);
        pac::NVIC::unmask(pac::interrupt::RTC0);
        pac::NVIC::unmask(pac::interrupt::TIMER1);
    }


    


    let mut automata = random_automata();
    let mut state = State::Running;
    let mut a_pressed = false;
    let mut b_pressed = false;

    unsafe { IMAGE = automata; }

    loop {
        match state {
            State::Paused => {
                if !b_pressed {
                    if let Ok(true) = board.buttons.button_b.is_low() {
                        b_pressed = true;
                        automata = update_automata(automata, conway_transitions);
                        unsafe { IMAGE = automata; };
                        cortex_m::interrupt::free(|cs| {
                            if let Some(rtc) = ANIM_TIMER.borrow(cs).borrow_mut().as_mut() {
                                rtc.reset_event(RtcInterrupt::Tick);
                            }
                            if let Some(mut display) = DISPLAY.borrow(cs).borrow_mut().as_mut() {
                                draw(&mut display, automata, 7);
                            }
                        });
                    }
                }

                if !a_pressed {
                    if let Ok(true) = board.buttons.button_a.is_low() {
                        a_pressed = true;
                        // Should restart the timer so that interrupts are
                        //  generated to drive the display.
                        cortex_m::interrupt::free(|cs| {
                            if let Some(rtc) = ANIM_TIMER.borrow(cs).borrow_mut().as_mut() {
                                rtc.enable_counter();
                            }
                        });
                        state = State::Running;
                    }
                }
            },

            State::Running => {
                if !b_pressed {
                    if let Ok(true) = board.buttons.button_b.is_low() {
                        b_pressed = true;
                        automata = random_automata();
                        unsafe { IMAGE = automata; };
                        cortex_m::interrupt::free(|cs| {
                            if let Some(rtc) = ANIM_TIMER.borrow(cs).borrow_mut().as_mut() {
                                rtc.reset_event(RtcInterrupt::Tick);
                            }
                            if let Some(mut display) = DISPLAY.borrow(cs).borrow_mut().as_mut() {
                                draw(&mut display, automata, 7);
                            }
                        });
                    }
                }

                if !a_pressed {
                    if let Ok(true) = board.buttons.button_a.is_low() {
                        a_pressed = true;
                        // Should stop the timer so that no interrupts are 
                        //  generated to drive the display.
                        cortex_m::interrupt::free(|cs| {
                            if let Some(rtc) = ANIM_TIMER.borrow(cs).borrow_mut().as_mut() {
                                rtc.disable_counter();
                                rtc.reset_event(RtcInterrupt::Tick);
                            }
                        });
                        state = State::Paused;
                    }
                }
            }

        }

        if let Ok(true) = board.buttons.button_a.is_high() { a_pressed = false; }
        if let Ok(true) = board.buttons.button_b.is_high() { b_pressed = false; }
    }
}

fn draw(display: &mut Display<TIMER1>, mut automata: [[u8; 5]; 5], brightness: u8) {
    for row in &mut automata {
        for cell in row {
            *cell *= brightness;
        }
    }

    display.show(&GreyscaleImage::new(&automata));
}

#[interrupt]
fn TIMER1() {
    cortex_m::interrupt::free(|cs| {
        if let Some(display) = DISPLAY.borrow(cs).borrow_mut().as_mut() {
            display.handle_display_event();
        }
    });
}

#[interrupt]
unsafe fn RTC0() {
    cortex_m::interrupt::free(|cs| {
        if let Some(rtc) = ANIM_TIMER.borrow(cs).borrow_mut().as_mut() {
            rtc.reset_event(RtcInterrupt::Tick);
        }
    });

    IMAGE = update_automata(IMAGE, conway_transitions);
    let image = IMAGE;

    cortex_m::interrupt::free(|cs| {
        if let Some(mut display) = DISPLAY.borrow(cs).borrow_mut().as_mut() {
            draw(&mut display, image, 7);
        }
    });
}