//! # SPI Driver for STM32
//!
//! Unlike the HAL, this implement 'SPI Peripheral Mode', i.e. for when the
//! clock signal is an input and not an output.

use stm32f0xx_hal::{pac, prelude::*, rcc::Rcc};

pub struct SpiPeripheral<const RXC: usize, const TXC: usize> {
	/// Our PAC object for register access
	dev: pac::SPI1,
	/// A space for bytes received from the host
	rx_buffer: [u8; RXC],
	/// How many bytes have been received?
	rx_idx: usize,
	/// How many bytes do we want?
	rx_want: usize,
	/// A space for data we're about to send
	tx_buffer: [u8; TXC],
	/// How many bytes have been played from the TX buffer
	tx_idx: usize,
	/// How many bytes are loaded into the TX buffer
	tx_ready: usize,
	/// The in-progress RX CRC
	rx_crc: neotron_bmc_protocol::CrcCalc,
}

impl<const RXC: usize, const TXC: usize> SpiPeripheral<RXC, TXC> {
	const MODE: embedded_hal::spi::Mode = embedded_hal::spi::MODE_0;

	/// Construct a new driver
	pub fn new<SCKPIN, MISOPIN, MOSIPIN>(
		dev: pac::SPI1,
		pins: (SCKPIN, MISOPIN, MOSIPIN),
		rcc: &mut Rcc,
	) -> SpiPeripheral<RXC, TXC>
	where
		SCKPIN: stm32f0xx_hal::spi::SckPin<pac::SPI1>,
		MISOPIN: stm32f0xx_hal::spi::MisoPin<pac::SPI1>,
		MOSIPIN: stm32f0xx_hal::spi::MosiPin<pac::SPI1>,
	{
		defmt::info!("pclk = {}", rcc.clocks.pclk().0,);

		// Set SPI up in Controller mode. This will cause the HAL to enable the clocks and power to the IP block.
		// It also checks the pins are OK.
		let spi_controller =
			stm32f0xx_hal::spi::Spi::spi1(dev, pins, Self::MODE, 8_000_000u32.hz(), rcc);
		// Now disassemble the driver so we can set it into Controller mode instead
		let (dev, _pins) = spi_controller.release();

		let mut spi = SpiPeripheral {
			dev,
			rx_buffer: [0u8; RXC],
			rx_idx: 0,
			rx_want: 0,
			tx_buffer: [0u8; TXC],
			tx_idx: 0,
			tx_ready: 0,
			rx_crc: neotron_bmc_protocol::CrcCalc::new(),
		};

		spi.config(Self::MODE);

		// Empty the receive register
		while spi.has_rx_data() {
			let _ = spi.raw_read();
		}

		// Enable the SPI device
		spi.stop();
		spi.dev.cr1.write(|w| {
			// Enable the peripheral
			w.spe().enabled();
			w
		});

		spi
	}

