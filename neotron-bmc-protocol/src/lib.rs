#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]

// ============================================================================
// Modules and Imports
// ============================================================================

#[cfg(feature = "defmt")]
use defmt::Format;

mod crc;

// ============================================================================
// Traits
// ============================================================================

/// Marks an object as being sendable over a byte-oriented communications link.
pub trait Sendable {
	/// Convert to bytes for transmission.
	///
	/// Copies into the given buffer, giving an error if it isn't large enough.
	fn render_to_buffer(&self, buffer: &mut [u8]) -> Result<usize, Error>;
}

/// Marks an object as being receivable over a byte-oriented communications link.
pub trait Receivable<'a>: Sized {
	/// Convert from received bytes.
	///
	/// You get `Err` if `data` is not long enough, or if there was a CRC error.
	fn from_bytes(data: &'a [u8]) -> Result<Self, Error> {
		let crc = calculate_crc(data);
		Self::from_bytes_with_crc(data, crc)
	}

	/// Convert from received bytes and a pre-calculated CRC.
	///
	/// You get `Err` if `data` is not long enough, or if there was a CRC error.
	fn from_bytes_with_crc(data: &'a [u8], calc_crc: u8) -> Result<Self, Error>;
}

// ============================================================================
// Enums
// ============================================================================

/// The ways this API can fail
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Error {
	BadCrc,
	BadLength,
	BadRequestType,
	BufferTooSmall,
	BadResponseResult,
}

/// The kinds of [`Request`] the *Host* can make to the NBMC
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, num_enum::IntoPrimitive, num_enum::TryFromPrimitive,
)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(u8)]
pub enum RequestType {
	Read = 0xC0,
	ReadAlt = 0xC1,
	ShortWrite = 0xC2,
	ShortWriteAlt = 0xC3,
	LongWrite = 0xC4,
	LongWriteAlt = 0xC5,
}

/// The NBMC returns this code to indicate whether the previous [`Request`] was
/// succesful or not.
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, num_enum::IntoPrimitive, num_enum::TryFromPrimitive,
)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(u8)]
pub enum ResponseResult {
	/// The [`Request`] was correctly understood and actioned.
	Ok = 0xA0,
	/// The [`Request`] was not correctly understood because the CRC did not match.
	///
	/// The message may have been corrupted in-flight (e.g. a byte dropped, or a bit flipped).
	CrcFailure = 0xA1,
	/// The [`Request`] was received correctly but the Request Type was not known.
	///
	/// Did you check the Protocol Version was supported?
	BadRequestType = 0xA2,
	/// The [`Request`] was received correctly but the requested Register was not known.
	///
	/// Did you check the Protocol Version was supported?
	BadRegister = 0xA3,
	/// The [`Request`] was received correctly but the given number of bytes
	/// could not be read from or written to the given Register..
	///
	/// Did you check the Protocol Version was supported?
	BadLength = 0xA4,
}

// ============================================================================
// Structs
// ============================================================================

/// A *Request* made by the *Host* to the *NBMC*
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Request {
	pub request_type: RequestType,
	pub register: u8,
	pub length_or_data: u8,
	crc: u8,
}

/// A *Response* sent by the *NBMC* in reply to a [`Request`] from a *Host*
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Response<'a> {
	pub result: ResponseResult,
	pub data: &'a [u8],
	crc: u8,
}

/// Describes the [semantic version](https://semver.org) of this implementation
/// of the NBMC interface.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct ProtocolVersion {
	major: u8,
	minor: u8,
	patch: u8,
}

// ============================================================================
// Impls
// ============================================================================

impl Request {
	/// Make a new Read Request, requesting the given register and number of
	/// bytes.
	///
	/// Setting `use_alt` to true will use the alternate Request Type. You
	/// should flip this for every successive call so that duplicate reads can
	/// be detected.
	pub fn new_read(use_alt: bool, register: u8, length: u8) -> Request {
		let mut req = Request {
			request_type: if use_alt {
				RequestType::ReadAlt
			} else {
				RequestType::Read
			},
			register,
			length_or_data: length,
			crc: 0x00,
		};
		let bytes = req.as_bytes();
		req.crc = calculate_crc(&bytes[0..=2]);
		req
	}

