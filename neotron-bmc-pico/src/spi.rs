//! # SPI Driver for STM32
//!
//! Unlike the HAL, this implement 'SPI Peripheral Mode', i.e. for when the
//! clock signal is an input and not an output.

use core::convert::TryInto;

use stm32f0xx_hal::{pac, prelude::*, rcc::Rcc};

pub struct SpiPeripheral<const RXC: usize, const TXC: usize> {
	dev: pac::SPI1,
	/// A space for bytes received from the host
	rx_buffer: [u8; RXC],
	/// How many bytes have been received?
	rx_idx: usize,
	/// A space for data we're about to send
	tx_buffer: [u8; TXC],
	/// How many bytes have been played from the TX buffer
	tx_idx: usize,
	/// How many bytes are loaded into the TX buffer
	tx_ready: usize,
	/// Has the RX been processed?
	is_done: bool,
}

impl<const RXC: usize, const TXC: usize> SpiPeripheral<RXC, TXC> {
	pub fn new<SCKPIN, MISOPIN, MOSIPIN>(
		dev: pac::SPI1,
		pins: (SCKPIN, MISOPIN, MOSIPIN),
		speed_hz: u32,
		rcc: &mut Rcc,
	) -> SpiPeripheral<RXC, TXC>
	where
		SCKPIN: stm32f0xx_hal::spi::SckPin<pac::SPI1>,
		MISOPIN: stm32f0xx_hal::spi::MisoPin<pac::SPI1>,
		MOSIPIN: stm32f0xx_hal::spi::MosiPin<pac::SPI1>,
	{
		defmt::info!(
			"pclk = {}, incoming spi_clock = {}",
			rcc.clocks.pclk().0,
			speed_hz
		);

		let mode = embedded_hal::spi::MODE_0;

		// Set SPI up in Controller mode. This will cause the HAL to enable the clocks and power to the IP block.
		// It also checks the pins are OK.
		let spi_controller = stm32f0xx_hal::spi::Spi::spi1(dev, pins, mode, 8_000_000u32.hz(), rcc);
		// Now disassemble the driver so we can set it into Controller mode instead
		let (dev, _pins) = spi_controller.release();

		// We are following DM00043574, Section 30.5.1 Configuration of SPI

		// 1. Disable SPI
		dev.cr1.modify(|_r, w| {
			w.spe().disabled();
			w
		});

		// 2. Write to the SPI_CR1 register. Apologies for the outdated terminology.
		dev.cr1.write(|w| {
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
			// 2f. Turn off soft-slave-management (SSM) and slave-select-internal (SSI)
			w.ssm().disabled();
			w.ssi().slave_selected();
			// 2g. Set the Master bit low for slave mode
			w.mstr().slave();
			w
		});

		// 3. Write to SPI_CR2 register
		dev.cr2.write(|w| {
			// 3a. Configure the DS[3:0] bits to select the data length for the transfer.
			unsafe { w.ds().bits(0b111) };
			// 3b. Disable hard-output on the CS pin
			w.ssoe().disabled();
			// 3c. Frame Format
			w.frf().motorola();
			// 3d. Set NSSP bit if required (we don't want NSS Pulse mode)
			w.nssp().no_pulse();
			// 3e. Configure the FIFO RX Threshold to 1/4 FIFO
			w.frxth().quarter();
			// 3f. LDMA_TX and LDMA_RX for DMA mode - not used
			// Extra: Turn on RX Not Empty Interrupt Enable
			w.rxneie().set_bit();
			w
		});

		// 4. SPI_CRCPR - not required

		// 5. DMA registers - not required

		let mut spi = SpiPeripheral {
			dev,
			rx_buffer: [0u8; RXC],
			rx_idx: 0,
			tx_buffer: [0u8; TXC],
			tx_idx: 0,
			tx_ready: 0,
			is_done: false,
		};

		// Empty the receive register
		while spi.has_rx_data() {
			let _ = spi.raw_read();
		}

		spi
	}

