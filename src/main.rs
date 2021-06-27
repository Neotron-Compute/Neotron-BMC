#![no_main]
#![no_std]

use neotron_bmc as _;

use neotron_bmc::monotonic::{Instant, Tim3Monotonic, U16Ext};

use cortex_m_rt::exception;

use rtic::app;

use stm32f0xx_hal::{
	gpio::gpioa::{PA10, PA11, PA12, PA9},
	gpio::gpiob::{PB0, PB1},
	gpio::{Alternate, Output, PushPull, AF1},
	pac,
	prelude::*,
	serial,
};

use cortex_m::interrupt::free as disable_interrupts;

static VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

const PERIOD_MS: u16 = 1000;

#[app(device = crate::pac, peripherals = true,  monotonic = crate::Tim3Monotonic)]
const APP: () = {
	struct Resources {
		uart_cts: PA11<Alternate<AF1>>,
		uart_rts: PA12<Alternate<AF1>>,
		led_power: PB0<Output<PushPull>>,
		led_status: PB1<Output<PushPull>>,
		serial: serial::Serial<pac::USART1, PA9<Alternate<AF1>>, PA10<Alternate<AF1>>>,
	}

	#[init(spawn = [led_status_blink])]
	fn init(ctx: init::Context) -> init::LateResources {
		defmt::info!("Neotron BMC version {:?} booting", VERSION);

		let dp: pac::Peripherals = ctx.device;

		let mut flash = dp.FLASH;
		let mut rcc = dp
			.RCC
			.configure()
			.hclk(48.mhz())
			.pclk(48.mhz())
			.sysclk(48.mhz())
			.freeze(&mut flash);

		defmt::info!("Configuring TIM3 at 7.8125 kHz...");
		crate::Tim3Monotonic::initialize(dp.TIM3);

		defmt::info!("Creating pins...");
		let gpioa = dp.GPIOA.split(&mut rcc);
		let gpiob = dp.GPIOB.split(&mut rcc);
		let (uart_tx, uart_rx, uart_cts, uart_rts, mut led_power, led_status) =
			disable_interrupts(|cs| {
				(
					gpioa.pa9.into_alternate_af1(cs),
					gpioa.pa10.into_alternate_af1(cs),
					gpioa.pa11.into_alternate_af1(cs),
					gpioa.pa12.into_alternate_af1(cs),
					gpiob.pb0.into_push_pull_output(cs),
					gpiob.pb1.into_push_pull_output(cs),
				)
			});

		defmt::info!("Creating UART...");

		let mut serial =
			serial::Serial::usart1(dp.USART1, (uart_tx, uart_rx), 115_200.bps(), &mut rcc);

		serial.listen(serial::Event::Rxne);

		ctx.spawn.led_status_blink().unwrap();

		led_power.set_high().unwrap();

		defmt::info!("Init complete!");

		init::LateResources {
			serial,
			uart_cts,
			uart_rts,
			led_power,
			led_status,
		}
	}

	#[idle(resources = [])]
	fn idle(_: idle::Context) -> ! {
		defmt::info!("Idle is running...");
		loop {
			cortex_m::asm::wfi();
			defmt::info!("It is now {}", crate::Instant::now().counts());
		}
	}

	#[task(binds = USART1, resources=[serial])]
	fn usart1_interrupt(ctx: usart1_interrupt::Context) {
		// Reading the register clears the RX-Not-Empty-Interrupt flag.
		match ctx.resources.serial.read() {
			Ok(b) => {
				defmt::info!("<< UART {:x}", b);
			}
			Err(_) => {
				defmt::warn!("<< UART None?");
			}
		}
	}

	#[task(schedule = [led_status_blink], resources = [led_status])]
	fn led_status_blink(ctx: led_status_blink::Context) {
		// Use the safe local `static mut` of RTIC
		static mut LED_STATE: bool = false;

		defmt::info!("blink time {}", ctx.scheduled.counts());

		if *LED_STATE {
			ctx.resources.led_status.set_low().unwrap();
			*LED_STATE = false;
		} else {
			ctx.resources.led_status.set_high().unwrap();
			*LED_STATE = true;
		}
		let next = ctx.scheduled + PERIOD_MS.millis();
		defmt::info!("Next blink at {}", next.counts());
		ctx.schedule.led_status_blink(next).unwrap();
	}

	// Let it use the USB interrupt as a generic software interrupt.
	extern "C" {
		fn USB();
	}
};

#[exception]
unsafe fn DefaultHandler(value: i16) {
	defmt::panic!("DefaultHandler({})", value);
}

// SPI pins
// spi_clk: gpioa.pa5.into_alternate_af0(cs),
// spi_cipo: gpioa.pa6.into_alternate_af0(cs),
// spi_copi: gpioa.pa7.into_alternate_af0(cs),
// I²C pins
// i2c_scl: gpiob.pb6.into_alternate_af4(cs),
// i2c_sda: gpiob.pb7.into_alternate_af4(cs),