	/// Make a new Short Write Request, writing the given byte to the given register.
	///
	/// Setting `use_alt` to true will use the alternate Request Type. You
	/// should flip this for every successive call so that duplicate reads can
	/// be detected.
	pub fn new_short_write(use_alt: bool, register: u8, data: u8) -> Request {
		let mut req = Request {
			request_type: if use_alt {
				RequestType::ShortWriteAlt
			} else {
				RequestType::ShortWrite
			},
			register,
			length_or_data: data,
			crc: 0x00,
		};
		let bytes = req.as_bytes();
		req.crc = calculate_crc(&bytes[0..=2]);
		req
	}

	/// Make a new Long Write Request, asking for a number of bytes to be
	/// written to the given register.
	///
	/// Setting `use_alt` to true will use the alternate Request Type. You
	/// should flip this for every successive call so that duplicate reads can
	/// be detected.
	pub fn new_long_write(use_alt: bool, register: u8, length: u8) -> Request {
		let mut req = Request {
			request_type: if use_alt {
				RequestType::LongWriteAlt
			} else {
				RequestType::LongWrite
			},
			register,
			length_or_data: length,
			crc: 0x00,
		};
		let bytes = req.as_bytes();
		req.crc = calculate_crc(&bytes[0..=2]);
		req
	}

	/// Convert to bytes for transmission.
	///
	/// Produces a fixed sized buffer.
	pub const fn as_bytes(&self) -> [u8; 4] {
		[
			self.request_type as u8,
			self.register,
			self.length_or_data,
			self.crc,
		]
	}
}

impl Sendable for Request {
	/// Convert to bytes for transmission.
	///
	/// Copies into the given buffer, giving an error if it isn't large enough.
	fn render_to_buffer(&self, buffer: &mut [u8]) -> Result<usize, Error> {
		let bytes = self.as_bytes();
		if buffer.len() < bytes.len() {
			return Err(Error::BufferTooSmall);
		}
		for (src, dest) in bytes.iter().zip(buffer.iter_mut()) {
			*dest = *src;
		}
		Ok(bytes.len())
	}
}

impl<'a> Receivable<'a> for Request {
	/// Convert from received bytes.
	///
	/// You get `Err` if the bytes could not be decoded.
	///
	/// ```
	/// # use neotron_bmc_protocol::{Request, Receivable};
	/// let bytes = [0xC0, 0x11, 0x03, 0xC6];
	/// let req = Request::from_bytes(&bytes).unwrap();
	/// ```
	fn from_bytes(data: &'a [u8]) -> Result<Request, Error> {
		if data.len() < 4 {
			return Err(Error::BadLength);
		}
		Request::from_bytes_with_crc(data, calculate_crc(&data[0..=3]))
	}

	/// Convert from received bytes, when the CRC is pre-calculated.
	///
	/// Use this if you were calculating the CRC on-the-fly, e.g. with a
	/// hardware CRC calculator.
	///
	/// You get `Err` if the bytes could not be decoded.
	///
	/// ```
	/// # use neotron_bmc_protocol::{Request, Receivable};
	/// let bytes = [0xC0, 0x11, 0x03, 0xC6];
	/// let req = Request::from_bytes(&bytes).unwrap();
	/// ```
	fn from_bytes_with_crc(data: &'a [u8], calc_crc: u8) -> Result<Request, Error> {
		if data.len() < 4 {
			return Err(Error::BadLength);
		}
		if calc_crc != 0 {
			// It's a quirk of CRC-8 that including the CRC always produces a
			// result of zero.
			return Err(Error::BadCrc);
		}
		Ok(Request {
			request_type: data[0].try_into().map_err(|_| Error::BadRequestType)?,
			register: data[1],
			length_or_data: data[2],
			crc: data[3],
		})
	}
}

impl<'a> Response<'a> {
	/// Make a new OK response, with some optional data
	pub fn new_ok_with_data(data: &'a [u8]) -> Response<'a> {
		Response {
			result: ResponseResult::Ok,
			data,
			crc: {
				let mut crc = crc::init();
				crc = crc::update(crc, &[ResponseResult::Ok as u8]);
				crc = crc::update(crc, data);
				crc::finalize(crc)
			},
		}
	}

	/// Make a new error response
	pub fn new_without_data(result: ResponseResult) -> Response<'a> {
		Response {
			result,
			data: &[],
			crc: calculate_crc(&[result as u8]),
		}
	}
}

