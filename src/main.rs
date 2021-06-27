#![no_main]
#![no_std]

use neotron_bmc as _;

use neotron_bmc::monotonic::{Instant, Tim3Monotonic, U16Ext};

use cortex_m_rt::exception;

use rtic::app;

use stm32f0xx_hal::{
	gpio::gpioa::{PA10, PA11, PA12, PA9},
	gpio::gpiob::{PB0, PB1},
	gpio::gpiof::{PF0, PF1},
	gpio::{Alternate, Input, Output, PullUp, PushPull, AF1},
	pac,
	prelude::*,
	serial,
};

use cortex_m::interrupt::free as disable_interrupts;

static VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

const LED_PERIOD_MS: u16 = 1000;
const DEBOUNCE_POLL_INTERVAL_MS: u16 = 75;

/// The states we can be in controlling the DC power
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DcPowerState {
	/// We've just enabled the DC power (so ignore any incoming long presses!)
	Starting = 1,
	/// We are now fully on. Look for a long press to turn off.
	On = 2,
	/// We are fully off.
	Off = 0,
}

#[app(device = crate::pac, peripherals = true,  monotonic = crate::Tim3Monotonic)]
const APP: () = {
	struct Resources {
		/// The power LED (D1101)
		led_power: PB0<Output<PushPull>>,
		/// The status LED (D1102)
		led_status: PB1<Output<PushPull>>,
		/// The FTDI UART header (J105)
		serial: serial::Serial<pac::USART1, PA9<Alternate<AF1>>, PA10<Alternate<AF1>>>,
		/// The Clear-To-Send line on the FTDI UART header (which the serial object can't handle)
		uart_cts: PA11<Alternate<AF1>>,
		/// The Ready-To-Receive line on the FTDI UART header (which the serial object can't handle)
		uart_rts: PA12<Alternate<AF1>>,
		/// The power button
		button_power: PF0<Input<PullUp>>,
		/// The reset button
		button_reset: PF1<Input<PullUp>>,
		/// Tracks power button state for short presses. 75ms x 2 = 150ms is a short press
		button_power_short_press: debouncr::Debouncer<u8, debouncr::Repeat2>,
		/// Tracks power button state for long presses. 75ms x 16 = 1200ms is a long press
		button_power_long_press: debouncr::Debouncer<u16, debouncr::Repeat16>,
		/// Tracks DC power state
		dc_power_enabled: DcPowerState,
	}

	#[init(spawn = [led_status_blink, button_poll])]
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
		let gpiof = dp.GPIOF.split(&mut rcc);
		let (
			uart_tx,
			uart_rx,
			uart_cts,
			uart_rts,
			mut led_power,
			led_status,
			button_power,
			button_reset,
		) = disable_interrupts(|cs| {
			(
				gpioa.pa9.into_alternate_af1(cs),
				gpioa.pa10.into_alternate_af1(cs),
				gpioa.pa11.into_alternate_af1(cs),
				gpioa.pa12.into_alternate_af1(cs),
				gpiob.pb0.into_push_pull_output(cs),
				gpiob.pb1.into_push_pull_output(cs),
				gpiof.pf0.into_pull_up_input(cs),
				gpiof.pf1.into_pull_up_input(cs),
			)
		});

		defmt::info!("Creating UART...");

		let mut serial =
			serial::Serial::usart1(dp.USART1, (uart_tx, uart_rx), 115_200.bps(), &mut rcc);

		serial.listen(serial::Event::Rxne);

		ctx.spawn.led_status_blink().unwrap();

		ctx.spawn.button_poll().unwrap();

		led_power.set_low().unwrap();

		defmt::info!("Init complete!");

		init::LateResources {
			serial,
			uart_cts,
			uart_rts,
			led_power,
			led_status,
			button_power,
			button_reset,
			button_power_short_press: debouncr::debounce_2(false),
			button_power_long_press: debouncr::debounce_16(false),
			dc_power_enabled: DcPowerState::Off,
		}
	}

	#[idle(resources = [])]
	fn idle(_ctx: idle::Context) -> ! {
		defmt::info!("Idle is running...");
		loop {
			cortex_m::asm::wfi();
			defmt::trace!("It is now {}", crate::Instant::now().counts());
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

		defmt::trace!("blink time {}", ctx.scheduled.counts());

		if *LED_STATE {
			ctx.resources.led_status.set_low().unwrap();
			*LED_STATE = false;
		} else {
			ctx.resources.led_status.set_high().unwrap();
			*LED_STATE = true;
		}
		let next = ctx.scheduled + LED_PERIOD_MS.millis();
		defmt::trace!("Next blink at {}", next.counts());
		ctx.schedule.led_status_blink(next).unwrap();
	}

	#[task(schedule = [button_poll], resources = [led_power, button_power, button_power_short_press, button_power_long_press, dc_power_enabled])]
	fn button_poll(ctx: button_poll::Context) {
		// Poll button
		let pressed: bool = ctx.resources.button_power.is_low().unwrap();

		// Update state
		let short_edge = ctx.resources.button_power_short_press.update(pressed);
		let long_edge = ctx.resources.button_power_long_press.update(pressed);

		// Dispatch event
		if short_edge == Some(debouncr::Edge::Rising) {
			defmt::trace!(
				"Power short press in! {}",
				*ctx.resources.dc_power_enabled as u8
			);
			if *ctx.resources.dc_power_enabled == DcPowerState::Off {
				*ctx.resources.dc_power_enabled = DcPowerState::Starting;
				ctx.resources.led_power.set_high().unwrap();
				defmt::info!("Power on!");
				// TODO: Enable DC PSU here
				// TODO: Start monitoring 3.3V and 5.0V rails here
				// TODO: Take system out of reset when 3.3V and 5.0V are good
			}
		} else if short_edge == Some(debouncr::Edge::Falling) {
			defmt::trace!(
				"Power short press out! {}",
				*ctx.resources.dc_power_enabled as u8
			);
			match *ctx.resources.dc_power_enabled {
				DcPowerState::Starting => {
					*ctx.resources.dc_power_enabled = DcPowerState::On;
				}
				DcPowerState::On => {
					// TODO: Tell host that power off was requested
				}
				DcPowerState::Off => {
					// Ignore
				}
			}
		}

		if long_edge == Some(debouncr::Edge::Rising) {
			defmt::trace!(
				"Power long press in! {}",
				*ctx.resources.dc_power_enabled as u8
			);
			if *ctx.resources.dc_power_enabled == DcPowerState::On {
				*ctx.resources.dc_power_enabled = DcPowerState::Off;
				ctx.resources.led_power.set_low().unwrap();
				defmt::info!("Power off!");
				// TODO: Put system in reset here
				// TODO: Disable DC PSU here
			}
		}

		// Re-schedule the timer interrupt
		ctx.schedule
			.button_poll(ctx.scheduled + DEBOUNCE_POLL_INTERVAL_MS.millis())
			.unwrap();
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
