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
//! * `cargo +nightly hf2 --release --example magic_wand --features="tf"`
//!
//! Try Test data to confirm model works
//! * `cargo +nightly hf2 --release --example magic_wand --features="tf_slope"`
//! Or
//! * `cargo +nightly hf2 --release --example magic_wand --features="tf_ring"`
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
//! * `cargo +nightly run --release --example magic_wand --features="tf_train"`
//!
//! Note you dont want to use use_rtt feature and thus cant rprint or train
//! unless you have a debugger attached which you probably dont

#![no_std]
#![no_main]

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
use rtt_target::{rprint, rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    #[cfg(feature = "use_rtt")]
    rtt_init_print!();

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

    let model = include_bytes!("../assets/models/magic_wand_edgebadge.tflite");

    #[cfg(feature = "tf_ring")]
    let test = include_bytes!("../assets/models/ring_micro_f9643d42_nohash_4.data")
        .chunks_exact(4)
        .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
        .collect::<heapless::Vec<_, heapless::consts::U384>>();

    #[cfg(feature = "tf_slope")]
    let test = include_bytes!("../assets/models/slope_micro_f2e59fea_nohash_1.data")
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

    #[cfg(not(feature = "tf_train"))]
    const N: usize = 128;

    #[cfg(feature = "tf_train")]
    const N: usize = 64;

    //  (x,y,z)
    let mut data = [0.0; N * 3];

    loop {
        #[cfg(all(feature = "use_rtt", not(feature = "tf_train")))]
        rprintln!("Magic Starts!");

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

        #[cfg(feature = "tf_train")]
        let color = {
            rprint!("-,-,-");
            for reading in data.chunks_exact(3) {
                rprint!(
                    "{:04.1?},{:04.1?},{:04.1?}",
                    reading[0],
                    reading[1],
                    reading[2]
                );
            }
            RGB8::default()
        };

        #[cfg(not(feature = "tf_train"))]
        let color = {
            #[cfg(feature = "use_rtt")]
            rprint!("{:04.1?}", &data[..]);

            #[cfg(not(any(feature = "tf_ring", feature = "tf_slope")))]
            interpreter.input(0, &data).unwrap();

            #[cfg(any(feature = "tf_ring", feature = "tf_slope"))]
            interpreter.input(0, &test).unwrap();

            interpreter.invoke().unwrap();

            let output_tensor = interpreter.output(0);
            assert_eq!([1, 4], output_tensor.info().dims);

            let res = output_tensor.as_data::<f32>();

            #[cfg(feature = "use_rtt")]
            rprint!("{:.4?}", res);

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

            color
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

// call udf (hardfault handler)
#[inline(never)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        cortex_m::asm::udf();
    }
}

// certain optimization levels cause hardfault in the prebuilt binary
// hardfault is called by udf, which means our led wont light if we light it there..
// so..  this avoids our led from being lit by panic handler...
#[inline(never)]
#[cortex_m_rt::exception]
fn HardFault(_ef: &cortex_m_rt::ExceptionFrame) -> ! {
    use embedded_hal::digital::v2::OutputPin;

    let peripherals = unsafe { hal::pac::Peripherals::steal() };
    let mut pins = hal::Pins::new(peripherals.PORT);
    let _ = pins.d13.into_open_drain_output(&mut pins.port).set_high();

    loop {}
}
