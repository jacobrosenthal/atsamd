#![no_std]
#![cfg_attr(feature = "usb", feature(align_offset, ptr_offset_from))]

pub extern crate embedded_hal as hal;

pub use paste;

#[cfg(feature = "samd21e18a")]
pub use atsamd21e18a as target_device;

#[cfg(feature = "samd21g18a")]
pub use atsamd21g18a as target_device;

#[cfg(feature = "samd21j18a")]
pub use atsamd21j18a as target_device;

#[cfg(feature = "samd51g19a")]
pub use atsamd51g19a as target_device;

#[cfg(feature = "samd51j19a")]
pub use atsamd51j19a as target_device;

#[cfg(feature = "samd51j20a")]
pub use atsamd51j20a as target_device;

#[cfg(feature = "use_rtt")]
pub use jlink_rtt;

#[cfg(feature = "use_rtt")]
#[macro_export]
macro_rules! dbgprint {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let mut out = $crate::jlink_rtt::NonBlockingOutput::new();
            writeln!(out, $($arg)*).ok();
        }
    };
}

#[cfg(not(feature = "use_rtt"))]
#[macro_export]
macro_rules! dbgprint {
    ($($arg:tt)*) => {{}};
}

#[macro_use]
pub mod common;
pub use self::common::*;

#[cfg(feature="samd51")] 
pub mod samd51;
#[cfg(feature="samd51")] 
pub use self::samd51::*;

#[cfg(not(feature="samd51"))]
pub mod samd21;
#[cfg(not(feature="samd51"))]
pub use self::samd21::*;

#[cfg(feature = "usb")]
pub use self::samd21::usb;
