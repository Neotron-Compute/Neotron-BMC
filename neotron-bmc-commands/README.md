# Neotron-BMC-Commands

Command codes for communication with the Neotron Board Management Controller (NBMC).

## System Registers

| Address | Name                                  | Type  | Contains                                                 | Length   |
| :-----: | ------------------------------------- | :---: | -------------------------------------------------------- | :------: |
| 0x00    | Protocol Version                      | RO    | The NBMC protocol version, [1, 0, 0]                     | 3        |
| 0x01    | Firmware Version                      | RO    | The NBMC firmware version, as a null-padded UTF-8 string | 32       |
| 0x10    | Interrupt Status                      | R/W1C | Which interrupts are currently active, as a bitmask.     | 2        |
| 0x11    | Interrupt Control                     | R/W   | Which interrupts are currently enabled, as a bitmask.    | 2        |
| 0x20    | Button Status                         | RO    | The current state of the buttons                         | 1        |
| 0x21    | System Temperature                    | RO    | Temperature in °C, as an `i8`                            | 1        |
| 0x22    | System Voltage (Standby 3.3V rail)    | RO    | Voltage in Volts/32, as a `u8`                           | 1        |
| 0x23    | System Voltage (Main 3.3V rail)       | RO    | Voltage in Volts/32, as a `u8`                           | 1        |
| 0x24    | System Voltage (5.0V rail)            | RO    | Voltage in Volts/32, as a `u8`                           | 1        |
| 0x25    | Power Control                         | R/W   | Enable/disable the power supply                          | 1        |
| 0x30    | UART Receive/Transmit Buffer          | FIFO  | Data received/to be sent over the UART                   | up to 64 |
| 0x31    | UART FIFO Control                     | R/W   | Settings for the UART FIFO                               | 1        |
| 0x32    | UART Control                          | R/W   | Settings for the UART                                    | 1        |
| 0x33    | UART Status                           | R/W1C | The current state of the UART                            | 1        |
| 0x34    | UART Baud Rate                        | R/W   | The UART baud rate in bps, as a `u32le`                  | 4        |
| 0x40    | PS/2 Keyboard Receive/Transmit Buffer | FIFO  | Data received/to be sent over the PS/2 keyboard port     | up to 16 |
| 0x41    | PS/2 Keyboard Control                 | R/W   | Settings for the PS/2 Keyboard port                      | 1        |
| 0x42    | PS/2 Keyboard Status                  | R/W1C | Current state of the PS/2 Keyboard port                  | 1        |
| 0x50    | PS/2 Mouse Receive/Transmit Buffer    | FIFO  | Data received/to be sent over the PS/2 Mouse port        | up to 16 |
| 0x51    | PS/2 Mouse Control                    | R/W   | Settings for the PS/2 Mouse port                         | 1        |
| 0x52    | PS/2 Mouse Status                     | R/W1C | Current state of the PS/2 Mouse port                     | 1        |
| 0x60    | I²C Receive/Transmit Buffer           | FIFO  | Data received/to be sent over the I²C Bus                | up to 16 |
| 0x61    | I²C FIFO Control                      | R/W   | Settings for the I²C FIFO                                | 1        |
| 0x62    | I²C Control                           | R/W   | Settings for the I²C Bus                                 | 1        |
| 0x63    | I²C Status                            | R/W1C | Current state of the I²C Bus                             | 1        |
| 0x64    | I²C Baud Rate                         | R/W   | The I²C clock rate in Hz, as a `u32le`                   | 4        |
| 0x70    | Speaker Tone Duration                 | R/W   | Duration of the note, in units of 10ms (0 = stop playing)| 1        |
| 0x71    | Speaker Tone Period (high)            | R/W   | Period of note (in 48kHz ticks), MSB                     | 1        |
| 0x72    | Speaker Tone Period (low)             | R/W   | Period of note (in 48kHz ticks), LSB                     | 1        |
| 0x73    | Speaker Tone Duty Cycle               | R/W   | Duty cycle of speaker PWM square wave (127 = 50%)        | 1        |

The register types are:

* `RO` - read only register, where writes will return an error
* `R/W` - read/write register
* `R/W1C` - reads as usual, but when writing a 1 bit clears that bit position and a 0 bit is ignored
* `FIFO` - a first-in, first-out buffer

### Address 0x00 - Protocol Version

This read-only register returns the protocol version supported. The protocol
version includes the set of registers, and the meaning of the fields within
those registers. A *Host* should first verify that the *NBMC* it is talking to
is semantically compatible before reading any other registers.

The three bytes are `major`, `minor` and `patch`. This document corresponds to
`[1, 0, 0]` (or *v1.0.0*).

### Address 0x01 - Firmware Version

This read-only register returns the firmware version of the NBMC, as a UTF-8
string. The register length is always 32 bytes, and the string is null-padded.
We also guarantee that the firmware version will always be less than or equal to
31 bytes, so you can also treat this string as null-terminated.

