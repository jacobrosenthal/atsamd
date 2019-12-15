#![no_main]
#![no_std]

// matti@miya:/etc/udev/rules.d$ cat 99-usb-test.rules
// SUBSYSTEMS=="usb", ATTR{idVendor}=="16c0", ATTR{idProduct}=="27d8",
// MODE:="0666"

#[allow(unused_imports)]
use panic_halt;
use pygamer as hal;

use embedded_hal::digital::v2::OutputPin;
use hal::clock::GenericClockController;
use hal::gpio::{OpenDrain, Output, Pa23};
use rtfm::app;
use usb_device::bus;
use usb_device::prelude::*;
use usbd_webusb::WebUsb;

use hal::usb::UsbBus;

mod blinky {
    use core::marker::PhantomData;
    use embedded_hal::digital::v2::OutputPin;
    use usb_device::class_prelude::*;

    pub struct BlinkyClass<B: UsbBus, LED> {
        spooky: core::marker::PhantomData<B>,
        led: LED,
    }

    impl<B: UsbBus, LED: OutputPin> BlinkyClass<B, LED> {
        pub fn new(_alloc: &UsbBusAllocator<B>, led: LED) -> Self {
            Self {
                spooky: PhantomData,
                led,
            }
        }
    }

    impl<B: UsbBus, LED: OutputPin> UsbClass<B> for BlinkyClass<B, LED> {
        fn control_out(&mut self, xfer: ControlOut<B>) {
            let req = xfer.request();

            if req.request_type == control::RequestType::Vendor
                && req.recipient == control::Recipient::Device
                && req.request == 1
            {
                if req.value > 0 {
                    self.led.set_low().ok();
                } else {
                    self.led.set_high().ok();
                }
            }
        }
    }
}

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice<'static, UsbBus>,
        blinky: blinky::BlinkyClass<UsbBus, Pa23<Output<OpenDrain>>>,
        webusb: WebUsb<UsbBus>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBus>> = None;

        let mut peripherals = cx.device;
        let mut clocks = GenericClockController::with_internal_32kosc(
            peripherals.GCLK,
            &mut peripherals.MCLK,
            &mut peripherals.OSC32KCTRL,
            &mut peripherals.OSCCTRL,
            &mut peripherals.NVMCTRL,
        );

        let mut pins = hal::Pins::new(peripherals.PORT).split();

        let mut led = pins.led_pin.into_open_drain_output(&mut pins.port);
        led.set_high().ok();

        *USB_BUS = Some(pins.usb.init(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.MCLK,
            &mut pins.port,
        ));

        let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27d8))
            .manufacturer("Fake Company")
            .product("Web Blinky")
            .serial_number("TEST")
            .build();

        init::LateResources {
            usb_dev,
            blinky: blinky::BlinkyClass::new(USB_BUS.as_ref().unwrap(), led),
            webusb: WebUsb::new(
                USB_BUS.as_ref().unwrap(),
                usbd_webusb::url_scheme::HTTPS,
                "virkkunen.net/b/blinky.html",
            ),
        }
    }

    //todo dry all three of these up into a sigle bind?
    #[task(binds = USB_OTHER, resources = [usb_dev, webusb, blinky])]
    fn usb_other(cx: usb_other::Context) {
        cx.resources
            .usb_dev
            .poll(&mut [cx.resources.webusb, cx.resources.blinky]);
    }

    #[task(binds = USB_TRCPT0, resources = [usb_dev, webusb, blinky])]
    fn usb_trcpt0(cx: usb_trcpt0::Context) {
        cx.resources
            .usb_dev
            .poll(&mut [cx.resources.webusb, cx.resources.blinky]);
    }

    #[task(binds = USB_TRCPT1, resources = [usb_dev, webusb, blinky])]
    fn usb_trcpt1(cx: usb_trcpt1::Context) {
        cx.resources
            .usb_dev
            .poll(&mut [cx.resources.webusb, cx.resources.blinky]);
    }
};
