#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

use defmt_rtt as _; // global logger
use panic_probe as _;
use stm32f0xx_hal as _; // memory layout // panic handler

pub mod ps2;
pub mod spi;

/// Reset SPI1.
///
/// Takes `Rcc` as a token to prove you have exclusive access. Ideally this
/// would be a method on the [`Rcc`] type, but it isn't in the STM32F030
/// HAL.
pub fn reset_spi1(_token: &mut stm32f0xx_hal::rcc::Rcc) {
	// See DS9773, Table 17
	let rcc_base = 0x4002_1000 as *mut u32;
	// See RM0360 Section 7.415
	let rcc_apb2rstr = unsafe { rcc_base.offset(0x0C / 4) };
	let spi1_bit = 1 << 12;
	// Write 1 to the register to do a reset
	unsafe {
		rcc_apb2rstr.write_volatile(spi1_bit);
		rcc_apb2rstr.write_volatile(0);
	}
}

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
	cortex_m::asm::udf()
}

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", {
	// NOTE(no-CAS) `timestamps` runs with interrupts disabled
	let n = COUNT.load(Ordering::Relaxed);
	COUNT.store(n + 1, Ordering::Relaxed);
	n
});

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
	loop {
		cortex_m::asm::bkpt();
	}
}
