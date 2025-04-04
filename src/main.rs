#![no_std]
#![no_main]

use cortex_m::delay;
// use panic_halt as _;
use defmt::*;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use rp235x_hal::arch::delay;

use hal::fugit::RateExtU32;
use hal::gpio::{FunctionI2C, Pin};
use hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use hal::Clock;
use heapless::String;
use panic_probe as _;
use rp235x_hal as hal;
use sh1106::{interface::DisplayInterface, prelude::*, Builder};
use ufmt::uwrite;

type PicoI2c = rp235x_hal::I2C<
    rp235x_hal::pac::I2C0,
    (
        Pin<
            rp235x_hal::gpio::bank0::Gpio16,
            rp235x_hal::gpio::FunctionI2c,
            rp235x_hal::gpio::PullUp,
        >,
        Pin<
            rp235x_hal::gpio::bank0::Gpio17,
            rp235x_hal::gpio::FunctionI2c,
            rp235x_hal::gpio::PullUp,
        >,
    ),
>;

type OledDisplay = GraphicsMode<I2cInterface<PicoI2c>>;

type UartP = UartPeripheral<
    rp235x_hal::uart::Enabled,
    rp235x_hal::pac::UART1,
    (
        Pin<
            rp235x_hal::gpio::bank0::Gpio4,
            rp235x_hal::gpio::FunctionUart,
            rp235x_hal::gpio::PullDown,
        >,
        Pin<
            rp235x_hal::gpio::bank0::Gpio5,
            rp235x_hal::gpio::FunctionUart,
            rp235x_hal::gpio::PullDown,
        >,
    ),
>;

// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
/// Adjust if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[hal::entry]
fn main() -> ! {
    info!("Program start");

    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // UART TX (characters sent from pico) on pin 1 (GPIO0) and RX (on pin 2 (GPIO1)
    let uart_pins = (
        pins.gpio4.into_function::<hal::gpio::FunctionUart>(),
        pins.gpio5.into_function::<hal::gpio::FunctionUart>(),
    );

    // Create a UART driver
    let uart: UartP = UartPeripheral::new(pac.UART1, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    // Write to the UART
    uart.write_full_blocking(b"ADC example\r\n");

    // Configure two pins as being I²C, not GPIO
    let sda_pin: Pin<_, FunctionI2C, _> = pins.gpio16.reconfigure();
    let scl_pin: Pin<_, FunctionI2C, _> = pins.gpio17.reconfigure();

    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin, // Try `not_an_scl_pin` here
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut display: OledDisplay = Builder::new().connect_i2c(i2c).into();

    display.init().unwrap();

    let text_style: embedded_graphics::mono_font::MonoTextStyle<'_, BinaryColor> =
        MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

    let mut i = 0;

    loop {
        print_text(&mut display, &uart, text_style, i);
        info!("Program start");

        delay(1000_000);

        i += 1;
    }
}

fn print_text(
    display: &mut OledDisplay,
    uart: &UartP,
    text_style: embedded_graphics::mono_font::MonoTextStyle<'_, BinaryColor>,
    i: i32,
) {
    uart.write_full_blocking(b"ADC example\r\n");

    display.clear();

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    let mut s: String<64> = String::new();
    uwrite!(s, "Hello Rust! {}", i).unwrap();

    Text::with_baseline(s.as_str(), Point::new(0, 16), text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    display.flush().unwrap();
}

#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"USB Serial Example"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];
