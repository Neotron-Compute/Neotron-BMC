#![no_main]
#![no_std]

use neotron_bmc as _;

use hal::{i2c, pac, prelude::*, serial, spi};
use stm32f0xx_hal as hal;

static VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

const MODE_0: spi::Mode = spi::Mode {
	polarity: spi::Polarity::IdleLow,
	phase: spi::Phase::CaptureOnFirstTransition,
};

#[cortex_m_rt::entry]
fn main() -> ! {
	defmt::info!("Neotron BMC version {:?} booting", VERSION);

	let p = pac::Peripherals::take().unwrap();

	let mut flash = p.FLASH;
	let mut rcc = p.RCC.configure().freeze(&mut flash);

	let gpioa = p.GPIOA.split(&mut rcc);
	let gpiob = p.GPIOB.split(&mut rcc);

	let (sck, miso, mosi, scl, sda, tx, rx) = cortex_m::interrupt::free(move |cs| {
		(
			// SPI pins
			gpioa.pa5.into_alternate_af0(cs),
			gpioa.pa6.into_alternate_af0(cs),
			gpioa.pa7.into_alternate_af0(cs),
			// I²C pins
			gpioa.pa9.into_alternate_af4(cs),
			gpioa.pa10.into_alternate_af4(cs),
			// USART pins
			gpiob.pb6.into_alternate_af0(cs),
			gpiob.pb7.into_alternate_af0(cs),
		)
	});

	// Configure SPI with 1MHz rate
	let spi = spi::Spi::spi1(p.SPI1, (sck, miso, mosi), MODE_0, 1.mhz(), &mut rcc);
	let serial = serial::Serial::usart1(p.USART1, (tx, rx), 115_200.bps(), &mut rcc);
	let i2c = i2c::I2c::i2c1(p.I2C1, (scl, sda), 100.khz(), &mut rcc);

	neotron_bmc::exit()
}
