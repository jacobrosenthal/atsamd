//! Dump the GD25Q16C 2MiB qspi flash sector. Requires RTT and debugger to view
//! results.
//! `JLinkGDBServer -if SWD -device ATSAMD51J19a -nogui` and `telnet localhost 19021`
//!
//! | device  | block   | sector   | page    | unit    |
//! |---------|---------|----------|---------|---------|
//! | 2097152 | 64/32K  | 4096     | 256     | bytes   |
//! | 8192    | 256/128 | 16       | -       | pages   |
//! | 512     | 16/8    | -        | -       | sectors |
//! | 32/64   | -       | -        | -       | blocks  |
//! http://www.gigadevice.com/datasheet/gd25q16c/

#![no_std]
#![no_main]

// const SECTOR_COUNT: u32 = 512;
const SECTOR_SIZE: usize = 4096;

use edgebadge as hal;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;
use hal::qspi::Command;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!(BlockIfFull, 128);

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

    let mut sets = hal::Pins::new(peripherals.PORT).split();

    let mut flash = sets
        .flash
        .init(&mut peripherals.MCLK, &mut sets.port, peripherals.QSPI);

    // Startup delay. Can't find documented but Adafruit use 5ms
    delay.delay_ms(5u8);
    // Reset. It is recommended to check the BUSY(WIP?) bit and the SUS before reset
    wait_ready(&mut flash);
    flash.run_command(Command::EnableReset).unwrap();
    flash.run_command(Command::Reset).unwrap();
    // tRST(30μs) to reset. During this period, no command will be accepted
    delay.delay_ms(1u8);

    // Check for GD25Q16C JEDEC ID
    let mut read_buf = [0u8; 3];
    flash.read_command(Command::ReadId, &mut read_buf).unwrap();
    assert_eq!(read_buf, [0x15, 0x40, 0xc8]);

    // 120MHz / 2 = 60mhz
    // faster than 104mhz at 3.3v would require High Performance Mode
    flash.set_clk_divider(2);

    // Enable Quad SPI mode. Requires write enable. Check WIP.
    flash.run_command(Command::WriteEnable).unwrap();
    flash
        .write_command(Command::WriteStatus, &[0x00, 0x02])
        .unwrap();
    wait_ready(&mut flash);

    // TODO QE just will not take
    rprintln!("{:02X?}", flash_status(&mut flash, Command::ReadStatus));
    rprintln!("{:02X?}", flash_status(&mut flash, Command::ReadStatus2));

    // Read back data
    // datasheet claims 6BH needs a single dummy byte, but doesnt work then?!
    // adafruit uses 8, and the underlying implementation uses 8 atm as well
    let mut read_buf = [0u8; SECTOR_SIZE];
    // for i in 0..SECTOR_COUNT {
    flash.read_memory(0 * SECTOR_SIZE as u32, &mut read_buf);
    rprintln!("{:02X?}", read_buf);
    // }

    // presumably as a consequence of no QE bit we just get back all EE
    // [EE, EE, EE, EE, EE, EE, EE, ...]

    loop {
        cortex_m::asm::nop();
    }
}

/// Wait for the write-in-progress and suspended write/erase.
fn wait_ready(flash: &mut hal::qspi::Qspi) {
    while flash_status(flash, Command::ReadStatus) & 0x01 != 0 {}
    while flash_status(flash, Command::ReadStatus2) & 0x80 != 0 {}
}

/// Returns the contents of the status register indicated by cmd.
fn flash_status(flash: &mut hal::qspi::Qspi, cmd: Command) -> u8 {
    let mut out = [0u8; 1];
    flash.read_command(cmd, &mut out).ok().unwrap();
    out[0]
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    cortex_m::asm::udf()
}