An official release will have a version string of the form `tags/v1.2.3`. An
unofficial release might be `heads/develop-dirty`. It is not recommended that
you rely on these formats or attempt to parse the version string. It is however
useful if you can quote this string when reporting issues with the firmware.

### Address 0x10 - Interrupt Status

This eight bit register indicates which Interrupts are currently 'active'. An
Interrupt will remain 'active' until a word is written to this register with a 1
bit in the relevant position.

| Bit | Interrupt                  |
| --- | -------------------------- |
| 7   | Voltage Alarm              |
| 6   | Button State Change        |
| 5   | UART TX Empty              |
| 4   | UART RX Not Empty          |
| 3   | I²C TX Empty               |
| 2   | I²C RX Not Empty           |
| 1   | PS/2 Mouse RX Not Empty    |
| 0   | PS/2 Keyboard RX Not Empty |

### Address 0x11 - Interrupt Control

This eight bit register indicates which Interrupts are currently 'enabled'. The
IRQ_nHOST signal is a level interrupt and it will be active (LOW) whenever the
value in the Interrupt Control register ANDed with the Interrupt Status register
is non-zero.

The bits have the same ordering as the Interrupt Status register.

### Address 0x20 - Button Status

This eight-bit register indicates the state of the power button.

Note that if the power button is held down for three seconds, the system will
power-off instantly, regardless of what the host does.

Note also that is it not possible to sample the reset button - pressing the
reset button will instantly assert the system reset line, rebooting the Host.

| Bits | Meaning                               |
| ---- | ------------------------------------- |
| 7-1  | Reserved for future use               |
| 0    | Power Button: 0 = normal, 1 = pressed |

### Address 0x21 - System Temperature

This eight-bit register provides the current system temperature in °C, as
measured on the STM32's internal temperature sensor. It is updated around once a
second.

### Address 0x22 - System Voltage (Standby 3.3V rail)

This eight-bit register provides the current 3.3V rail voltage in units of 1/32
of a Volt. It is updated around once a second. A value of 105 (3.28V) to 106
(3.31V) is nominal. An interrupt is raised when the value exceeds 3.63V (116) or
is lower than 2.97V (95).

### Address 0x23 - System Voltage (Main 3.3V rail)

This eight-bit register provides the current 3.3V rail voltage in units of 1/32
of a Volt. It is updated around once a second. A value of 105 (3.28V) to 106
(3.31V) is nominal. An interrupt is raised when the value exceeds 3.63V (116) or
is lower than 2.97V (95).

### Address 0x24 - System Voltage (5.0V rail)

This eight-bit register provides the current 3.3V rail voltage in units of 1/32
of a Volt. It is updated around once a second. A value of 160 (5.00V) is
nominal. An interrupt is raised when the value exceeds 5.5V (176) or is lower
than 4.5V (144).

### Address 0x25 - Power Control

This eight-bit register controls the main DC/DC power supply unit. The Host
should disable the DC/DC supply (by writing zero here) if it wishes to power
down.

| Bits | Meaning                        |
| ---- | ------------------------------ |
| 7-1  | Reserved for future use        |
| 0    | DC/DC control: 0 = off, 1 = on |

### Address 0x30 - UART Receive/Transmit Buffer

TODO

### Address 0x31 - UART FIFO Control

TODO

### Address 0x32 - UART Control

TODO

### Address 0x33 - UART Status

TODO

### Address 0x34 - UART Baud Rate

TODO

### Address 0x40 - PS/2 Keyboard Receive/Transmit Buffer

TODO

### Address 0x41 - PS/2 Keyboard Control

TODO

### Address 0x42 - PS/2 Keyboard Status

TODO

### Address 0x50 - PS/2 Mouse Receive/Transmit Buffer

TODO

### Address 0x51 - PS/2 Mouse Control

TODO

### Address 0x52 - PS/2 Mouse Status

TODO

### Address 0x60 - I²C Receive/Transmit Buffer

TODO

### Address 0x61 - I²C FIFO Control

TODO

### Address 0x62 - I²C Control

TODO

### Address 0x63 - I²C Status

TODO

### Address 0x64 - I²C Baud Rate

TODO

### Address 0x70 - Speaker Tone Duration

Sets the duration of the tone to be played, and starts the tone playing. You
should set the other three registers (if required) before setting this register.

There is no way to know when the tone is ended; the host should keep track of
the duration it set and wait the appropriate period of time.

### Address 0x71 - Speaker Tone Period (High)

Sets the upper 8 bits of the tone period. This is the inverse of frequency, in 48 kHz units.

### Address 0x72 - Speaker Tone Period (Low)

Sets the lower 8 bits of the tone period. See *Speaker Tone Period (High)* for details.

### Address 0x73 - Speaker Tone Duty Cycle

Sets the duty-cycle of the speaker tone. A value of 127 is 50:50 (a square wave).
