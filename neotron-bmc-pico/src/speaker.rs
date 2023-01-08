use stm32f0xx_hal::{
	pac::{RCC, TIM14},
	rcc::Rcc,
};

#[derive(Debug, Default)]
pub struct RegisterState {
	/// The duration of the current note (0 = off)
	pub duration: u16,
	/// The PWM period (in 48kHz ticks)
	pub period: u16,
	/// The duty cycle (0 - 255)
	pub duty_cycle: u8,
	/// Whether the speaker config is dirty (needs to be sent to the PWM device)
	pub needs_update: bool,
}

impl RegisterState {
	pub fn duty_cycle(&self) -> u8 {
		self.duty_cycle
	}

	pub fn set_duty_cycle(&mut self, duty_cycle: u8) {
		self.duty_cycle = duty_cycle;
	}

	pub fn period(&self) -> u16 {
		self.period
	}

	pub fn set_period(&mut self, period: u16) {
		self.period = period;
	}

	pub fn period_high(&self) -> u8 {
		(self.period >> 8) as u8
	}

	pub fn set_period_high(&mut self, period_high: u8) {
		self.period = (self.period & 0xff00) | period_high as u16;
	}

	pub fn period_low(&self) -> u8 {
		(self.period & 0xff) as u8
	}

	pub fn set_period_low(&mut self, period_low: u8) {
		self.period = (self.period() & 0xff) | ((period_low as u16) << 8);
	}

	pub fn duration(&self) -> u16 {
		self.duration
	}

	pub fn set_duration(&mut self, duration: u16) {
		self.duration = duration;
		self.needs_update = true;
	}

	pub fn needs_update(&self) -> bool {
		self.needs_update
	}

	pub fn set_needs_update(&mut self, needs_update: bool) {
		self.needs_update = needs_update;
	}

	pub fn setup(&self, _rcc: &mut Rcc, tim14: &TIM14) {
		let rcc = RCC::ptr();
		// enable and reset peripheral to a clean slate state
		unsafe {
			(*rcc).apb1enr.modify(|_, w| w.tim14en().set_bit());
			(*rcc).apb1rstr.modify(|_, w| w.tim14rst().set_bit());
			(*rcc).apb1rstr.modify(|_, w| w.tim14rst().clear_bit());
		}

		tim14
			.ccmr1_output()
			.modify(|_, w| w.oc1pe().set_bit().oc1m().bits(6));

		// prescale 1000 (48MHz -> 48 kHz)
		tim14.psc.write(|w| w.psc().bits(1000));
	}
}

pub struct Hardware(TIM14);

impl Hardware {
	pub fn new(tim14: TIM14) -> Self {
		Self(tim14)
	}

	pub fn disable(&mut self) {
		self.0.ccer.modify(|_, w| w.cc1e().clear_bit());
	}

	pub fn enable(&mut self) {
		self.0.ccer.modify(|_, w| w.cc1e().set_bit());
	}

	fn update_register(&self, config: &RegisterState) {
		// period
		self.0
			.arr
			.write(|w| unsafe { w.bits(config.period() as u32) });

		// duty cycle (255 = whole period)
		self.0.ccr1.write(|w| {
			w.ccr()
				.bits((config.period() * config.duty_cycle() as u16) / 255)
		});

		// enable auto-reload preload
		self.0.cr1.modify(|_, w| w.arpe().set_bit());

		// Trigger update event to load the registers
		self.0.cr1.modify(|_, w| w.urs().set_bit());
		self.0.egr.write(|w| w.ug().set_bit());
		self.0.cr1.modify(|_, w| w.urs().clear_bit());

		self.0.cr1.write(|w| w.cen().set_bit());
	}

	/// Update the status of the registers and PWM output. Return `true` if a note is to be played.
	pub fn update<F: FnOnce()>(&mut self, register: &RegisterState, enable_cb: F) -> bool {
		if register.duration > 0 {
			// a note has to be played - enable PWM
			self.update_register(register);
			self.enable();
			enable_cb();
			true
		} else {
			// nothing to play (duration == 0), just disable PWM
			self.disable();
			false
		}
	}

	pub fn set_note(&mut self, duration: u16, period: u16, duty_cycle: u8) {
		let config = RegisterState {
			duration,
			period,
			duty_cycle,
			needs_update: true,
		};
		self.update_register(&config);
	}
}
