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
//! Get training Data over rtt:
//! * `cargo +nightly run --release --example magic_wand --features="tensorflow,use_rtt,train"`
//!
//! Note you dont want to use use_rtt feature and thus cant dbgprint or train
//! unless you have a debugger attached which you probably dont
//!
//! Note accelerometer seems to get stuck uploading. Until a solution is found,
//! if the device doesn't use panic led, and unplug or toggle power switch.

#![no_std]
#![no_main]

use pygamer as hal;

use cortex_m::peripheral::DWT;
use embedded_hal::{digital::v1_compat::OldOutputPin, timer::CountDown};
use hal::gpio;
use hal::prelude::*;
use hal::sercom::{I2CMaster2, Sercom2Pad0, Sercom2Pad1};
use hal::time::KiloHertz;
use hal::usb::UsbBus;
use hal::{clock::GenericClockController, timer::TimerCounter};
use lis3dh::{accelerometer::RawAccelerometer, Lis3dh, SlaveAddr};
use rtic::cyccnt::Duration;
use smart_leds::{brightness, colors, hsv::RGB8, SmartLedsWrite};
use ufmt::uwriteln;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use ws2812_timer_delay as ws2812;

#[cfg(not(feature = "train"))]
const N: usize = 128;

#[cfg(feature = "train")]
const N: usize = 64;

#[rtic::app(device = crate::hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        usb_bus: &'static UsbBusAllocator<UsbBus>,
        usb_serial: SerialPort<'static, UsbBus>,
        usb_device: UsbDevice<'static, UsbBus>,

        neopixel: NeopixelType,
        lis3dh: Lis3dhType,
    }

    #[init]
    fn init(mut c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

        let mut peripherals = c.device;
        let mut clocks = GenericClockController::with_internal_32kosc(
            peripherals.GCLK,
            &mut peripherals.MCLK,
            &mut peripherals.OSC32KCTRL,
            &mut peripherals.OSCCTRL,
            &mut peripherals.NVMCTRL,
        );

        let mut pins = hal::Pins::new(peripherals.PORT).split();

        let gclk0 = clocks.gclk0();
        let timer_clock = clocks.tc2_tc3(&gclk0).unwrap();
        let mut timer = TimerCounter::tc3_(&timer_clock, peripherals.TC3, &mut peripherals.MCLK);
        timer.start(3_000_000u32.hz());

        let neopixel = pins.neopixel.init(timer, &mut pins.port);

        // Initialize (enable) the monotonic timer (CYCCNT)
        c.core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
        DWT::unlock();
        c.core.DWT.enable_cycle_counter();

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

        *USB_BUS = Some(pins.usb.init(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.MCLK,
            &mut pins.port,
        ));

        let usb_serial = SerialPort::new(USB_BUS.as_ref().unwrap());

        let usb_device =
            UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Fake company")
                .product("Serial port")
                .serial_number("TEST")
                .device_class(USB_CLASS_CDC)
                .build();

        // c.spawn.foo().ok();

        init::LateResources {
            usb_bus: USB_BUS.as_ref().unwrap(),
            usb_serial,
            usb_device,
            neopixel,
            lis3dh,
        }
    }

    //neopixels and inference are blocking so we need to run them at a low
    //priority. Were not doing an power saving in idle, so well use that instead
    //of scheduling a task
    #[idle(resources = [neopixel, lis3dh, usb_serial])]
    fn main(c: main::Context) -> ! {
        loop {
            let neopixel = c.resources.neopixel;
            let lis3dh = c.resources.lis3dh;
            // let mut usb_serial = &mut c.resources.usb_serial;

            let mut data = [0.0; N * 3];

            #[cfg(not(feature = "train"))]
            c.resources
                .usb_serial
                .write("Magic Starts!\r".as_bytes())
                .ok();

            // write(&mut c.resources.usb_serial);

            // // let _ = usb_serial.write("Magic Starts!\r".as_bytes());
            // let _ = usb_serial.write(&[0x3a, 0x29]);

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

            // #[cfg(feature = "train")]
            // let _ = usb_serial.write("-,-,-\r".as_bytes());

            // #[cfg(feature = "train")]
            // for reading in data.chunks_exact(3) {
            //     uwriteln!(DirtyWriter(c.resources.usb_serial), "-,-,-\r").unwrap();

            //     fmt!(
            //         "{:04.1?},{:04.1?},{:04.1?}",
            //         reading[0],
            //         reading[1],
            //         reading[2]
            //     );
            // }

            // #[cfg(not(feature = "train"))]
            // dbgprint!("{:04.1?}", &data[..]);

            // #[cfg(not(feature = "train"))]
            // {
            //     #[cfg(not(any(feature = "test_ring", feature = "test_slope")))]
            //     interpreter.input(0, &data).unwrap();

            //     #[cfg(any(feature = "test_ring", feature = "test_slope"))]
            //     interpreter.input(0, &test).unwrap();

            //     interpreter.invoke().unwrap();
            // }

            // let output_tensor = interpreter.output(0);
            // assert_eq!([1, 4], output_tensor.info().dims);

            // let res = output_tensor.as_data::<f32>();
            let res = [0.0, 0.98, 0.0, 0.0];

            // #[cfg(not(feature = "train"))]
            // dbgprint!("{:.4?}", res);
            // uwriteln!(DirtyWriter(c.resources.usb_serial), "-,-,-\r").unwrap();

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
        }
    }

    // #[task(capacity = 2 ,resources = [neopixel, lis3dh, usb_serial], schedule = [foo])]
    // fn foo(c: foo::Context) {

    //     // only update every 3 second. for pygamer at 120mhz that is is 375000000
    //     c.schedule
    //         .foo(c.scheduled + Duration::from_cycles(375000000))
    //         .ok();
    // }
    #[task(binds = USB_OTHER, resources = [usb_device, usb_serial])]
    fn usb_other(cx: usb_other::Context) {
        usb_poll(cx.resources.usb_device, cx.resources.usb_serial);
    }

    #[task(binds = USB_TRCPT0, resources = [usb_device, usb_serial])]
    fn usb_trcpt0(cx: usb_trcpt0::Context) {
        usb_poll(cx.resources.usb_device, cx.resources.usb_serial);
    }

    #[task(binds = USB_TRCPT1, resources = [usb_device, usb_serial])]
    fn usb_trcpt1(cx: usb_trcpt1::Context) {
        usb_poll(cx.resources.usb_device, cx.resources.usb_serial);
    }

    // RTIC requires that unused interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks. Chosen randomly, feel free to replace.
    extern "C" {
        fn SDHC0();
    }
};

