//! Using STM32F0 TIM2 as monotonic 32 bit timer.

use core::u32;
use rtic::Monotonic;
use stm32f0xx_hal::pac;
use stm32f0xx_hal::rcc::Clocks;

/// Implementor of the `rtic::Monotonic` traits and used to consume the timer
/// to not allow for erroneous configuration.
///
/// This uses TIM2 internally.
pub struct MonoTimer<T, const FREQ: u32>(T);

impl<const FREQ: u32> MonoTimer<pac::TIM2, FREQ> {
	/// Initialize the timer instance.
	pub fn new(timer: pac::TIM2, clocks: &Clocks) -> Self {
		// Enable and reset TIM2 in RCC
		//
		// Correctness: Since we only modify TIM2 related registers in the RCC
		// register block, and since we own pac::TIM2, we should be safe.
		unsafe {
			let rcc = &*pac::RCC::ptr();

			// Enable timer
			rcc.apb1enr.modify(|_, w| w.tim2en().set_bit());

			// Reset timer
			rcc.apb1rstr.modify(|_, w| w.tim2rst().set_bit());
			rcc.apb1rstr.modify(|_, w| w.tim2rst().clear_bit());
		}

		let prescaler = (clocks.pclk().0 / FREQ) - 1;

		// Set up prescaler
		timer.psc.write(|w| w.psc().bits(prescaler as u16));

		// Update prescaler
		timer.egr.write(|w| w.ug().update());

		// Enable counter
		timer.cr1.modify(|_, w| w.cen().set_bit());

		MonoTimer(timer)
	}
}

impl<const FREQ: u32> Monotonic for MonoTimer<pac::TIM2, FREQ> {
	type Instant = fugit::TimerInstantU32<FREQ>;
	type Duration = fugit::TimerDurationU32<FREQ>;

	unsafe fn reset(&mut self) {
		self.0.dier.modify(|_, w| w.cc1ie().set_bit());
	}

	#[inline(always)]
	fn now(&mut self) -> Self::Instant {
		Self::Instant::from_ticks(self.0.cnt.read().cnt().bits())
	}

	fn set_compare(&mut self, instant: Self::Instant) {
		self.0
			.ccr1
			.write(|w| w.ccr().bits(instant.duration_since_epoch().ticks()));
	}

	fn clear_compare_flag(&mut self) {
		self.0.sr.modify(|_, w| w.cc1if().clear_bit());
	}

	#[inline(always)]
	fn zero() -> Self::Instant {
		Self::Instant::from_ticks(0)
	}
}

// End of file
