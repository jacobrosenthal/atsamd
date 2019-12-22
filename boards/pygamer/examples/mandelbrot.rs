//! Generate a mandelbrot set and draw it to screen

#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_halt;
use pygamer as hal;

use embedded_graphics::drawable::Pixel;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::style::PrimitiveStyle;

use hal::adc::Adc;
use hal::clock::GenericClockController;
use hal::entry;
use hal::pac::gclk::pchctrl::GEN_A::GCLK11;
use hal::pac::{CorePeripherals, Peripherals};
use itertools::Itertools;
use num::Complex;

use hal::gpio;

use smart_leds::hsv::{hsv2rgb, Hsv};

/// The width and height of the display
const DISP_SIZE_X: i32 = 160;
const DISP_SIZE_Y: i32 = 128;

const DELTA: i16 = 2;
const CHARACTER_COLOR: Rgb565 = RgbColor::RED;
const CHARACTER_SIZE: i32 = 10;

//todo good variable choices?
const MAX_ITERATIONS: u8 = 255;
const CXMIN: f32 = -2f32;
const CXMAX: f32 = 1f32;
const CYMIN: f32 = -1.5f32;
const CYMAX: f32 = 1.5f32;
const SCALEX: f32 = (CXMAX - CXMIN) / DISP_SIZE_X as f32;
const SCALEY: f32 = (CYMAX - CYMIN) / DISP_SIZE_Y as f32;

fn to_pixels(triple: (i32, i32, u32)) -> Pixel<Rgb565> {
    let color = hsv2rgb(Hsv {
        hue: triple.2 as u8,
        sat: 255,
        val: 32,
    });

    Pixel(
        Point::new(triple.0 as i32, triple.1 as i32),
        Rgb565::new(color.r, color.g, color.b),
    )
}
fn mandelbrot(pair: (i32, i32)) -> (i32, i32, u32) {
    let cx = CXMIN + pair.0 as f32 * SCALEX;
    let cy = CYMIN + pair.1 as f32 * SCALEY;

    let c = Complex::new(cx, cy);
    let mut z = Complex::new(0f32, 0f32);

    let mut i = 0;
    for t in 0..MAX_ITERATIONS {
        //todo manhattan norm ok?
        if z.l1_norm() > 2.0 {
            break;
        }
        z = z * z + c;
        i = t;
    }

    (pair.0, pair.1, i as u32)
}

fn move_rectangle(
    display: &mut st7735_lcd::ST7735<
        hal::sercom::SPIMaster4<
            hal::sercom::Sercom4Pad2<gpio::Pb14<gpio::PfC>>,
            hal::sercom::Sercom4Pad3<gpio::Pb15<gpio::PfC>>,
            hal::sercom::Sercom4Pad1<gpio::Pb13<gpio::PfC>>,
        >,
        hal::gpio::Pb5<gpio::Output<gpio::PushPull>>,
        hal::gpio::Pa0<gpio::Output<gpio::PushPull>>,
    >,
    position: &mut Point,
    new_position: Point,
) {
    //keep within screen (including size of our pixel boy)
    if new_position.x < 0
        || new_position.x + CHARACTER_SIZE > DISP_SIZE_X
        || new_position.y < 0
        || new_position.y + CHARACTER_SIZE > DISP_SIZE_Y
    {
        return;
    }

    // max sum is DISP_SIZE_X*DISP_SIZE_Y*MAX_ITERATIONS
    // 160*128*255 or 5222400 < U32 Max of 4294967295
    let presence: u32 = (new_position.x..=(new_position.x + CHARACTER_SIZE))
        .cartesian_product(new_position.y..=(new_position.y + CHARACTER_SIZE))
        .map(mandelbrot)
        .map(|(_x, _y, i)| if i > 10 { i } else { 0 })
        .sum();

    //avoid moving into mandelbrot set
    if presence == 0 {
        // Clear old rectangle
        (position.x..=(position.x + CHARACTER_SIZE))
            .cartesian_product(position.y..=(position.y + CHARACTER_SIZE))
            .map(mandelbrot)
            .map(to_pixels)
            .draw(display)
            .ok();

        //draw new location
        Rectangle::new(
            new_position,
            Point::new(
                new_position.x + CHARACTER_SIZE,
                new_position.y + CHARACTER_SIZE,
            ),
        )
        .into_styled(PrimitiveStyle::with_fill(CHARACTER_COLOR))
        .draw(display)
        .ok();

        *position = new_position;
    }
}

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

    let mut adc1 = Adc::adc1(peripherals.ADC1, &mut peripherals.MCLK, &mut clocks, GCLK11);
    let mut joystick = pins.joystick.init(&mut pins.port);

    //draw background
    (0..DISP_SIZE_X)
        .cartesian_product(0..DISP_SIZE_Y)
        .map(mandelbrot)
        .map(to_pixels)
        .draw(&mut display)
        .ok();

    //draw square at starting point
    let mut position = Point::new(0, 20);
    Rectangle::new(
        position,
        Point::new(position.x + CHARACTER_SIZE, position.y + CHARACTER_SIZE),
    )
    .into_styled(PrimitiveStyle::with_fill(CHARACTER_COLOR))
    .draw(&mut display)
    .ok();

    loop {
        let (x, y) = joystick.read(&mut adc1);

        //zero around zero
        // let x: i16 = x as i16 - 2048;
        // let y: i16 = y as i16 - 2048;

        //map up/down to control rainbow color 0-255
        let x = map_from(x as i16, (0, 4095), (-DELTA, DELTA));
        let y = map_from(y as i16, (0, 4095), (-DELTA, DELTA));

        // ///ie just add our x and y, scaled down?, to existing point
        let new_position = Point::new(position.x + x as i32, position.y + y as i32);
        move_rectangle(&mut display, &mut position, new_position);
    }
}

fn map_from(input: i16, from_range: (i16, i16), to_range: (i16, i16)) -> i16 {
    debug_assert!(from_range.0 < from_range.1);
    debug_assert!(to_range.0 < to_range.1);
    debug_assert!(input >= from_range.0);
    debug_assert!(input <= from_range.1);

    let from: f32 = (from_range.1 - from_range.0).into();
    let to: f32 = (to_range.1 - to_range.0).into();
    ((input - from_range.0) as f32 / from * to + to_range.0 as f32) as i16
}
