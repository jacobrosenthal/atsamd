//! Blink an led without using the BSP split() method.

#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_halt;
use pygamer as hal;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::entry;
use hal::pac::{interrupt, CorePeripherals, Peripherals};
use hal::prelude::*;

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
    let mut delay = Delay::new(core.SYST, &mut clocks);
    delay.delay_ms(400u16);

    let mut pins = hal::Pins::new(peripherals.PORT);

    let gclk0 = clocks.gclk0();
    &clocks.eic(&gclk0).unwrap();

    peripherals.EIC.ctrla.modify(|_, w| w.swrst().set_bit());
    while peripherals.EIC.syncbusy.read().swrst().bit_is_set() {
        cortex_m::asm::nop();
    }
    peripherals.MCLK.apbamask.modify(|_, w| w.eic_().set_bit());
    peripherals.EIC.ctrla.modify(|_, w| w.cksel().set_bit());
    peripherals.EIC.config[0].modify(|_, w| w.sense2().high().filten2().set_bit());
    peripherals
        .EIC
        .intenset
        .write(|w| unsafe { w.extint().bits(0x04) });
    peripherals
        .EIC
        .evctrl
        .write(|w| unsafe { w.extinteo().bits(0x04) });
    peripherals.EIC.ctrla.modify(|_, w| w.enable().set_bit());

    // a1 is pa5 is EIC/EXTINT[5]
    let _external_interrupt: hal::gpio::Pa5<hal::gpio::PfA> = pins
        .a1
        .into_pull_down_input(&mut pins.port)
        .into_function_a(&mut pins.port);

    while peripherals.EIC.syncbusy.read().swrst().bit_is_set() {
        cortex_m::asm::nop();
    }

    unsafe {
        core.NVIC.set_priority(interrupt::EIC_EXTINT_5, 1);
        hal::pac::NVIC::unmask(interrupt::EIC_EXTINT_5);
    }

    loop {
        delay.delay_ms(200u8);
    }
}
