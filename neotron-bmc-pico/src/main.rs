#![no_main]
#![no_std]

///! Neotron BMC Firmware
///!
///! This is the firmware for the Neotron Board Management Controller (BMC). It controls the power, reset, UART and PS/2 ports on a Neotron mainboard.
///! For more details, see the `README.md` file.
///!
///! # Licence
///! This source code as a whole is licensed under the GPL v3. Third-party crates are covered by their respective licences.
use cortex_m::interrupt::free as disable_interrupts;
use heapless::{
	consts::*,
	i,
	spsc::{Consumer, Producer, Queue},
};
use rtic::app;
use stm32f0xx_hal::{
	gpio::gpioa::{PA10, PA11, PA12, PA15, PA2, PA3, PA9},
	gpio::gpiob::{PB0, PB1, PB3, PB4, PB5},
	gpio::gpiof::{PF0, PF1},
	gpio::{Alternate, Floating, Input, Output, PullUp, PushPull, AF1},
	pac,
	prelude::*,
	serial,
};

use neotron_bmc_pico as _;
use neotron_bmc_pico::monotonic::{Tim3Monotonic, U16Ext};

/// Version string auto-generated by git.
static VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

/// At what rate do we blink the status LED when we're running?
const LED_PERIOD_MS: u16 = 1000;

/// How often we poll the power and reset buttons in milliseconds.
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

/// Handles decoding incoming PS/2 packets
///
/// Each packet has 11 bits:
///
/// * Start Bit
/// * 8 Data Bits (LSB first)
/// * Parity Bit
/// * Stop Bit
#[derive(Debug)]
pub struct Ps2Decoder {
	bit_count: u8,
	collector: u16,
}