// throw away incoming
fn usb_poll<B: usb_device::bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>,
) {
    if !usb_dev.poll(&mut [serial]) {
        return;
    }
    let mut buf = [0; 10];
    match serial.read(&mut buf) {
        Ok(_) => {}
        Err(UsbError::WouldBlock) => {}
        e => panic!("USB read error: {:?}", e),
    }
}

type NeopixelType = ws2812::Ws2812<
    hal::timer::TimerCounter3,
    OldOutputPin<gpio::Pa15<gpio::Output<gpio::PushPull>>>,
>;

type Lis3dhType =
    Lis3dh<I2CMaster2<Sercom2Pad0<gpio::Pa12<gpio::PfC>>, Sercom2Pad1<gpio::Pa13<gpio::PfC>>>>;

#[inline(never)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        //pin d13 a23
        let pin_no = 23;
        let variant = 0;
        let pinmux = &(*hal::pac::PORT::ptr()).group0.pmux;
        let pincfg = &(*hal::pac::PORT::ptr()).group0.pincfg;
        let dirset = &(*hal::pac::PORT::ptr()).group0.dirset;
        let outset = &(*hal::pac::PORT::ptr()).group0.outset;

        //into_function_a
        pinmux[pin_no >> 1].modify(|_, w| {
            if pin_no & 1 == 1 {
                // Odd-numbered pin
                w.pmuxo().bits(variant)
            } else {
                // Even-numbered pin
                w.pmuxe().bits(variant)
            }
        });

        pincfg[pin_no].modify(|_, bits| bits.pmuxen().set_bit());

        //into_push_pull
        dirset.write(|bits| {
            bits.bits(1 << pin_no);
            bits
        });

        pincfg[pin_no].write(|bits| {
            bits.pmuxen().clear_bit();
            bits.inen().set_bit();
            bits.pullen().clear_bit();
            bits.drvstr().clear_bit();
            bits
        });

        //set_high
        outset.write(|bits| {
            bits.bits(1 << pin_no);
            bits
        });
    }

    loop {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

// pub struct DirtyWriter<'a, B: 'static + usb_device::bus::UsbBus>(&'a mut SerialPort<'static, B>);

// impl<'a, B: usb_device::bus::UsbBus> ufmt::uWrite for DirtyWriter<'a, B> {
//     type Error = usb_device::UsbError;
//     fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
//         match self.0.write(&s.as_bytes()) {
//             Ok(_) => Ok(()),
//             Err(UsbError::WouldBlock) => Ok(()),
//             Err(e) => Err(e),
//         }
//     }
// }

// fn write<B: usb_device::bus::UsbBus>(serial: &mut resources::usb_serial) {
//     serial.write("Magic Starts!\r".as_bytes()).ok();
// }