	/// Enable the SPI peripheral (i.e. when CS is low)
	pub fn enable(&mut self) {
		self.rx_idx = 0;
		self.tx_idx = 0;
		self.tx_ready = 0;
		self.is_done = false;
		self.dev.cr1.modify(|_r, w| {
			w.spe().enabled();
			w
		});
		// Load our dummy byte (our TX FIFO will send this then repeat it whilst
		// it underflows during the receive phase).
		self.raw_write(0xFF);
		// Get an IRQ when there's RX data available
		self.enable_rxne_irq();
		// self.disable_txe_irq();
	}

	/// Disable the SPI peripheral (i.e. when CS is high)
	pub fn disable(&mut self) {
		self.disable_txe_irq();
		self.disable_rxne_irq();
		self.dev.cr1.modify(|_r, w| {
			w.spe().disabled();
			w
		});
	}

	/// Enable TX Empty interrupt
	fn enable_txe_irq(&mut self) {
		self.dev.cr2.modify(|_r, w| {
			w.txeie().set_bit();
			w
		});
	}

	/// Disable TX Empty interrupt
	fn disable_txe_irq(&mut self) {
		self.dev.cr2.modify(|_r, w| {
			w.txeie().clear_bit();
			w
		});
	}

	/// Enable RX Not Empty interrupt
	fn enable_rxne_irq(&mut self) {
		self.dev.cr2.modify(|_r, w| {
			w.rxneie().set_bit();
			w
		});
	}

	/// Disable RX Not Empty interrupt
	fn disable_rxne_irq(&mut self) {
		self.dev.cr2.modify(|_r, w| {
			w.rxneie().clear_bit();
			w
		});
	}

	/// Does the RX FIFO have any data in it?
	fn has_rx_data(&self) -> bool {
		self.dev.sr.read().rxne().is_not_empty()
	}

	fn raw_read(&mut self) -> u8 {
		// PAC only supports 16-bit read, but that pops two bytes off the FIFO.
		// So force a 16-bit read.
		unsafe { core::ptr::read_volatile(&self.dev.dr as *const _ as *const u8) }
	}

	fn raw_write(&mut self, data: u8) {
		// PAC only supports 16-bit read, but that pops two bytes off the FIFO.
		// So force a 16-bit read.
		unsafe { core::ptr::write_volatile(&self.dev.dr as *const _ as *mut u8, data) }
	}

	/// Get a slice of data received so far.
	pub fn get_received(&self) -> Option<&[u8]> {
		if !self.is_done {
			Some(&self.rx_buffer[0..self.rx_idx])
		} else {
			None
		}
	}

	/// Mark the RX as processed, so we don't do it again (until the SPI is
	/// disabled and re-enabled by the next chip select).
	pub fn mark_done(&mut self) {
		self.is_done = true;
	}

	pub fn handle_isr(&mut self) {
		let irq_status = self.dev.sr.read();
		if irq_status.rxne().is_not_empty() {
			self.read_isr();
		}
		if irq_status.txe().is_empty() {
			self.tx_isr();
		}
	}

	/// Try and read from the SPI FIFO
	///
	/// If we read some data, we also load any waiting 'reply byte'.
	fn read_isr(&mut self) {
		let cmd = self.raw_read();
		if self.rx_idx < self.rx_buffer.len() {
			self.rx_buffer[self.rx_idx] = cmd;
			self.rx_idx += 1;
		}
	}

	/// Call this in the TXEIE interrupt. It will load the SPI FIFO with some
	/// data, either from `tx_buffer` or a padding byte.
	fn tx_isr(&mut self) {
		if (self.tx_idx < self.tx_ready) && (self.tx_idx < self.tx_buffer.len()) {
			// We have some data yet to send
			let next_tx = self.tx_buffer[self.tx_idx];
			self.raw_write(next_tx);
			self.tx_idx += 1;
		} else {
			// No data - send padding
			self.raw_write(0xFF);
		}
	}

	/// Load some data into the TX buffer.
	///
	/// You get an error if you try to load too much.
	pub fn set_transmit(&mut self, data: &[u8]) -> Result<(), usize> {
		self.tx_ready = 0;
		self.tx_idx = 0;
		if data.len() > TXC {
			// Too much data
			return Err(TXC);
		}
		for (inc, space) in data.iter().zip(self.tx_buffer.iter_mut()) {
			*space = *inc;
		}
		self.tx_ready = data.len();
		// Start the IRQ driven data transmission
		// self.enable_txe_irq();
		Ok(())
	}
}
