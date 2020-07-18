//! Port of Tensorflow Gesture Demo
//! https://learn.adafruit.com/tensorflow-lite-for-edgebadge-kit-quickstart/gesture-demo
//!
//! With the screen facing you, and the USB port pointing to the ceiling perform
//! one of three gestures:
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
//!
//! Upload:
//! * `cargo +nightly hf2 --release --example magic_wand --features="tensorflow"`
//!
//! Try Test data to confirm model works
//! * `cargo +nightly hf2 --release --example magic_wand --features="tensorflow, test_slope"`
//! Or
//! * `cargo +nightly hf2 --release --example magic_wand --features="tensorflow, test_ring"`
//!
//! Note seems too highly tuned to original data somehow. I needed to add a set
//! of training data from our device to get any decent results...As a result the
//! included model is NOT the upstream model but theirs with my single set of
//! data added
//!
//! Note if no lights seem to work, turn the board over and check the d13 led
//! which indicates the device is panicing somewhere. Sadly it can't tell you
//! which line.
//!
//! Get training Data over rtt:
//! * `cargo +nightly run --release --example magic_wand --features="tensorflow,use_rtt,train"`
//!
//! Note you dont want to use use_rtt feature and thus cant dbgprint or train
//! unless you have a debugger attached which you probably dont

#![no_std]
#![no_main]

#[cfg(feature = "use_rtt")]
use panic_rtt as _;
#[cfg(not(feature = "use_rtt"))]
use pygamer_panic_led as _;

use pygamer as hal;

use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;
use hal::time::KiloHertz;
use hal::timer::SpinTimer;
use hal::{clock::GenericClockController, delay::Delay};
use lis3dh::{accelerometer::RawAccelerometer, Lis3dh, SlaveAddr};
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

    let mut lis3dh = Lis3dh::new(i2c, SlaveAddr::Alternate).unwrap();
    lis3dh.set_mode(lis3dh::Mode::HighResolution).unwrap();
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

    #[cfg(not(feature = "train"))]
    const N: usize = 128;

    #[cfg(feature = "train")]
    const N: usize = 64;

    //  (x,y,z)
    let mut data = [0.0; N * 3];

    loop {
        #[cfg(not(feature = "train"))]
        dbgprint!("Magic Starts!");

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

        (0..N).for_each(|n| {
            while !lis3dh.is_data_ready().unwrap() {}
            let dat = lis3dh.accel_raw().unwrap();

            // test data is normalized to 1mg per digit
            // shift to justify, .002 scale, *1000 for to mg
            let x = (dat[0] >> 4) as f32 * 2.0;
            let y = (dat[1] >> 4) as f32 * 2.0;
            let z = (dat[2] >> 4) as f32 * 2.0;

            // invert and move around for our board orientation
            data[n * 3] = -z;
            data[n * 3 + 1] = -x;
            data[n * 3 + 2] = y;
        });

        #[cfg(feature = "train")]
        dbgprint!("-,-,-");
        #[cfg(feature = "train")]
        for reading in data.chunks_exact(3) {
            dbgprint!(
                "{:04.1?},{:04.1?},{:04.1?}",
                reading[0],
                reading[1],
                reading[2]
            );
        }

        #[cfg(not(feature = "train"))]
        dbgprint!("{:04.1?}", &data[..]);

        #[cfg(not(feature = "train"))]
        {
            #[cfg(not(any(feature = "test_ring", feature = "test_slope")))]
            interpreter.input(0, &data).unwrap();

            #[cfg(any(feature = "test_ring", feature = "test_slope"))]
            interpreter.input(0, &test).unwrap();

            interpreter.invoke().unwrap();
        }

        let output_tensor = interpreter.output(0);
        assert_eq!([1, 4], output_tensor.info().dims);

        let res = output_tensor.as_data::<f32>();

        #[cfg(not(feature = "train"))]
        dbgprint!("{:.4?}", res);

        // 0 WingScore
        // 1 RingScore
        // 2 SlopeScore
        // 3 NegativeScore
        let color = if res[0] > 0.5 {
            colors::YELLOW
        } else if res[1] > 0.5 {
            colors::PURPLE
        } else if res[2] > 0.5 {
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
