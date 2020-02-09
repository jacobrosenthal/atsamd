# Adafruit PyGamer Board Support Crate

This crate provides a type-safe API for working with the [Adafruit PyGamer
board](https://www.adafruit.com/product/4242).

## Examples?

Check out the repository for examples:

https://github.com/atsamd-rs/atsamd/tree/master/boards/pygamer/examples

## debugging

Youll need to look up your debug pins, probably labeled swd, which will require 2 programming pins, a power and gnd. Often you'll have to dig through board resources like eagle files to find these. In addition note you may need to solder wires to gain access to these.  Note youll almost certainly still need to provide power via a battery or usb connection. 

### jlink
Install the [jlink software and documentation pack](https://www.segger.com/downloads/jlink/) for your operating system.

To use jlink as a gdb server run `JLinkGDBServer -if SWD -device ATSAMD51J19a -nogui` Note new cheaper jlink devices push a pop up once a day at least you may need to click accept on. If it works you'll see a bunch of chip information, your target voltage, and waiting for a connection. 
```
Target voltage: 3.30 V
Listening on TCP/IP port 2331
Connecting to target...
Connected to target
Waiting for GDB connection...
```

In your code you can panic via rtt using the `use_rtt` feature and then bringing in that crate
```rust
use panic_rtt as _;
```

and or use jlink_rtt to console log via rtt
```rust
use jlink_rtt;
```
```rust
	let mut output = jlink_rtt::Output::new();
	let _ = writeln!("Hello {}", 42);
```

Both of which will output on the telnet which you can see by keeping another window open with `telnet localhost 19021`
```
$ telnet localhost 19021
Trying ::1...
telnet: connect to address ::1: Connection refused
Trying 127.0.0.1...
Connected to localhost.
Escape character is '^]'.
SEGGER J-Link V6.61c (beta) - Real time terminal output
J-Link EDU Mini V1 compiled Jan  7 2020 16:53:19 V1.0, SN=801001259
Process: JLinkGDBServerCLExe
```
Note, to exit from telnet by usuing control and ] and typing quit.

Finally you build your code. The included .cargo/config runner will build and pass the elf file onto the gdb server when you use `cargo run`, and the included .gdbinit will reset *after* loading so that we run past the bootloader as well.