/// This is our system state, as accessible via SPI reads and writes.
#[derive(Debug)]
pub struct RegisterState {
	firmware_version: &'static str,
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
		pin_uart_cts: PA11<Alternate<AF1>>,
		/// The Ready-To-Receive line on the FTDI UART header (which the serial object can't handle)
		pin_uart_rts: PA12<Alternate<AF1>>,
		/// The power button
		button_power: PF0<Input<PullUp>>,
		/// The reset button
		button_reset: PF1<Input<PullUp>>,
		/// Tracks power button state for short presses. 75ms x 2 = 150ms is a short press
		press_button_power_short: debouncr::Debouncer<u8, debouncr::Repeat2>,
		/// Tracks power button state for long presses. 75ms x 16 = 1200ms is a long press
		press_button_power_long: debouncr::Debouncer<u16, debouncr::Repeat16>,
		/// Tracks DC power state
		state_dc_power_enabled: DcPowerState,
		/// Controls the DC-DC PSU
		pin_dc_on: PA3<Output<PushPull>>,
		/// Controls the Reset signal across the main board, putting all the
		/// chips (except this BMC!) in reset when pulled low.
		pin_sys_reset: PA2<Output<PushPull>>,
		/// Clock pin for PS/2 Keyboard port
		ps2_clk0: PA15<Input<Floating>>,
		/// Clock pin for PS/2 Mouse port
		ps2_clk1: PB3<Input<Floating>>,
		/// Data pin for PS/2 Keyboard port
		ps2_dat0: PB4<Input<Floating>>,
		/// Data pin for PS/2 Mouse port
		ps2_dat1: PB5<Input<Floating>>,
		/// The external interrupt peripheral
		exti: pac::EXTI,
		/// Our register state
		register_state: RegisterState,
		/// Keyboard PS/2 decoder
		kb_decoder: Ps2Decoder,
		/// Mouse PS/2 decoder
		ms_decoder: Ps2Decoder,
		/// Keyboard bytes sink
		kb_c: Consumer<'static, u16, U8>,
		/// Keyboard bytes source
		kb_p: Producer<'static, u16, U8>,
	}

	/// The entry point to our application.
	///
	/// Sets up the hardware and spawns the regular tasks.
	///
	/// * Task `led_power_blink` - blinks the LED
	/// * Task `button_poll` - checks the power and reset buttons
	#[init(spawn = [led_power_blink, button_poll])]
	fn init(ctx: init::Context) -> init::LateResources {
		static mut Q: Queue<u16, U8> = Queue(i::Queue::new());

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
			pin_uart_cts,
			pin_uart_rts,
			mut led_power,
			led_status,
			button_power,
			button_reset,
			mut pin_dc_on,
			mut pin_sys_reset,
			ps2_clk0,
			ps2_clk1,
			ps2_dat0,
			ps2_dat1,
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
				gpioa.pa3.into_push_pull_output(cs),
				gpioa.pa2.into_push_pull_output(cs),
				gpioa.pa15.into_floating_input(cs),
				gpiob.pb3.into_floating_input(cs),
				gpiob.pb4.into_floating_input(cs),
				gpiob.pb5.into_floating_input(cs),
			)
		});

		pin_sys_reset.set_low().unwrap();
		pin_dc_on.set_low().unwrap();

		defmt::info!("Creating UART...");

		let mut serial =
			serial::Serial::usart1(dp.USART1, (uart_tx, uart_rx), 115_200.bps(), &mut rcc);

		serial.listen(serial::Event::Rxne);

		ctx.spawn.led_power_blink().unwrap();

		ctx.spawn.button_poll().unwrap();

		led_power.set_low().unwrap();

		// Set EXTI15 to use PORT A (PA15)
		dp.SYSCFG.exticr4.write(|w| w.exti15().pa15());

		// Enable EXTI15 interrupt as external falling edge
		dp.EXTI.imr.modify(|_r, w| w.mr15().set_bit());
		dp.EXTI.emr.modify(|_r, w| w.mr15().set_bit());
		dp.EXTI.ftsr.modify(|_r, w| w.tr15().set_bit());

		defmt::info!("Init complete!");

		let (kb_p, kb_c) = Q.split();

		init::LateResources {
			serial,
			pin_uart_cts,
			pin_uart_rts,
			led_power,
			led_status,
			button_power,
			button_reset,
			press_button_power_short: debouncr::debounce_2(false),
			press_button_power_long: debouncr::debounce_16(false),
			state_dc_power_enabled: DcPowerState::Off,
			pin_dc_on,
			pin_sys_reset,
			ps2_clk0,
			ps2_clk1,
			ps2_dat0,
			ps2_dat1,
			exti: dp.EXTI,
			register_state: RegisterState {
				firmware_version: "Neotron BMC v0.0.0",
			},
			kb_p,
			kb_c,
			kb_decoder: Ps2Decoder::new(),
			ms_decoder: Ps2Decoder::new(),
		}
	}

	/// Our idle task.
	///
	/// This task is called when there is nothing else to do. We
	/// do a little logging, then put the CPU to sleep waiting for an interrupt.
	#[idle(resources = [kb_c])]
	fn idle(ctx: idle::Context) -> ! {
		defmt::info!("Idle is running...");
		loop {
			if let Some(word) = ctx.resources.kb_c.dequeue() {
				if let Some(byte) = Ps2Decoder::check_word(word) {
					defmt::info!("< KB {:x}", byte);
				} else {
					defmt::info!("< Bad KB {:x}", word);
				}
			}
		}
	}

	/// This is the PS/2 Keyboard task.
	///
	/// It is very high priority, as we can't afford to miss a clock edge.
	///
	/// It fires when there is a falling edge on the PS/2 Keyboard clock pin.
	#[task(
		binds = EXTI4_15,
		priority = 15,
		resources=[ps2_clk0, ps2_dat0, exti, kb_decoder, kb_p])]
	fn exti4_15_interrupt(ctx: exti4_15_interrupt::Context) {
		let data_bit = ctx.resources.ps2_dat0.is_high().unwrap();
		// Do we have a complete word (and if so, is the parity OK)?
		if let Some(data) = ctx.resources.kb_decoder.add_bit(data_bit) {
			// Don't dump in the ISR - we're busy. Add it to this nice lockless queue instead.
			ctx.resources.kb_p.enqueue(data).unwrap();
		}
		// Clear the pending flag
		ctx.resources.exti.pr.write(|w| w.pr15().set_bit());
	}

	/// This is the USART1 task.
	///
	/// It fires whenever there is new data received on USART1. We should flag to the host
	/// that data is available.
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

	/// This is the LED blink task.
	///
	/// This task is called periodically. We check whether the status LED is currently on or off,
	/// and set it to the opposite. This makes the LED blink.
	#[task(schedule = [led_power_blink], resources = [led_power, state_dc_power_enabled])]
	fn led_power_blink(ctx: led_power_blink::Context) {
		// Use the safe local `static mut` of RTIC
		static mut LED_STATE: bool = false;

		if *ctx.resources.state_dc_power_enabled == DcPowerState::Off {
			defmt::trace!("blink time {}", ctx.scheduled.counts());
			if *LED_STATE {
				ctx.resources.led_power.set_low().unwrap();
				*LED_STATE = false;
			} else {
				ctx.resources.led_power.set_high().unwrap();
				*LED_STATE = true;
			}
			let next = ctx.scheduled + LED_PERIOD_MS.millis();
			defmt::trace!("Next blink at {}", next.counts());
			ctx.schedule.led_power_blink(next).unwrap();
		}
	}

	/// This task polls our power and reset buttons.
	///
	/// We poll them rather than setting up an interrupt as we need to debounce them, which involves waiting a short period and checking them again. Given that we have to do that, we might as well not bother with the interrupt.
	#[task(
		schedule = [button_poll],
		spawn = [led_power_blink],
		resources = [
			led_power, button_power, press_button_power_short, press_button_power_long, state_dc_power_enabled,
			pin_sys_reset, pin_dc_on
		]
	)]
	fn button_poll(ctx: button_poll::Context) {
		// Poll button
		let pressed: bool = ctx.resources.button_power.is_low().unwrap();

		// Update state
		let short_edge = ctx.resources.press_button_power_short.update(pressed);
		let long_edge = ctx.resources.press_button_power_long.update(pressed);

		// Dispatch event
		match (long_edge, short_edge, *ctx.resources.state_dc_power_enabled) {
			(None, Some(debouncr::Edge::Rising), DcPowerState::Off) => {
				defmt::info!("Power button pressed whilst off.");
				// Button pressed - power on system
				*ctx.resources.state_dc_power_enabled = DcPowerState::Starting;
				ctx.resources.led_power.set_high().unwrap();
				defmt::info!("Power on!");
				ctx.resources.pin_dc_on.set_high().unwrap();
				// TODO: Start monitoring 3.3V and 5.0V rails here
				// TODO: Take system out of reset when 3.3V and 5.0V are good
				ctx.resources.pin_sys_reset.set_high().unwrap();
			}
			(None, Some(debouncr::Edge::Falling), DcPowerState::Starting) => {
				defmt::info!("Power button released.");
				// Button released after power on
				*ctx.resources.state_dc_power_enabled = DcPowerState::On;
			}
			(Some(debouncr::Edge::Rising), None, DcPowerState::On) => {
				defmt::info!("Power button held whilst on.");
				*ctx.resources.state_dc_power_enabled = DcPowerState::Off;
				ctx.resources.led_power.set_low().unwrap();
				defmt::info!("Power off!");
				ctx.resources.pin_sys_reset.set_low().unwrap();
				// TODO: Wait for 100ms for chips to stop?
				ctx.resources.pin_dc_on.set_low().unwrap();
				// Start LED blinking again
				ctx.spawn.led_power_blink().unwrap();
			}
			_ => {
				// Do nothing
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

impl Ps2Decoder {
	fn new() -> Ps2Decoder {
		Ps2Decoder {
			bit_count: 0,
			collector: 0,
		}
	}

	fn reset(&mut self) {
		self.bit_count = 0;
		self.collector = 0;
	}

	fn add_bit(&mut self, bit: bool) -> Option<u16> {
		if bit {
			self.collector |= 1 << self.bit_count;
		}
		self.bit_count += 1;
		if self.bit_count == 11 {
			let result = self.collector;
			self.reset();
			Some(result)
		} else {
			None
		}
	}

	/// Check 11-bit word has 1 start bit, 1 stop bit and an odd parity bit.
	fn check_word(word: u16) -> Option<u8> {
		let start_bit = (word & 0x0001) != 0;
		let parity_bit = (word & 0x0200) != 0;
		let stop_bit = (word & 0x0400) != 0;
		let data = ((word >> 1) & 0xFF) as u8;

		if start_bit {
			return None;
		}

		if !stop_bit {
			return None;
		}

		let need_parity = (data.count_ones() % 2) == 0;

		// Odd parity, so these must not match
		if need_parity != parity_bit {
			return None;
		}

		Some(data)
	}
}

// TODO: Pins we haven't used yet
// SPI pins
// spi_clk: gpioa.pa5.into_alternate_af0(cs),
// spi_cipo: gpioa.pa6.into_alternate_af0(cs),
// spi_copi: gpioa.pa7.into_alternate_af0(cs),
// I²C pins
// i2c_scl: gpiob.pb6.into_alternate_af4(cs),
// i2c_sda: gpiob.pb7.into_alternate_af4(cs),