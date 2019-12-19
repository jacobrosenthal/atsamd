//! Generate a mandelbrot set and draw it to screen

#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_halt;
use pygamer as hal;

use embedded_graphics::drawable::Pixel;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::*;

use hal::clock::GenericClockController;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
use itertools::Itertools;
use num::Complex;

/// The width and height of the display
const DISP_SIZE_X: usize = 160;
const DISP_SIZE_Y: usize = 128;

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

    //todo good variable choices?
    let max_iterations = 256u16;
    let cxmin = -2f32;
    let cxmax = 1f32;
    let cymin = -1.5f32;
    let cymax = 1.5f32;
    let scalex = (cxmax - cxmin) / DISP_SIZE_X as f32;
    let scaley = (cymax - cymin) / DISP_SIZE_Y as f32;

    (0..DISP_SIZE_X)
        .cartesian_product(0..DISP_SIZE_Y)
        .map(|(x, y)| {
            let cx = cxmin + x as f32 * scalex;
            let cy = cymin + y as f32 * scaley;

            let c = Complex::new(cx, cy);
            let mut z = Complex::new(0f32, 0f32);

            let mut i = 0;
            for t in 0..max_iterations {
                //todo manhattan norm ok?
                if z.l1_norm() > 2.0 {
                    break;
                }
                z = z * z + c;
                i = t;
            }

            //todo pixel threshold?
            let color = if i > 10 {
                RgbColor::RED
            } else {
                RgbColor::BLACK
            };

            Pixel(Point::new(x as i32, y as i32), color)
        })
        .draw(&mut display);

    loop {}
}
