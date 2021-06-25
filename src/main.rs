#![no_main]
#![no_std]

use neotron_bmc as _;

use hal::{i2c, pac, prelude::*, serial, spi};
use stm32f0xx_hal as hal;

static VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

/// Holds all the I/O pins in our system
#[allow(unused)]
struct Pins {
	// spi_clk: hal::gpio::gpioa::PA5<hal::gpio::Alternate<hal::gpio::AF0>>,
	// spi_cipo: hal::gpio::gpioa::PA6<hal::gpio::Alternate<hal::gpio::AF0>>,
	// spi_copi: hal::gpio::gpioa::PA7<hal::gpio::Alternate<hal::gpio::AF0>>,
	// i2c_scl: hal::gpio::gpiob::PB6<hal::gpio::Alternate<hal::gpio::AF4>>,
	// i2c_sda: hal::gpio::gpiob::PB7<hal::gpio::Alternate<hal::gpio::AF4>>,
	uart_tx: hal::gpio::gpioa::PA9<hal::gpio::Alternate<hal::gpio::AF1>>,
	uart_rx: hal::gpio::gpioa::PA10<hal::gpio::Alternate<hal::gpio::AF1>>,
	uart_cts: hal::gpio::gpioa::PA11<hal::gpio::Alternate<hal::gpio::AF1>>,
	uart_rts: hal::gpio::gpioa::PA12<hal::gpio::Alternate<hal::gpio::AF1>>,
	led_power: hal::gpio::gpiob::PB0<hal::gpio::Output<hal::gpio::PushPull>>,
	led_status: hal::gpio::gpiob::PB1<hal::gpio::Output<hal::gpio::PushPull>>,
}

#[cortex_m_rt::entry]
fn main() -> ! {
	defmt::info!("Neotron BMC version {:?} booting", VERSION);

	let p = pac::Peripherals::take().unwrap();

	// Configure the clocks.
	// We have no external crystal, and instead run from the High Speed Internal Oscillator.
	let mut flash = p.FLASH;
	let mut rcc = p
		.RCC
		.configure()
		.hclk(48.mhz())
		.pclk(48.mhz())
		.sysclk(48.mhz())
		.freeze(&mut flash);

	defmt::info!("Clocks are clocked");

	let gpioa = p.GPIOA.split(&mut rcc);
	let gpiob = p.GPIOB.split(&mut rcc);

	let mut pins: Pins = cortex_m::interrupt::free(move |cs| {
		Pins {
			// SPI pins
			// spi_clk: gpioa.pa5.into_alternate_af0(cs),
			// spi_cipo: gpioa.pa6.into_alternate_af0(cs),
			// spi_copi: gpioa.pa7.into_alternate_af0(cs),
			// I²C pins
			// i2c_scl: gpiob.pb6.into_alternate_af4(cs),
			// i2c_sda: gpiob.pb7.into_alternate_af4(cs),
			// USART pins
			uart_tx: gpioa.pa9.into_alternate_af1(cs),
			uart_rx: gpioa.pa10.into_alternate_af1(cs),
			uart_cts: gpioa.pa11.into_alternate_af1(cs),
			uart_rts: gpioa.pa12.into_alternate_af1(cs),
			// LED pins
			led_power: gpiob.pb0.into_push_pull_output(cs),
			led_status: gpiob.pb1.into_push_pull_output(cs),
		}
	});

	// // Configure UART. Pick some default baud rate - we can change it later
	let mut serial = serial::Serial::usart1(
		p.USART1,
		(pins.uart_tx, pins.uart_rx),
		115_200.bps(),
		&mut rcc,
	);

	use core::fmt::Write;

	loop {
		serial.write_str("Hello, world!\r\n").unwrap();

		defmt::info!("Off Off");

		pins.led_power.set_low().unwrap();
		pins.led_status.set_low().unwrap();

		cortex_m::asm::delay(6_000_000);

		defmt::info!("Off On");

		pins.led_power.set_low().unwrap();
		pins.led_status.set_high().unwrap();

		cortex_m::asm::delay(6_000_000);

		defmt::info!("On Off");

		pins.led_power.set_high().unwrap();
		pins.led_status.set_low().unwrap();

		cortex_m::asm::delay(6_000_000);

		defmt::info!("On On");

		pins.led_power.set_high().unwrap();
		pins.led_status.set_high().unwrap();

		cortex_m::asm::delay(6_000_000);
	}
}
