//! # Basic PS/2 Decoder
//!
//! Like the one in 'pc_keyboard' but simpler. Designed for use when you want to
//! collect the bits but not decode the bytes.

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
	bit_mask: u16,
	collector: u16,
	ticks: u8,
}

impl Ps2Decoder {
	const MAX_TICKS_BEFORE_RESET: u8 = 3;

	/// Create a new PS/2 Decoder
	pub const fn new() -> Ps2Decoder {
		Ps2Decoder {
			bit_mask: 1,
			collector: 0,
			ticks: 0,
		}
	}

	/// Reset the PS/2 decoder
	pub fn reset(&mut self) {
		self.bit_mask = 1;
		self.collector = 0;
	}

	/// Call this on a timer tick. Too many timer ticks without a new bit
	/// arriving causes a reset.
	pub fn poll(&mut self) {
		if self.collector != 0 {
			self.ticks += 1;
			if self.ticks == Self::MAX_TICKS_BEFORE_RESET {
				self.reset();
			}
		}
	}

	/// Add a bit, and if we have enough, return the 11-bit PS/2 word.
	pub fn add_bit(&mut self, bit: bool) -> Option<u16> {
		if bit {
			self.collector |= self.bit_mask;
		}
		self.ticks = 0;
		// Was that the last bit we needed?
		if self.bit_mask == 0b100_0000_0000 {
			let result = self.collector;
			self.reset();
			Some(result)
		} else {
			self.bit_mask <<= 1;
			None
		}
	}

	/// Check 11-bit word has 1 start bit, 1 stop bit and an odd parity bit.
	///
	/// If so, you get back the 8 bit data within the word. Otherwise you get
	/// None.
	pub fn check_word(word: u16) -> Option<u8> {
		let start_bit = (word & 0b000_0000_0001) != 0;
		let parity_bit = (word & 0b010_0000_0000) != 0;
		let stop_bit = (word & 0b100_0000_0000) != 0;
		let data = ((word >> 1) & 0xFF) as u8;

		if start_bit {
			return None;
		}

		if !stop_bit {
			return None;
		}

		let need_parity = (data.count_ones() % 2) == 0;

		// Check we have the correct parity bit
		if need_parity != parity_bit {
			return None;
		}

		Some(data)
	}
}
