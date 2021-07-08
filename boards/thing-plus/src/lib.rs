#![no_std]
#![recursion_limit = "1024"]

pub mod pins;
use atsamd_hal as hal;

#[cfg(feature = "rt")]
pub use cortex_m_rt::entry;

pub use pins::Pins;

use hal::*;

pub use hal::common::*;
pub use hal::samd51::*;
pub use hal::target_device as pac;

#[cfg(feature = "panic_led")]
#[inline(never)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use embedded_hal::digital::v2::OutputPin;

    let peripherals = unsafe { crate::pac::Peripherals::steal() };
    let mut pins = Pins::new(peripherals.PORT);
    let _ = pins.d13.into_open_drain_output(&mut pins.port).set_high();

    loop {
        cortex_m::asm::udf()
    }
}
