//! # Neotron BMC Commands
//!
//! Definitions of all the commands supported by the BMC.

#![no_std]

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Command {
	/// # Protocol Version
	/// The NBMC protocol version, [1, 0, 0]
	/// * Length: 3
	/// * Mode: RO
	ProtocolVersion = 0x00,
	/// # Firmware Version
	/// The NBMC firmware version, as a null-padded UTF-8 string
	/// * Length: 32
	/// * Mode: RO
	FirmwareVersion = 0x01,
	/// # Interrupt Status
	/// Which interrupts are currently active, as a bitmask.
	/// * Length: 2
	/// * Mode: R/W1C
	InterruptStatus = 0x10,
	/// # Interrupt Control
	/// Which interrupts are currently enabled, as a bitmask.
	/// * Length: 2
	/// * Mode: R/W
	InterruptControl = 0x11,
	/// # Button Status
	/// The current state of the buttons
	/// * Length: 1
	/// * Mode: RO
	ButtonStatus = 0x20,
	/// # System Temperature
	/// Temperature in °C, as an `i8`
	/// * Length: 1
	/// * Mode: RO
	SystemTemperature = 0x21,
	/// # System Voltage (Standby 3.3V rail)
	/// Voltage in Volts/32, as a `u8`
	/// * Length: 1
	/// * Mode: RO
	SystemVoltage33S = 0x22,
	/// # System Voltage (Main 3.3V rail)
	/// Voltage in Volts/32, as a `u8`
	/// * Length: 1
	/// * Mode: RO
	SystemVoltage33 = 0x23,
	/// # System Voltage (5.0V rail)
	/// Voltage in Volts/32, as a `u8`
	/// * Length: 1
	/// * Mode: RO
	SystemVoltage55 = 0x24,
	/// # Power Control
	/// Enable/disable the power supply
	/// * Length: 1
	/// * Mode: R/W
	PowerControl = 0x25,
	/// # UART Receive/Transmit Buffer
	/// Data received/to be sent over the UART
	/// * Length: up to 64
	/// * Mode: FIFO
	UartBuffer = 0x30,
	/// # UART FIFO Control
	/// Settings for the UART FIFO
	/// * Length: 1
	/// * Mode: R/W
	UartFifoControl = 0x31,
	/// # UART Control
	/// Settings for the UART
	/// * Length: 1
	/// * Mode: R/W
	UartControl = 0x32,
	/// # UART Status
	/// The current state of the UART
	/// * Length: 1
	/// * Mode: R/W1C
	UartStatus = 0x33,
	/// # UART Baud Rate
	/// The UART baud rate in bps, as a `u32le`
	/// * Length: 4
	/// * Mode: R/W
	UartBaudRate = 0x34,
	/// # PS/2 Keyboard Receive/Transmit Buffer
	/// Data received/to be sent over the PS/2 keyboard port
	/// * Length: up to 16
	/// * Mode: FIFO
	Ps2KbBuffer = 0x40,
	/// # PS/2 Keyboard Control
	/// Settings for the PS/2 Keyboard port
	/// * Length: 1
	/// * Mode: R/W
	Ps2KbControl = 0x41,
	/// # PS/2 Keyboard Status
	/// Current state of the PS/2 Keyboard port
	/// * Length: 1
	/// * Mode: R/W1C
	Ps2KbStatus = 0x42,
	/// # PS/2 Mouse Receive/Transmit Buffer
	/// Data received/to be sent over the PS/2 Mouse port
	/// * Length: up to 16
	/// * Mode: FIFO
	Ps2MouseBuffer = 0x50,
	/// # PS/2 Mouse Control
	/// Settings for the PS/2 Mouse port
	/// * Length: 1
	/// * Mode: R/W
	Ps2MouseControl = 0x51,
	/// # PS/2 Mouse Status
	/// Current state of the PS/2 Mouse port
	/// * Length: 1
	/// * Mode: R/W1C
	Ps2MouseStatus = 0x52,
	/// # I²C Receive/Transmit Buffer
	/// Data received/to be sent over the I²C Bus
	/// * Length: up to 16
	/// * Mode: FIFO
	I2cBuffer = 0x60,
	/// # I²C FIFO Control
	/// Settings for the I²C FIFO
	/// * Length: 1
	/// * Mode: R/W
	I2cFifoControl = 0x61,
	/// # I²C Control
	/// Settings for the I²C Bus
	/// * Length: 1
	/// * Mode: R/W
	I2cControl = 0x62,
	/// # I²C Status
	/// Current state of the I²C Bus
	/// * Length: 1
	/// * Mode: R/W1C
	I2cStatus = 0x63,
	/// # I²C Baud Rate
	/// The I²C clock rate in Hz, as a `u32le`
	/// * Length: 4
	/// * Mode: R/W
	I2cBaudRate = 0x64,
}

impl Command {
	pub fn parse(byte: u8) -> Option<Command> {
		match byte {
			0x00 => Some(Command::ProtocolVersion),
			0x01 => Some(Command::FirmwareVersion),
			0x10 => Some(Command::InterruptStatus),
			0x11 => Some(Command::InterruptControl),
			0x20 => Some(Command::ButtonStatus),
			0x21 => Some(Command::SystemTemperature),
			0x22 => Some(Command::SystemVoltage33S),
			0x23 => Some(Command::SystemVoltage33),
			0x24 => Some(Command::SystemVoltage55),
			0x25 => Some(Command::PowerControl),
			0x30 => Some(Command::UartBuffer),
			0x31 => Some(Command::UartFifoControl),
			0x32 => Some(Command::UartControl),
			0x33 => Some(Command::UartStatus),
			0x34 => Some(Command::UartBaudRate),
			0x40 => Some(Command::Ps2KbBuffer),
			0x41 => Some(Command::Ps2KbControl),
			0x42 => Some(Command::Ps2KbStatus),
			0x50 => Some(Command::Ps2MouseBuffer),
			0x51 => Some(Command::Ps2MouseControl),
			0x52 => Some(Command::Ps2MouseStatus),
			0x60 => Some(Command::I2cBuffer),
			0x61 => Some(Command::I2cFifoControl),
			0x62 => Some(Command::I2cControl),
			0x63 => Some(Command::I2cStatus),
			0x64 => Some(Command::I2cBaudRate),
			_ => None,
		}
	}
}
