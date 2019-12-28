//! Game of life from https://github.com/rustwasm/wasm_game_of_life

#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_halt;
use pygamer as hal;

use embedded_graphics::prelude::*;

use hal::clock::GenericClockController;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};

use heapless::consts::*;
use heapless::Vec;

mod life;
use life::{Cell, Universe};

/// The width and height of the display
const DISP_SIZE_X: i32 = 160;
const DISP_SIZE_Y: i32 = 128;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut pins = hal::Pins::new(peripherals.PORT).split();
    let mut delay = hal::delay::Delay::new(core.SYST, &mut clocks);

    let (mut display, _backlight) = pins
        .display
        .init(
            &mut clocks,
            peripherals.SERCOM4,
            &mut peripherals.MCLK,
            peripherals.TC2,
            &mut delay,
            &mut pins.port,
        )
        .unwrap();

    //wasted space, but there is no U20480
    let cells = Vec::<Cell, U32768>::new();
    //more wasted space, a copy to edit while we keep last values
    let next = Vec::<Cell, U32768>::new();

    let mut universe = Universe::new(DISP_SIZE_X as u32, DISP_SIZE_Y as u32, cells, next);

    loop {
        universe.tick();

        universe
            .iter()
            .map(|(row, col, cell)| {
                let color = if cell == Cell::Alive {
                    RgbColor::RED
                } else {
                    RgbColor::BLACK
                };
                Pixel(Point::new(col as i32, row as i32), color)
            })
            .draw(&mut display)
            .ok();
    }
}
