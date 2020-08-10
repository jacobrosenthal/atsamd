//! Blink an led without using the BSP split() method.

#![no_std]
#![no_main]

use pygamer as hal;

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};
use hal::clock::GenericClockController;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let _core = CorePeripherals::take().unwrap();
    let mut _clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );

    // panic!("uncomment should see the red led");
    loop {
        continue;
    }
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
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
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