	/// Set up the registers
	fn config(&mut self, mode: embedded_hal::spi::Mode) {
		// We are following DM00043574, Section 30.5.1 Configuration of SPI

		// 1. Disable SPI
		self.dev.cr1.modify(|_r, w| {
			w.spe().disabled();
			w
		});

		// 2. Write to the SPI_CR1 register. Apologies for the outdated terminology.
		self.dev.cr1.write(|w| {
			// 2a. Configure the serial clock baud rate (ignored in peripheral mode)
			w.br().div2();
			// 2b. Configure the CPHA and CPOL bits.
			if mode.phase == embedded_hal::spi::Phase::CaptureOnSecondTransition {
				w.cpha().second_edge();
			} else {
				w.cpha().first_edge();
			}
			if mode.polarity == embedded_hal::spi::Polarity::IdleHigh {
				w.cpol().idle_high();
			} else {
				w.cpol().idle_low();
			}
			// 2c. Select simplex or half-duplex mode (nope, neither of those)
			w.rxonly().clear_bit();
			w.bidimode().clear_bit();
			w.bidioe().clear_bit();
			// 2d. Configure the LSBFIRST bit to define the frame format
			w.lsbfirst().clear_bit();
			// 2e. Configure the CRCL and CRCEN bits if CRC is needed (it is not)
			w.crcen().disabled();
			// 2f. Turn on soft-slave-management (SSM) (we control the NSS signal with the SSI bit).
			w.ssm().enabled();
			w.ssi().slave_not_selected();
			// 2g. Set the Master bit low for slave mode
			w.mstr().slave();
			w
		});

		// 3. Write to SPI_CR2 register
		self.dev.cr2.write(|w| {
			// 3a. Configure the DS[3:0] bits to select the data length for the transfer (0b111 = 8-bit words).
			unsafe { w.ds().bits(0b111) };
			// 3b. Disable hard-output on the CS pin (ignored in Master mode)
			w.ssoe().disabled();
			// 3c. Frame Format
			w.frf().motorola();
			// 3d. Set NSSP bit if required (we don't want NSS Pulse mode)
			w.nssp().no_pulse();
			// 3e. Configure the FIFO RX Threshold to 1/4 FIFO (8 bits)
			w.frxth().quarter();
			// 3f. Disable DMA mode
			w.txdmaen().disabled();
			w.rxdmaen().disabled();
			// Extra: Turn on RX and Error interrupts, but not TX. We swap
			// interrupts once the read phase is complete.
			w.rxneie().masked();
			w.txeie().masked();
			w.errie().masked();
			w
		});

		// 4. SPI_CRCPR - not required

		// 5. DMA registers - not required
	}

	/// Enable the SPI peripheral (i.e. when CS goes low).
	///
	/// We tell it how many bytes we are expecting, so it knows when to update
	/// the main thread.
	pub fn start(&mut self, num_bytes: usize) {
		if num_bytes > RXC {
			panic!("Read too large");
		}
		self.rx_idx = 0;
		self.rx_want = num_bytes as usize;
		self.tx_idx = 0;
		self.tx_ready = 0;
		self.rx_crc.reset();
		// Empty the receive register
		while self.has_rx_data() {
			let _ = self.raw_read();
		}
		// Turn on RX interrupt; turn off TX interrupt
		self.dev.cr2.write(|w| {
			w.rxneie().not_masked();
			w.txeie().masked();
			w
		});
		// Tell the SPI engine it has a chip-select
		self.dev.cr1.modify(|_r, w| {
			w.ssi().slave_selected();
			w
		});
	}

	/// Disable the SPI peripheral (i.e. when CS goes high)
	pub fn stop(&mut self) {
		self.dev.cr1.modify(|_r, w| {
			w.ssi().slave_not_selected();
			w
		});
	}

	/// Fully reset the SPI peripheral
	pub fn reset(&mut self, _rcc: &mut stm32f0xx_hal::rcc::Rcc) {
		self.dev.cr1.write(|w| {
			// Disable the peripheral
			w.spe().disabled();
			w
		});

		// Reset the IP manually. This is OK as we have exclusive access to the
		// RCC peripheral. But sadly the RCC peripheral doesn't let us reset
		// anything (it assumes it can handle it all internally).
		let reset_reg = 0x4002_100C as *mut u32;
		let spi1_bit = 1 << 12;
		unsafe {
			*reset_reg |= spi1_bit;
			*reset_reg &= !(spi1_bit);
		}

		// Reconfigure
		self.config(Self::MODE);

		// Empty the receive register
		while self.has_rx_data() {
			let _ = self.raw_read();
		}

		// Enable the SPI device and leave it idle
		self.stop();
		self.dev.cr1.write(|w| {
			// Enable the peripheral
			w.spe().enabled();
			w
		});
	}

	/// Does the RX FIFO have any data in it?
	fn has_rx_data(&self) -> bool {
		self.dev.sr.read().rxne().is_not_empty()
	}

	fn raw_read(&mut self) -> u8 {
		// PAC only supports 16-bit read, but that pops two bytes off the FIFO.
		// So force an 8-bit read.
		unsafe { core::ptr::read_volatile(&self.dev.dr as *const _ as *const u8) }
	}

