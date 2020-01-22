//! Makes the pygamer appear as a USB serial port loop back device.
//! Repeats back all characters sent to it, but in upper case.

#![no_std]
#![no_main]

use hal::clock::GenericClockController;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
use hal::pins::Keys;
use panic_halt as _;
use pygamer as hal;
use usb_device::prelude::*;
use usbd_midi::{
    data::{
        byte::u7::U7,
        midi::{channel::Channel, message::Message as MidiMessage, notes::Note},
        usb::constants::USB_CLASS_NONE,
        usb_midi::{cable_number::CableNumber, usb_midi_event_packet::UsbMidiEventPacket},
    },
    midi_device::MidiClass,
};

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut _core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );

    let mut pins = hal::Pins::new(peripherals.PORT).split();

    let usb_bus = pins.usb.init(
        peripherals.USB,
        &mut clocks,
        &mut peripherals.MCLK,
        &mut pins.port,
    );

    let mut midi = MidiClass::new(&usb_bus);
    let mut buttons = pins.buttons.init(&mut pins.port);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Midi Controller")
        .serial_number("1")
        .device_class(USB_CLASS_NONE)
        .build();

    loop {
        let _ = usb_dev.poll(&mut [&mut midi]);

        for message in buttons.events().map(|button| match button {
            Keys::SelectDown => MidiMessage::NoteOn(Channel::Channel1, Note::C3, U7::MAX),
            Keys::SelectUp => MidiMessage::NoteOff(Channel::Channel1, Note::C3, U7::MAX),
            Keys::StartDown => MidiMessage::NoteOn(Channel::Channel1, Note::D3, U7::MAX),
            Keys::StartUp => MidiMessage::NoteOff(Channel::Channel1, Note::D3, U7::MAX),
            Keys::BDown => MidiMessage::NoteOn(Channel::Channel1, Note::E3, U7::MAX),
            Keys::BUp => MidiMessage::NoteOff(Channel::Channel1, Note::E3, U7::MAX),
            Keys::ADown => MidiMessage::NoteOn(Channel::Channel1, Note::F3, U7::MAX),
            Keys::AUp => MidiMessage::NoteOff(Channel::Channel1, Note::F3, U7::MAX),
        }) {
            let _ = midi.send_message(UsbMidiEventPacket::from_midi(CableNumber::Cable1, message));
        }
    }
}
