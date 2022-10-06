/**
 * \file
 * Functions and types for CRC checks.
 *
 * Generated on Thu Oct  6 17:31:55 2022
 * by pycrc v0.9.2, https://pycrc.org
 * using the configuration:
 *  - Width         = 8
 *  - Poly          = 0x07
 *  - XorIn         = 0x00
 *  - ReflectIn     = False
 *  - XorOut        = 0x00
 *  - ReflectOut    = False
 *  - Algorithm     = bit-by-bit
 */

// Translated from C to Rust by hand.

pub(crate) const fn init() -> u8 {
	0x00
}

/// Update a CRC with more data
pub(crate) fn update(mut crc: u8, data: &[u8]) -> u8 {
	for c in data.iter() {
		for i in 0..8 {
			let bit = (crc & 0x80) != 0;
			crc = (crc << 1) | ((c >> (7 - i)) & 0x01);
			if bit {
				crc ^= 0x07;
			}
		}
	}
	crc
}

/// Finish the CRC calculation
pub(crate) fn finalize(mut crc: u8) -> u8 {
	for _i in 0..8 {
		let bit = (crc & 0x80) != 0;
		crc <<= 1;
		if bit {
			crc ^= 0x07;
		}
	}
	crc
}

// ============================================================================
// End of File
// ============================================================================
