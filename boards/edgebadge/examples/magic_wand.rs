//! Port of https://learn.adafruit.com/tensorflow-lite-for-edgebadge-kit-quickstart/gesture-demo
//! Note I dont have the orientation figured out yet. My goal is that With the
//! screen facing you, and the USB port pointing to the ceiling perform one of
//! three gestures:
//!
//! Wing: This gesture is a W starting at your top left, going down, up, down up
//! to your top right When that gesture is detected you'lll see the front
//! NeoPixels turn yellow.
//!
//! Ring: This gesture is a O starting at top center, then moving clockwise in a
//! circle to the right, then down, then left and back to when you started in
//! the top center When that gesture is detected you'll see the front NeoPixels
//! turn purple.
//!
//! Slope: This gesture is an L starting at your top right, moving diagonally to
//! your bottom left, then straight across to bottom right. When that gesture is
//! detected you'll see the front NeoPixels turn light blue.
//!
//! Setup:
//! * figure out how to install arm-none-eabi-gcc for your os
//! * `rustup update` to get a recent nightly near august
//! Upload:
//! * `cargo +nightly hf2 --release --example magic_wand --features="tensorflow"`
//!
//! Note you dont want to use use_rtt feature and thus cant dbgprint anything
//! unless you have a debugger attached which you probably dont

#![no_std]
#![no_main]

use edgebadge as hal;
#[cfg(not(feature = "use_rtt"))]
use panic_halt as _;
#[cfg(feature = "use_rtt")]
use panic_rtt as _;

use accelerometer::RawAccelerometer;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;
use hal::time::KiloHertz;
use hal::timer::SpinTimer;
use hal::{clock::GenericClockController, delay::Delay};
use lis3dh::{Lis3dh, SlaveAddr};
use smart_leds::{brightness, colors, hsv::RGB8, SmartLedsWrite};
use tfmicro::{MicroInterpreter, Model, MutableOpResolver};

#[cfg(feature = "use_rtt")]
pub use hal::dbgprint;

#[cfg(not(feature = "use_rtt"))]
macro_rules! dbgprint {
    ($($arg:tt)*) => {{}};
}

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core_peripherals = CorePeripherals::take().unwrap();

    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut pins = hal::Pins::new(peripherals.PORT).split();

    let timer = SpinTimer::new(4);
    let mut neopixel = pins.neopixel.init(timer, &mut pins.port);

    let mut delay = Delay::new(core_peripherals.SYST, &mut clocks);

    let i2c = pins.i2c.init(
        &mut clocks,
        KiloHertz(400),
        peripherals.SERCOM2,
        &mut peripherals.MCLK,
        &mut pins.port,
    );

    let mut lis3dh = Lis3dh::new(i2c, SlaveAddr::Default).unwrap();
    lis3dh.set_mode(lis3dh::Mode::Normal).unwrap();
    lis3dh.set_range(lis3dh::Range::G4).unwrap();
    lis3dh.set_datarate(lis3dh::DataRate::Hz_25).unwrap();

    let model = include_bytes!("../models/magic_wand_edgebadge.tflite");

    #[cfg(feature = "test_ring")]
    let test = include_bytes!("../models/ring_micro_f9643d42_nohash_4.data")
        .chunks_exact(4)
        .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
        .collect::<heapless::Vec<_, heapless::consts::U384>>();

    #[cfg(feature = "test_slope")]
    let test = include_bytes!("../models/slope_micro_f2e59fea_nohash_1.data")
        .chunks_exact(4)
        .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
        .collect::<heapless::Vec<_, heapless::consts::U384>>();

    #[cfg(any(feature = "test_ring", feature = "test_slope"))]
    dbgprint!("{:?}", &test[..]);

    // Map the model into a usable data structure. This doesn't involve
    // any copying or parsing, it's a very lightweight operation.
    let model = Model::from_buffer(&model[..]).unwrap();

    // Create an area of memory to use for input, output, and
    // intermediate arrays.
    const TENSOR_ARENA_SIZE: usize = 60 * 1024;
    let mut tensor_arena: [u8; TENSOR_ARENA_SIZE] = [0; TENSOR_ARENA_SIZE];

    // Pull in all needed operation implementations
    let micro_op_resolver = MutableOpResolver::empty()
        .depthwise_conv_2d()
        .max_pool_2d()
        .conv_2d()
        .fully_connected()
        .softmax();

    // Build an interpreter to run the model with
    let mut interpreter =
        MicroInterpreter::new(&model, micro_op_resolver, &mut tensor_arena[..]).unwrap();

    // Check properties of the input sensor
    assert_eq!([1, 128, 3, 1], interpreter.input_info(0).dims);

    // 128 samples of * (x,y,z)
    let mut data = [0.0; 384];

    loop {
        //lights dont work on debug builds
        neopixel
            .write(brightness(
                [
                    colors::GREEN,
                    colors::GREEN,
                    colors::GREEN,
                    colors::GREEN,
                    colors::GREEN,
                ]
                .iter()
                .cloned(),
                1,
            ))
            .unwrap();

        (0..128).for_each(|n| {
            while !lis3dh.is_data_ready().unwrap() {}
            let dat = lis3dh.accel_raw().unwrap();

            // tfmicro test data is normalized to 1mg per digit
            // adafruit *9.80665/9.8 so.. dont do that
            // adafruit * by 1000/8190 or 0.122100122, or divide by 8, or shift 3
            // even though should be >> 1 because theyre in high res though they
            // dont know it
            let x = (dat[0] >> 3) as f32;
            let y = (dat[1] >> 3) as f32;
            let z = (dat[2] >> 3) as f32;

            data[n * 3] = -z;
            data[n * 3 + 1] = -x;
            data[n * 3 + 2] = y;
        });

        #[cfg(feature = "train")]
        dbgprintln("-,-,-");
        #[cfg(feature = "train")]
        for reading in data.chunks_exact(3) {
            dbgprint!("{:04.2?},{}:04.2?},{}:04.2?}", &data[..]);
        }

        // dbgprint!("{:04.2?}", &data[..]);

        #[cfg(not(any(feature = "test_ring", feature = "test_slope")))]
        interpreter.input(0, &data).unwrap();

        #[cfg(any(feature = "test_ring", feature = "test_slope"))]
        interpreter.input(0, &test).unwrap();

        interpreter.invoke().unwrap();

        let output_tensor = interpreter.output(0);
        assert_eq!([1, 4], output_tensor.info().dims);

        // dbgprint!("{:.4?}", output_tensor.as_data::<f32>());

        let res = output_tensor.as_data::<f32>();

        // 0 WingScore
        // 1 RingScore
        // 2 SlopeScore
        // 3 NegativeScore
        let color = if res[0] > res[1] && res[0] > res[2] && res[0] > res[3] {
            colors::YELLOW
        } else if res[1] > res[0] && res[1] > res[2] && res[1] > res[3] {
            colors::PURPLE
        } else if res[2] > res[0] && res[2] > res[1] && res[2] > res[3] {
            colors::BLUE
        } else {
            RGB8::default()
        };

        neopixel
            .write(brightness(
                [color, color, color, color, color].iter().cloned(),
                1,
            ))
            .unwrap();
        delay.delay_ms(1000_u32);
        delay.delay_ms(1000_u32);
        delay.delay_ms(1000_u32);
    }
}