	fn raw_write(&mut self, data: u8) {
		// PAC only supports 16-bit read, but that pushes two bytes onto the FIFO.
		// So force an 8-bit write.
		unsafe { core::ptr::write_volatile(&self.dev.dr as *const _ as *mut u8, data) }
	}

	/// Get a slice of data received so far.
	pub fn get_received(&self) -> Option<(&[u8], u8)> {
		Some(((&self.rx_buffer[0..self.rx_idx]), self.rx_crc.get()))
	}

	/// Call this when the SPI peripheral interrupt fires.
	///
	/// It will handle incoming bytes and/or outgoing bytes, depending on what
	/// phase we are in.
	pub fn handle_isr(&mut self) -> bool {
		let mut have_packet = false;
		let irq_status = self.dev.sr.read();
		if irq_status.rxne().is_not_empty() {
			if self.rx_isr() {
				// We've got enough, turn the RX interrupt off. Everything else
				// we receive is going to be garbage.
				self.dev.cr2.write(|w| {
					w.rxneie().masked();
					w
				});
				have_packet = true;
			}
		}
		if irq_status.txe().is_empty() {
			self.tx_isr();
		}
		have_packet
	}

	/// Try and read from the SPI FIFO
	fn rx_isr(&mut self) -> bool {
		let byte = self.raw_read();
		if self.rx_want == 0 {
			panic!("unwanted data 0x{:02x}", byte);
		}
		if self.rx_idx < self.rx_buffer.len() {
			self.rx_buffer[self.rx_idx] = byte;
			self.rx_crc.add(byte);
			self.rx_idx += 1;
		}
		self.rx_idx == self.rx_want
	}

	/// Call this in the TXEIE interrupt. It will load the SPI FIFO with some
	/// data, either from `tx_buffer` or a padding byte.
	fn tx_isr(&mut self) {
		if self.tx_idx < self.tx_ready {
			// We have some data yet to send. This is safe as long as we do the
			// bounds check when we set `self.tx_ready`.
			let next_tx = unsafe { *self.tx_buffer.get_unchecked(self.tx_idx) };
			self.raw_write(next_tx);
			self.tx_idx += 1;
		} else {
			// No data - send 0x00
			self.raw_write(0x00);
		}
	}

	/// Load some data into the TX buffer.
	///
	/// You get an error if you try to load too much.
	pub fn set_transmit(&mut self, data: &[u8]) -> Result<(), usize> {
		self.tx_ready = 0;
		self.tx_idx = 0;
		if data.len() > TXC {
			// Too much data. This check is important for safety in
			// [`Self::tx_isr`].
			return Err(TXC);
		}
		for (inc, space) in data.iter().zip(self.tx_buffer.iter_mut()) {
			*space = *inc;
		}
		// We must never set this to be longer than `TXC` as we do an unchecked
		// read from `self.tx_buffer` in [`Self::tx_isr`].
		self.tx_ready = data.len();
		// Turn on the TX interrupt
		self.dev.cr2.write(|w| {
			w.txeie().not_masked();
			w
		});
		Ok(())
	}

	/// Render some message into the TX buffer.
	///
	/// You get an error if you try to load too much.
	pub fn set_transmit_sendable(
		&mut self,
		message: &dyn neotron_bmc_protocol::Sendable,
	) -> Result<(), ()> {
		self.tx_ready = 0;
		self.tx_idx = 0;

		match message.render_to_buffer(&mut self.tx_buffer) {
			Ok(n) => {
				// We must never set this to be longer than `TXC` as we do an
				// unchecked read from `self.tx_buffer` in [`Self::tx_isr`].
				self.tx_ready = n.min(TXC);
				// Turn on the TX interrupt
				self.dev.cr2.write(|w| {
					w.txeie().not_masked();
					w
				});
				Ok(())
			}
			Err(_) => Err(()),
		}
	}
}
