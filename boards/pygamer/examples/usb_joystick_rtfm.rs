#![no_std]
#![no_main]

use panic_halt as _;
use pygamer as hal;

use hal::adc::Adc;
use hal::clock::GenericClockController;
use hal::entry;
use hal::pac::gclk::pchctrl::GEN_A::GCLK11;
use hal::pac::{interrupt, CorePeripherals, Peripherals};
use hal::prelude::*;

use hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_hid::descriptor::{JoystickReport, SerializedDescriptor};
use usbd_hid::hid_class::HIDClass;

use cortex_m::asm::delay as cycle_delay;
use cortex_m::interrupt::free as disable_interrupts;
use cortex_m::peripheral::NVIC;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );

    let mut pins = hal::Pins::new(peripherals.PORT).split();
    let mut red_led = pins.led_pin.into_open_drain_output(&mut pins.port);
    red_led.set_low().unwrap();

    let mut buttons = pins.buttons.init(&mut pins.port);
    let mut adc1 = Adc::adc1(peripherals.ADC1, &mut peripherals.MCLK, &mut clocks, GCLK11);
    let mut joystick = pins.joystick.init(&mut pins.port);

    let bus_allocator = unsafe {
        USB_ALLOCATOR = Some(pins.usb.init(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.MCLK,
            &mut pins.port,
        ));
        USB_ALLOCATOR.as_ref().unwrap()
    };

    unsafe {
        USB_HID = Some(HIDClass::new(&bus_allocator, JoystickReport::desc(), 60));
        USB_BUS = Some(
            UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Fake company")
                .product("Twitchy Mousey")
                .serial_number("TEST")
                .device_class(0x3) // HID (mice, keyboards, joysticks, gamepads)
                .build(),
        );
    }

    unsafe {
        core.NVIC.set_priority(interrupt::USB_OTHER, 1);
        core.NVIC.set_priority(interrupt::USB_TRCPT0, 1);
        core.NVIC.set_priority(interrupt::USB_TRCPT1, 1);
        NVIC::unmask(interrupt::USB_OTHER);
        NVIC::unmask(interrupt::USB_TRCPT0);
        NVIC::unmask(interrupt::USB_TRCPT1);
    }

    loop {
        let (x, y) = joystick.read(&mut adc1);
        //four buttons
        let buttons = buttons.mask();

        cycle_delay(25 * 1024 * 1024);
        push_mouse_movement(JoystickReport {
            x: x as i8,
            y: y as i8,
            buttons,
        })
        .ok()
        .unwrap_or(0);
    }
}

fn push_mouse_movement(report: JoystickReport) -> Result<usize, usb_device::UsbError> {
    disable_interrupts(|_| unsafe { USB_HID.as_mut().map(|hid| hid.push_input(&report)) }).unwrap()
}

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut USB_HID: Option<HIDClass<UsbBus>> = None;

fn poll_usb() {
    unsafe {
        USB_BUS.as_mut().map(|usb_dev| {
            USB_HID.as_mut().map(|hid| {
                usb_dev.poll(&mut [hid]);
            });
        });
    };
}

#[interrupt]
fn USB_OTHER() {
    poll_usb();
}

#[interrupt]
fn USB_TRCPT0() {
    poll_usb();
}

#[interrupt]
fn USB_TRCPT1() {
    poll_usb();
}
