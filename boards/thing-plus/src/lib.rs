#![no_std]
#![recursion_limit = "1024"]

#[cfg(feature = "rt")]
pub use cortex_m_rt::entry;

pub use atsamd_hal as hal;
pub use hal::pac;

pub mod pins;
pub use pins::*;

#[cfg(feature = "panic_led")]
#[inline(never)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use embedded_hal::digital::v2::OutputPin;

    let peripherals = unsafe { crate::pac::Peripherals::steal() };
    let mut pins = Pins::new(peripherals.PORT);
    pins.d13.into_push_pull_output().set_high().ok();

    loop {
        cortex_m::asm::udf()
    }
}