impl<'a> Sendable for Response<'a> {
	/// Convert to bytes for transmission.
	///
	/// Copies into the given buffer, giving an error if it isn't large enough.
	///
	/// ```
	/// # use neotron_bmc_protocol::{Response, ResponseResult, Sendable};
	/// let mut buffer = [0u8; 5];
	///
	/// let req = Response::new_ok_with_data(&[]);
	/// assert_eq!(req.render_to_buffer(&mut buffer).unwrap(), 2);
	/// assert_eq!(&buffer[0..=1], [0xA0, 0x69]);
	///
	/// let req = Response::new_ok_with_data(&[0x00, 0x01]);
	/// assert_eq!(req.render_to_buffer(&mut buffer).unwrap(), 4);
	/// assert_eq!(&buffer[0..=3], [0xA0, 0x00, 0x01, 0x4F]);
	///
	/// let req = Response::new_without_data(ResponseResult::BadRequestType);
	/// assert_eq!(req.render_to_buffer(&mut buffer).unwrap(), 2);
	/// assert_eq!(&buffer[0..=1], [0xA2, 0x67]);
	/// ```
	fn render_to_buffer(&self, buffer: &mut [u8]) -> Result<usize, Error> {
		let len = 1 + self.data.len() + 1;
		if buffer.len() < len {
			return Err(Error::BufferTooSmall);
		}
		buffer[0] = self.result as u8;
		for (src, dest) in self.data.iter().zip(buffer[1..].iter_mut()) {
			*dest = *src;
		}
		buffer[len - 1] = self.crc;
		Ok(len)
	}
}

impl<'a> Receivable<'a> for Response<'a> {
	/// Convert from received bytes.
	///
	/// You get `Err` if the bytes could not be decoded.
	///
	/// ```
	/// # use neotron_bmc_protocol::{Response, Receivable};
	/// let bytes = [0xA0, 0x00, 0x01, 0x4F];
	/// let req = Response::from_bytes(&bytes).unwrap();
	///
	/// ```
	fn from_bytes_with_crc(data: &'a [u8], calc_crc: u8) -> Result<Response<'a>, Error> {
		if calc_crc != 0 {
			// It's a quirk of CRC-8 that including the CRC always produces a
			// result of zero.
			return Err(Error::BadCrc);
		}
		Ok(Response {
			result: data[0].try_into().map_err(|_| Error::BadResponseResult)?,
			data: &data[1..=(data.len() - 2)],
			crc: data[data.len() - 1],
		})
	}
}

impl ProtocolVersion {
	/// Construct a new [`ProtocolVersion`].
	///
	/// This isn't a message but instead can form part of a message. For
	/// example, you should have a register address which provides the version
	/// of the NBMC protocol implemented.
	///
	/// Pass in the major version, the minor version and the patch version.
	pub const fn new(major: u8, minor: u8, patch: u8) -> ProtocolVersion {
		ProtocolVersion {
			major,
			minor,
			patch,
		}
	}

	/// Check if this [`ProtocolVersion`] is compatible with `my_version`.
	///
	/// ```
	/// # use neotron_bmc_protocol::ProtocolVersion;
	/// let my_version = ProtocolVersion::new(1, 1, 0);
	///
	/// // This is compatible.
	/// let bmc_a = ProtocolVersion::new(1, 1, 1);
	/// assert!(bmc_a.is_compatible_with(&my_version));
	///
	/// // This is incompatible - patch is too low.
	/// let bmc_b = ProtocolVersion::new(1, 0, 0);
	/// assert!(!bmc_b.is_compatible_with(&my_version));
	///
	/// // This is incompatible - major is too high.
	/// let bmc_c = ProtocolVersion::new(2, 0, 0);
	/// assert!(!bmc_c.is_compatible_with(&my_version));
	///
	/// // This is incompatible - major is too low.
	/// let bmc_d = ProtocolVersion::new(0, 1, 0);
	/// assert!(!bmc_d.is_compatible_with(&my_version));
	/// ```
	pub const fn is_compatible_with(&self, my_version: &ProtocolVersion) -> bool {
		if self.major == my_version.major {
			if self.minor > my_version.minor {
				true
			} else if self.minor == my_version.minor {
				self.patch >= my_version.patch
			} else {
				false
			}
		} else {
			false
		}
	}

