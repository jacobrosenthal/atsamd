//! Blink an led without using the BSP split() method.

#![no_std]
#![no_main]

use edgebadge::{self as hal, entry, pac, Pins};
use panic_halt as _;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::watchdog::{Watchdog, WatchdogTimeout};
use pac::{CorePeripherals, Peripherals};

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
    let mut delay = Delay::new(core.SYST, &mut clocks);
    delay.delay_ms(400u16);

    let mut pins = Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    let mut wdt = Watchdog::new(peripherals.WDT);
    wdt.start(WatchdogTimeout::Cycles256 as u8);

    loop {
        delay.delay_ms(200u8);
        wdt.feed();
        red_led.set_high().unwrap();
        delay.delay_ms(200u8);
        wdt.feed();
        red_led.set_low().unwrap();
    }
}