	/// Convert to bytes for transmission.
	pub const fn as_bytes(&self) -> [u8; 3] {
		[self.major, self.minor, self.patch]
	}
}

impl Sendable for ProtocolVersion {
	fn render_to_buffer(&self, buffer: &mut [u8]) -> Result<usize, Error> {
		let bytes = self.as_bytes();
		if buffer.len() < bytes.len() {
			return Err(Error::BufferTooSmall);
		}
		for (src, dest) in bytes.iter().zip(buffer.iter_mut()) {
			*dest = *src;
		}
		Ok(bytes.len())
	}
}

// ============================================================================
// Functions
// ============================================================================

/// An object for calculating CRC8 values on-the-fly.
pub struct CrcCalc(u8);

impl CrcCalc {
	/// Make a new CRC calculator
	pub const fn new() -> CrcCalc {
		CrcCalc(crc::init())
	}

	/// Reset the CRC calculator
	pub fn reset(&mut self) {
		self.0 = crc::init();
	}

	/// Add one byte to the CRC calculator
	pub fn add(&mut self, byte: u8) {
		self.0 = crc::update(self.0, &[byte]);
	}

	/// Add several bytes to the CRC calculator
	pub fn add_buffer(&mut self, bytes: &[u8]) {
		self.0 = crc::update(self.0, bytes);
	}

	/// Get the CRC
	pub fn get(&self) -> u8 {
		crc::finalize(self.0)
	}
}

/// Calculates the CRC-8 of the given bytes.
///
/// ```
/// # use neotron_bmc_protocol::calculate_crc;
/// assert_eq!(calculate_crc(&[0xC0, 0x11, 0x03]), 0xC6);
/// assert_eq!(calculate_crc(&[0xA0]), 0x69);
/// assert_eq!(calculate_crc(&[0xA0, 0x69]), 0x00);
/// ```
pub fn calculate_crc(data: &[u8]) -> u8 {
	let mut crc_calc = CrcCalc::new();
	crc_calc.add_buffer(data);
	crc_calc.get()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn read_request() {
		let req = Request::new_read(false, 0x10, 0x20);
		let bytes = req.as_bytes();
		assert_eq!(bytes, [0xC0, 0x10, 0x20, 0x3A]);
		let decoded_req = Request::from_bytes(&bytes).unwrap();
		assert_eq!(req, decoded_req);
	}

	#[test]
	fn read_request_alt() {
		let req = Request::new_read(true, 0x10, 0x20);
		let bytes = req.as_bytes();
		assert_eq!(bytes, [0xC1, 0x10, 0x20, 0x51]);
		let decoded_req = Request::from_bytes(&bytes).unwrap();
		assert_eq!(req, decoded_req);
	}

	#[test]
	fn short_write_request() {
		let req = Request::new_short_write(false, 0x11, 0x22);
		let bytes = req.as_bytes();
		assert_eq!(bytes, [0xC2, 0x11, 0x22, 0xF7]);
		let decoded_req = Request::from_bytes(&bytes).unwrap();
		assert_eq!(req, decoded_req);
	}

	#[test]
	fn short_write_request_alt() {
		let req = Request::new_short_write(true, 0x11, 0x22);
		let bytes = req.as_bytes();
		assert_eq!(bytes, [0xC3, 0x11, 0x22, 0x9C]);
		let decoded_req = Request::from_bytes(&bytes).unwrap();
		assert_eq!(req, decoded_req);
	}

	#[test]
	fn long_write_request() {
		let req = Request::new_long_write(false, 0x0F, 0x50);
		let bytes = req.as_bytes();
		assert_eq!(bytes, [0xC4, 0x0F, 0x50, 0x52]);
		let decoded_req = Request::from_bytes(&bytes).unwrap();
		assert_eq!(req, decoded_req);
	}

	#[test]
	fn long_write_request_alt() {
		let req = Request::new_long_write(true, 0x0F, 0x50);
		let bytes = req.as_bytes();
		assert_eq!(bytes, [0xC5, 0x0F, 0x50, 0x39]);
		let decoded_req = Request::from_bytes(&bytes).unwrap();
		assert_eq!(req, decoded_req);
	}
}

// ============================================================================
// End of File
// ============================================================================
