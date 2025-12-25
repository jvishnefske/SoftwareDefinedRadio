# Software Specification: Embedded SDR Amateur Radio Transceiver

## Document Information
- **Version:** 1.0 Draft
- **Target Platform:** STM32L4 Series (ARM Cortex-M4F)
- **Language:** Embedded Rust (safe, verifiable)
- **Reference Designs:** (tr)uSDX, QRPLabs QMX

---

## 1. System Overview

### 1.1 Product Concept

A compact, multi-mode HF amateur radio transceiver implementing software-defined radio (SDR) architecture where DSP replaces traditional analog signal processing. The design supports two primary operating modes:

| Mode | Description | Primary Interface |
|------|-------------|-------------------|
| **Portable** | Standalone battery operation | OLED display + encoder/buttons |
| **PC-Connected** | Tethered to host computer | USB IQ streaming + CAT control |

### 1.2 Key Design Philosophy

- **DSP-Centric Architecture:** Maximize software processing to minimize analog hardware complexity
- **Safe Embedded Rust:** Memory-safe firmware with compile-time verification
- **Dual-Mode Operation:** Seamless transition between standalone and PC-connected modes
- **Minimal MVP Display:** Basic OLED sufficient for portable operation; rich UI via PC software

### 1.3 Frequency Coverage

| Band | Frequency (MHz) | Wavelength |
|------|-----------------|------------|
| 80m | 3.5 - 4.0 | 80 meters |
| 40m | 7.0 - 7.3 | 40 meters |
| 30m | 10.1 - 10.15 | 30 meters |
| 20m | 14.0 - 14.35 | 20 meters |
| 17m | 18.068 - 18.168 | 17 meters |
| 15m | 21.0 - 21.45 | 15 meters |

---

## 2. Hardware Architecture

### 2.1 Block Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        ANTENNA                                          │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                    ┌────────▼────────┐
                    │   T/R Switch    │
                    │   (RF Relay)    │
                    └───┬─────────┬───┘
                        │         │
              ┌─────────▼───┐ ┌───▼─────────┐
              │  RX Path    │ │  TX Path    │
              │  LPF Bank   │ │  LPF Bank   │
              └─────────┬───┘ └───┬─────────┘
                        │         │
              ┌─────────▼───┐ ┌───▼─────────┐
              │  Quadrature │ │  Class-E    │
              │  Sampling   │ │  PA Stage   │
              │  Detector   │ │  (BS170/    │
              │  (FST3253)  │ │  FDT86256)  │
              └─────────┬───┘ └───▲─────────┘
                        │         │
┌───────────────────────▼─────────┴───────────────────────────────────────┐
│                         STM32L4 MCU                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │  ADC     │  │  DAC     │  │  DSP     │  │  USB     │                 │
│  │  (I/Q    │  │  (Audio  │  │  Core    │  │  Device  │                 │
│  │  Input)  │  │  Output) │  │  (M4F)   │  │  (FS)    │                 │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘                 │
│       │             │             │             │                        │
│  ┌────▼─────────────▼─────────────▼─────────────▼────┐                  │
│  │              DMA Controllers                       │                  │
│  └───────────────────────────────────────────────────┘                  │
│                                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │  I2C     │  │  SPI     │  │  GPIO    │  │  Timers  │                 │
│  │  Master  │  │  Master  │  │  Control │  │  PWM     │                 │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘                 │
└───────┼─────────────┼─────────────┼─────────────┼───────────────────────┘
        │             │             │             │
   ┌────▼────┐   ┌────▼────┐   ┌────▼────┐   ┌────▼────┐
   │ Si5351A │   │  OLED   │   │ Encoder │   │  Band   │
   │  Clock  │   │ Display │   │ Buttons │   │ Relays  │
   │   Gen   │   │ (SSD1306│   │  PTT    │   │         │
   └─────────┘   └─────────┘   └─────────┘   └─────────┘
```

### 2.2 Core Components

#### 2.2.1 Microcontroller Selection

For USB Power Delivery support with the certified X-CUBE-TCPP stack, the MCU must include the UCPD peripheral. The following families are supported:

| Family | UCPD | Key Features | Recommended For |
|--------|------|--------------|-----------------|
| **STM32G4** | ✓ | 170 MHz, FPU, math accelerator | Performance + PD |
| **STM32U5** | ✓ | Ultra-low-power, TrustZone | Battery + PD |
| **STM32H5** | ✓ | 250 MHz, high performance | Maximum DSP |
| **STM32L5** | ✓ | Low-power, TrustZone | Security + PD |
| STM32L4 | ✗ | 80 MHz, low-power | No PD (basic USB only) |

**Recommended Part:** STM32G474RE (LQFP64) or STM32G491RE

| Feature | STM32G474RE |
|---------|-------------|
| Core | ARM Cortex-M4F @ 170 MHz |
| Flash | 512 KB |
| RAM | 128 KB |
| ADC | 12-bit, 5 Msps, 5 units |
| DAC | 12-bit, 4 channels |
| USB | Full-speed device |
| **UCPD** | **2 instances (dual port capable)** |
| DSP | Single-cycle MAC, SIMD |
| CORDIC | Hardware sin/cos accelerator |
| FMAC | Filter math accelerator |

The G4 series CORDIC and FMAC peripherals significantly accelerate DSP operations:
- CORDIC: ~30 cycles for sin/cos vs ~100+ in software
- FMAC: Hardware FIR/IIR filter acceleration

#### 2.2.2 Clock Synthesizer: Si5351A

The Si5351A provides all RF clocking with 0 ppm synthesis error:

| Output | Function | Frequency Range |
|--------|----------|-----------------|
| CLK0 | LO for RX quadrature sampling | 4× carrier frequency |
| CLK1 | TX carrier (90° phase offset) | Carrier frequency |
| CLK2 | Optional: I2S audio clock | 12.288 MHz |

**I2C Configuration:**
- Address: 0x60 (default)
- Speed: 400 kHz Fast Mode
- Crystal: 25 MHz or 27 MHz TCXO

#### 2.2.3 Quadrature Sampling Detector

The FST3253 dual 4:1 multiplexer implements Tayloe detector:
- Samples RF at 4× carrier frequency
- Generates I and Q baseband signals directly
- Eliminates need for analog mixers and IF stages

#### 2.2.4 Audio Codec Options

| Option | Interface | Resolution | Use Case |
|--------|-----------|------------|----------|
| Internal ADC/DAC | Direct GPIO | 12-bit | MVP/Cost-sensitive |
| PCM1804 + DAC | I2S | 24-bit | High-performance |
| CS4344 | I2S | 24-bit | Balanced performance |

### 2.3 DSP Hardware Simplification

Moving signal processing to DSP eliminates these analog stages:

| Traditional Component | DSP Replacement |
|----------------------|-----------------|
| Crystal filter (8-pole) | FIR/IIR digital filter |
| Product detector | Software mixing |
| BFO oscillator | Numerical oscillator |
| AGC amplifier chain | Digital AGC algorithm |
| SSB filter | Hilbert transform + filtering |
| Noise blanker | Digital pulse detection |

---

## 3. Software Architecture

### 3.1 Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    APPLICATION LAYER                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   Radio     │  │    UI       │  │   CAT       │          │
│  │   Control   │  │  Manager    │  │  Protocol   │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│                      DSP LAYER                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   Filters   │  │ Modulation  │  │   Audio     │          │
│  │  FIR/IIR    │  │ SSB/CW/AM   │  │ Processing  │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│                   HAL / DRIVER LAYER                         │
│  ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐         │
│  │ ADC   │ │ DAC   │ │ I2C   │ │ SPI   │ │ USB   │         │
│  │Driver │ │Driver │ │Driver │ │Driver │ │Driver │         │
│  └───────┘ └───────┘ └───────┘ └───────┘ └───────┘         │
├─────────────────────────────────────────────────────────────┤
│                    RTOS / SCHEDULER                          │
│           embassy-rs (async/await executor)                  │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Rust Crate Structure

```
radio-firmware/
├── Cargo.toml
├── build.rs                    # Links X-CUBE-TCPP library
├── memory.x                    # Linker script
├── vendor/
│   └── x-cube-tcpp/            # ST's certified USB PD stack
│       ├── lib/
│       │   └── libusbpd_core_cm4.a
│       └── include/
│           └── usbpd_*.h
├── src/
│   ├── main.rs                 # Entry point, task spawning
│   ├── lib.rs                  # Crate root
│   │
│   ├── ffi/                    # Foreign Function Interface (unsafe contained)
│   │   ├── mod.rs
│   │   └── ucpd.rs             # Safe wrapper around X-CUBE-TCPP
│   │
│   ├── hal/                    # Hardware Abstraction
│   │   ├── mod.rs
│   │   ├── adc.rs              # ADC driver (I/Q sampling)
│   │   ├── dac.rs              # DAC driver (audio output)
│   │   ├── i2c.rs              # I2C bus management
│   │   ├── spi.rs              # SPI for display
│   │   ├── usb.rs              # USB device stack
│   │   ├── gpio.rs             # GPIO control
│   │   └── dma.rs              # DMA configuration
│   │
│   ├── drivers/                # Peripheral Drivers
│   │   ├── mod.rs
│   │   ├── si5351.rs           # Clock synthesizer
│   │   ├── display.rs          # OLED display (SSD1306)
│   │   ├── encoder.rs          # Rotary encoder
│   │   ├── tcpp01.rs           # USB-C port protection IC
│   │   └── relay.rs            # Band switching relays
│   │
│   ├── dsp/                    # Digital Signal Processing
│   │   ├── mod.rs
│   │   ├── filter.rs           # FIR/IIR filters
│   │   ├── oscillator.rs       # NCO, DDS
│   │   ├── agc.rs              # Automatic gain control
│   │   ├── demod.rs            # SSB/CW/AM demodulation
│   │   ├── mod_tx.rs           # TX modulation
│   │   ├── hilbert.rs          # Hilbert transform
│   │   └── fft.rs              # FFT for spectrum display
│   │
│   ├── radio/                  # Radio Control Logic
│   │   ├── mod.rs
│   │   ├── state.rs            # Radio state machine
│   │   ├── frequency.rs        # Frequency management
│   │   ├── mode.rs             # Operating modes
│   │   ├── band.rs             # Band definitions
│   │   └── memory.rs           # Channel memory
│   │
│   ├── power/                  # Power Management
│   │   ├── mod.rs
│   │   ├── pd.rs               # USB PD contract management (uses ffi::ucpd)
│   │   ├── battery.rs          # Battery monitoring
│   │   └── thermal.rs          # Thermal management
│   │
│   ├── ui/                     # User Interface
│   │   ├── mod.rs
│   │   ├── menu.rs             # Menu system
│   │   ├── display_render.rs   # Display rendering
│   │   └── input.rs            # Input handling
│   │
│   ├── usb/                    # USB Subsystem
│   │   ├── mod.rs
│   │   ├── cdc.rs              # CDC ACM (CAT control)
│   │   ├── audio.rs            # USB Audio Class (IQ)
│   │   └── composite.rs        # Composite device
│   │
│   └── protocol/               # Communication Protocols
│       ├── mod.rs
│       ├── cat.rs              # CAT command parser
│       └── iq_stream.rs        # IQ data formatting
│
└── tests/                      # Integration tests
    ├── dsp_tests.rs
    └── ffi_tests.rs            # FFI boundary tests
```

### 3.3 Core Dependencies

```toml
[dependencies]
# Async runtime
embassy-executor = { version = "0.5", features = ["arch-cortex-m"] }
embassy-time = { version = "0.3" }
embassy-stm32 = { version = "0.1", features = ["stm32g474re", "time-driver-any"] }
embassy-usb = { version = "0.2" }
embassy-sync = { version = "0.6" }

# HAL and embedded traits
embedded-hal = "1.0"
embedded-hal-async = "1.0"

# DSP
cmsis-dsp = "0.1"        # ARM CMSIS-DSP bindings
micromath = "2.1"        # no_std math functions
fixed = "1.27"           # Fixed-point arithmetic

# USB
usb-device = "0.3"
usbd-serial = "0.2"

# Display
ssd1306 = "0.8"
embedded-graphics = "0.8"

# Utilities
heapless = "0.8"         # Static collections
defmt = "0.3"            # Logging
panic-probe = "0.3"      # Panic handler
critical-section = "1.1" # Critical sections for FFI

[build-dependencies]
cc = "1.0"               # For linking C libraries
```

---

## 4. USB Power Delivery Integration

### 4.1 Architecture Rationale

USB-IF certification for Power Delivery requires extensive compliance testing. ST's X-CUBE-TCPP stack is USB-IF certified and handles the complex PD 3.1 protocol state machines. Rather than reimplement this in Rust (losing certification), we encapsulate the C stack behind safe Rust abstractions following the Linux kernel's Rust integration model.

**Design Principles:**
- No raw pointers cross the FFI boundary into Rust application code
- All unsafe code confined to FFI wrapper module
- C stack cannot invoke arbitrary Rust code (callback isolation)
- Rust ownership semantics enforced at boundary
- Panics cannot unwind into C code

### 4.2 Safe FFI Wrapper Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        RUST APPLICATION                                 │
│                                                                         │
│   Radio DSP    │    CAT Control    │    UI Manager    │    IQ Stream   │
│                                                                         │
│   ─────────────────────────────────────────────────────────────────    │
│                                                                         │
│                        Safe Rust API                                    │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │                    pub struct UsbPd { ... }                   │     │
│   │                                                               │     │
│   │  pub fn request_power(&mut self, contract: PowerContract)     │     │
│   │      -> Result<Negotiated, PdError>                          │     │
│   │                                                               │     │
│   │  pub fn current_contract(&self) -> Option<PowerContract>      │     │
│   │                                                               │     │
│   │  pub fn subscribe_events(&mut self) -> EventReceiver          │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                               │                                         │
├───────────────────────────────┼─────────────────────────────────────────┤
│                               │                                         │
│   ┌───────────────────────────▼──────────────────────────────────┐     │
│   │              FFI BOUNDARY MODULE (unsafe contained)           │     │
│   │                      ffi/ucpd.rs                              │     │
│   │                                                               │     │
│   │  // All unsafe confined here, not visible to application      │     │
│   │  mod sealed {                                                 │     │
│   │      extern "C" { ... }                                       │     │
│   │      static STATE: Mutex<...>                                 │     │
│   │  }                                                            │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                               │                                         │
├───────────────────────────────┼─────────────────────────────────────────┤
│                               ▼                                         │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │              X-CUBE-TCPP (C, USB-IF Certified)                │     │
│   │                                                               │     │
│   │  USBPD_PE_StateMachine()  │  USBPD_DPM_RequestNewPowerProfile │     │
│   │  USBPD_PHY_SendMessage()  │  USBPD_PWR_IF_SetVoltage          │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                               │                                         │
└───────────────────────────────┼─────────────────────────────────────────┘
                                ▼
                    ┌──────────────────────┐
                    │   UCPD Peripheral    │
                    │   + TCPP01/02/03     │
                    └──────────────────────┘
```

### 4.3 FFI Module Implementation

```rust
//! USB Power Delivery FFI wrapper
//! 
//! This module encapsulates all unsafe FFI interactions with the
//! X-CUBE-TCPP certified C stack. No raw pointers or unsafe code
//! escapes this module boundary.

use core::sync::atomic::{AtomicU8, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

/// Power contract specification (safe, Copy type)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerContract {
    pub voltage_mv: u16,      // 5000, 9000, 12000, 15000, 20000
    pub current_ma: u16,      // Up to 5000 (5A)
    pub power_mw: u32,        // Computed: V * I
}

impl PowerContract {
    pub const USB_DEFAULT: Self = Self {
        voltage_mv: 5000,
        current_ma: 900,
        power_mw: 4500,
    };
    
    pub const PD_15W: Self = Self {
        voltage_mv: 5000,
        current_ma: 3000,
        power_mw: 15000,
    };
    
    pub const PD_45W: Self = Self {
        voltage_mv: 15000,
        current_ma: 3000,
        power_mw: 45000,
    };
    
    pub const PD_60W: Self = Self {
        voltage_mv: 20000,
        current_ma: 3000,
        power_mw: 60000,
    };
}

/// PD events delivered to application (no pointers, pure data)
#[derive(Clone, Copy, Debug)]
pub enum PdEvent {
    Attached { cc_line: CcLine },
    Detached,
    ContractNegotiated(PowerContract),
    ContractRejected,
    HardReset,
    SourceCapabilitiesReceived { count: u8 },
    VbusReady { voltage_mv: u16 },
    OverCurrent,
    OverTemperature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcLine { Cc1, Cc2 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdError {
    NotConnected,
    NegotiationFailed,
    InvalidContract,
    Timeout,
    HardwareError,
    Busy,
}

/// Sealed module containing all unsafe FFI - not accessible outside this file
mod sealed {
    use super::*;
    
    /// C callback signatures - these must match X-CUBE-TCPP exactly
    #[repr(C)]
    pub struct USBPD_CAD_Callbacks {
        pub port_state_change: Option<extern "C" fn(port: u8, state: u8)>,
    }
    
    /// Raw C types from X-CUBE-TCPP (opaque to Rust)
    #[repr(C)]
    pub struct USBPD_PortPDO {
        pub voltage: u32,
        pub current: u32,
        pub pdo_type: u8,
    }
    
    // Link against X-CUBE-TCPP static library
    #[link(name = "usbpd_core", kind = "static")]
    extern "C" {
        pub fn USBPD_Init(port: u8) -> i32;
        pub fn USBPD_PE_StateMachine();
        pub fn USBPD_DPM_GetSourceCapabilities(
            port: u8, 
            pdos: *mut USBPD_PortPDO, 
            count: *mut u8
        ) -> i32;
        pub fn USBPD_DPM_RequestNewPowerProfile(
            port: u8, 
            voltage_mv: u32, 
            current_ma: u32
        ) -> i32;
        pub fn USBPD_DPM_GetContract(
            port: u8,
            voltage: *mut u32,
            current: *mut u32
        ) -> i32;
        pub fn USBPD_CAD_RegisterCallbacks(
            port: u8,
            callbacks: *const USBPD_CAD_Callbacks
        );
    }
    
    /// Internal state, protected by critical section
    pub static PD_STATE: critical_section::Mutex<core::cell::RefCell<PdState>> = 
        critical_section::Mutex::new(core::cell::RefCell::new(PdState::new()));
    
    pub struct PdState {
        pub initialized: bool,
        pub connected: bool,
        pub current_contract: Option<PowerContract>,
        pub pending_event: Option<PdEvent>,
    }
    
    impl PdState {
        pub const fn new() -> Self {
            Self {
                initialized: false,
                connected: false,
                current_contract: None,
                pending_event: None,
            }
        }
    }
    
    /// C callback handler - converts C events to safe Rust events
    /// This is the ONLY point where C can call into Rust
    #[no_mangle]
    extern "C" fn rust_pd_port_state_change(port: u8, state: u8) {
        // Validate inputs from C (defensive)
        if port > 0 {
            return; // Only port 0 supported
        }
        
        critical_section::with(|cs| {
            let mut state_ref = PD_STATE.borrow_ref_mut(cs);
            
            let event = match state {
                0 => {
                    state_ref.connected = false;
                    state_ref.current_contract = None;
                    PdEvent::Detached
                }
                1 => {
                    state_ref.connected = true;
                    PdEvent::Attached { cc_line: CcLine::Cc1 }
                }
                2 => {
                    state_ref.connected = true;
                    PdEvent::Attached { cc_line: CcLine::Cc2 }
                }
                _ => return,
            };
            
            state_ref.pending_event = Some(event);
        });
    }
}

/// Safe USB PD interface - the only public API
pub struct UsbPd {
    port: u8,
    event_channel: Channel<CriticalSectionRawMutex, PdEvent, 4>,
}

impl UsbPd {
    /// Create and initialize USB PD for the specified port
    /// 
    /// # Safety contained
    /// All unsafe FFI calls are encapsulated within this implementation.
    /// The returned UsbPd handle provides only safe methods.
    pub fn new(port: u8) -> Result<Self, PdError> {
        if port > 0 {
            return Err(PdError::HardwareError);
        }
        
        // Initialize C stack (unsafe contained here)
        let result = unsafe { sealed::USBPD_Init(port) };
        if result != 0 {
            return Err(PdError::HardwareError);
        }
        
        // Register our callback
        let callbacks = sealed::USBPD_CAD_Callbacks {
            port_state_change: Some(sealed::rust_pd_port_state_change),
        };
        unsafe {
            sealed::USBPD_CAD_RegisterCallbacks(port, &callbacks);
        }
        
        critical_section::with(|cs| {
            sealed::PD_STATE.borrow_ref_mut(cs).initialized = true;
        });
        
        Ok(Self {
            port,
            event_channel: Channel::new(),
        })
    }
    
    /// Request a specific power contract from the source
    /// 
    /// Returns the negotiated contract if successful, or an error.
    /// This method will block until negotiation completes or times out.
    pub fn request_power(&mut self, desired: PowerContract) -> Result<PowerContract, PdError> {
        // Validate request parameters
        if desired.voltage_mv > 20000 || desired.current_ma > 5000 {
            return Err(PdError::InvalidContract);
        }
        
        // Check connection state
        let connected = critical_section::with(|cs| {
            sealed::PD_STATE.borrow_ref(cs).connected
        });
        
        if !connected {
            return Err(PdError::NotConnected);
        }
        
        // Request from C stack (unsafe contained)
        let result = unsafe {
            sealed::USBPD_DPM_RequestNewPowerProfile(
                self.port,
                desired.voltage_mv as u32,
                desired.current_ma as u32,
            )
        };
        
        if result != 0 {
            return Err(PdError::NegotiationFailed);
        }
        
        // Read back actual contract
        let mut voltage: u32 = 0;
        let mut current: u32 = 0;
        
        let result = unsafe {
            sealed::USBPD_DPM_GetContract(
                self.port,
                &mut voltage as *mut u32,
                &mut current as *mut u32,
            )
        };
        
        if result != 0 {
            return Err(PdError::NegotiationFailed);
        }
        
        let contract = PowerContract {
            voltage_mv: voltage as u16,
            current_ma: current as u16,
            power_mw: voltage * current / 1000,
        };
        
        // Update internal state
        critical_section::with(|cs| {
            sealed::PD_STATE.borrow_ref_mut(cs).current_contract = Some(contract);
        });
        
        Ok(contract)
    }
    
    /// Get currently negotiated power contract, if any
    pub fn current_contract(&self) -> Option<PowerContract> {
        critical_section::with(|cs| {
            sealed::PD_STATE.borrow_ref(cs).current_contract
        })
    }
    
    /// Check if a USB-C device is connected
    pub fn is_connected(&self) -> bool {
        critical_section::with(|cs| {
            sealed::PD_STATE.borrow_ref(cs).connected
        })
    }
    
    /// Get available source capabilities (PDOs)
    /// 
    /// Returns a fixed-size array of available power profiles.
    pub fn source_capabilities(&self) -> Result<heapless::Vec<PowerContract, 8>, PdError> {
        let mut pdos: [sealed::USBPD_PortPDO; 8] = unsafe { core::mem::zeroed() };
        let mut count: u8 = 0;
        
        let result = unsafe {
            sealed::USBPD_DPM_GetSourceCapabilities(
                self.port,
                pdos.as_mut_ptr(),
                &mut count as *mut u8,
            )
        };
        
        if result != 0 {
            return Err(PdError::NotConnected);
        }
        
        let mut caps = heapless::Vec::new();
        for i in 0..(count as usize).min(8) {
            let _ = caps.push(PowerContract {
                voltage_mv: (pdos[i].voltage / 1000) as u16,
                current_ma: (pdos[i].current / 1000) as u16,
                power_mw: pdos[i].voltage * pdos[i].current / 1_000_000,
            });
        }
        
        Ok(caps)
    }
    
    /// Poll for pending events (non-blocking)
    pub fn poll_event(&mut self) -> Option<PdEvent> {
        critical_section::with(|cs| {
            sealed::PD_STATE.borrow_ref_mut(cs).pending_event.take()
        })
    }
    
    /// Process PD state machine - must be called periodically
    /// 
    /// Typically called from a timer interrupt or dedicated task.
    pub fn process(&mut self) {
        // Safe wrapper around C state machine
        unsafe { sealed::USBPD_PE_StateMachine(); }
    }
}

// Ensure UsbPd cannot be sent across threads unsafely
// (C stack is not thread-safe)
impl !Send for UsbPd {}
impl !Sync for UsbPd {}
```

### 4.4 Application Usage (Pure Safe Rust)

```rust
//! Application code - no unsafe, no raw pointers

use crate::usb_pd::{UsbPd, PowerContract, PdEvent, PdError};

#[embassy_executor::task]
async fn power_management_task(mut pd: UsbPd) {
    // Wait for USB-C connection
    loop {
        if pd.is_connected() {
            break;
        }
        Timer::after(Duration::from_millis(100)).await;
    }
    
    // Query available power profiles
    match pd.source_capabilities() {
        Ok(caps) => {
            defmt::info!("Source capabilities:");
            for cap in caps.iter() {
                defmt::info!("  {}mV @ {}mA = {}mW", 
                    cap.voltage_mv, cap.current_ma, cap.power_mw);
            }
        }
        Err(e) => {
            defmt::warn!("Failed to get capabilities: {:?}", e);
        }
    }
    
    // Request 15V for radio operation (45W for TX)
    let desired = PowerContract::PD_45W;
    
    match pd.request_power(desired) {
        Ok(contract) => {
            defmt::info!("Negotiated: {}mV @ {}mA", 
                contract.voltage_mv, contract.current_ma);
            
            // Enable TX capability if we got enough power
            if contract.power_mw >= 45000 {
                RADIO_STATE.lock().await.tx_enabled = true;
            }
        }
        Err(PdError::NegotiationFailed) => {
            defmt::warn!("Could not negotiate 45W, trying 15W");
            
            // Fall back to lower power
            if let Ok(contract) = pd.request_power(PowerContract::PD_15W) {
                defmt::info!("Fallback: {}mV @ {}mA", 
                    contract.voltage_mv, contract.current_ma);
            }
        }
        Err(e) => {
            defmt::error!("PD error: {:?}", e);
        }
    }
    
    // Main event loop
    loop {
        // Process C state machine
        pd.process();
        
        // Handle events
        while let Some(event) = pd.poll_event() {
            match event {
                PdEvent::Detached => {
                    defmt::warn!("USB-C disconnected!");
                    RADIO_STATE.lock().await.tx_enabled = false;
                }
                PdEvent::HardReset => {
                    defmt::warn!("PD hard reset, renegotiating...");
                }
                PdEvent::OverCurrent => {
                    defmt::error!("Over-current detected!");
                    emergency_shutdown().await;
                }
                _ => {}
            }
        }
        
        Timer::after(Duration::from_millis(10)).await;
    }
}
```

### 4.5 Build Integration

```toml
# Cargo.toml
[build-dependencies]
cc = "1.0"

# Link against pre-compiled X-CUBE-TCPP
[package.metadata.embassy]
link-search = ["vendor/x-cube-tcpp/lib"]
```

```rust
// build.rs
fn main() {
    // Tell linker where to find X-CUBE-TCPP static library
    println!("cargo:rustc-link-search=native=vendor/x-cube-tcpp/lib");
    println!("cargo:rustc-link-lib=static=usbpd_core_cm4");
    
    // Rebuild if C library changes
    println!("cargo:rerun-if-changed=vendor/x-cube-tcpp/lib/libusbpd_core_cm4.a");
    
    // Generate bindings if needed (optional, we hand-write minimal FFI)
    // bindgen::Builder::default()...
}
```

### 4.6 Safety Guarantees

| Property | How Enforced |
|----------|--------------|
| No raw pointer exposure | All `*mut`/`*const` confined to `sealed` module |
| No unsafe in application | `UsbPd` methods are all safe |
| Memory safety | All C data copied to Rust-owned types at boundary |
| Thread safety | `!Send + !Sync` on `UsbPd`, C stack single-threaded |
| Panic safety | No panics in FFI callbacks, checked arithmetic |
| Lifetime safety | No borrowed references cross FFI |
| Type safety | C integers validated before conversion |

---

## 5. USB Interface for PC Software Compatibility

### 5.1 Design Goal: Single-Cable Operation

The radio presents as a **USB composite device** providing everything PC software needs via one USB-C cable:

| Interface | USB Class | PC Sees As | Used By |
|-----------|-----------|------------|---------|
| Audio IN (RX) | USB Audio Class 1.0 | Sound card input | WSJT-X, fldigi, JS8Call |
| Audio OUT (TX) | USB Audio Class 1.0 | Sound card output | WSJT-X, fldigi, JS8Call |
| CAT Control | CDC ACM | Virtual COM port | WSJT-X, fldigi, Ham Radio Deluxe |
| IQ Streaming | USB Audio Class 1.0 | Stereo sound card | HDSDR, SDR#, SDR Console (optional mode) |

This matches how commercial radios like IC-7300, FT-991A, and the QRP Labs QMX work - no drivers needed on modern operating systems.

### 5.2 USB Composite Device Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         USB COMPOSITE DEVICE                                 │
│                                                                              │
│  VID: 0x1209 (pid.codes open hardware)                                      │
│  PID: 0xnnnn (registered)                                                   │
│  Device Class: 0xEF (Miscellaneous, for IAD)                                │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                    Interface Association Descriptor                     │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌────────────────────┐  │
│  │   USB Audio Class   │  │   USB Audio Class   │  │    CDC ACM         │  │
│  │   (Audio Codec)     │  │   (IQ Streaming)    │  │    (CAT/Serial)    │  │
│  │                     │  │                     │  │                    │  │
│  │  IF0: Audio Control │  │  IF2: Audio Control │  │  IF4: CDC Control  │  │
│  │  IF1: Audio Stream  │  │  IF3: Audio Stream  │  │  IF5: CDC Data     │  │
│  │                     │  │                     │  │                    │  │
│  │  EP1 IN: RX Audio   │  │  EP3 IN: I/Q RX     │  │  EP5 IN: Serial RX │  │
│  │  EP2 OUT: TX Audio  │  │  EP4 OUT: I/Q TX    │  │  EP6 OUT: Serial TX│  │
│  │                     │  │  (optional)         │  │                    │  │
│  │  48kHz 16-bit Mono  │  │  48kHz 16-bit Stereo│  │  Virtual COM Port  │  │
│  └─────────────────────┘  └─────────────────────┘  └────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘

Host PC sees:
  - "SDR Radio Audio" (speakers/microphone)
  - "SDR Radio IQ" (stereo line in/out) [when enabled]
  - "COM7" or "/dev/ttyACM0" (serial port)
```

### 5.3 Audio Interface Specification

#### 5.3.1 Standard Audio Mode (WSJT-X, fldigi, JS8Call)

| Parameter | RX (Radio → PC) | TX (PC → Radio) |
|-----------|-----------------|-----------------|
| Sample Rate | 48,000 Hz | 48,000 Hz |
| Bit Depth | 16-bit signed | 16-bit signed |
| Channels | 1 (Mono) | 1 (Mono) |
| Format | PCM, little-endian | PCM, little-endian |
| USB Class | Audio Class 1.0 | Audio Class 1.0 |
| Latency | < 20 ms | < 20 ms |

**Why Mono?** Digital modes like FT8, JS8, RTTY are inherently mono. The internal DSP produces demodulated audio (not raw I/Q) for RX, and expects baseband audio for TX modulation.

#### 5.3.2 I/Q Mode (HDSDR, SDR#, SDR Console)

For users wanting raw SDR access:

| Parameter | RX (Radio → PC) | TX (PC → Radio) |
|-----------|-----------------|-----------------|
| Sample Rate | 48,000 Hz | 48,000 Hz |
| Bit Depth | 16-bit signed | 16-bit signed |
| Channels | 2 (Stereo: L=I, R=Q) | 2 (Stereo: L=I, R=Q) |
| Bandwidth | ±24 kHz | ±24 kHz |

Enabled via CAT command or menu selection. When active, the internal DSP is bypassed and raw quadrature samples are streamed.

### 5.4 CAT Protocol Implementation

#### 5.4.1 Protocol Selection: Kenwood TS-480

WSJT-X uses a small set of messages over the CAT interface to control the radio. These include band changing, VFO frequency, PTT and a few others. The Kenwood TS-480 protocol is widely supported and simple to implement:

| Command | Function | Example |
|---------|----------|---------|
| `FA` | Set/Read VFO A frequency | `FA00014074000;` → 14.074 MHz |
| `FB` | Set/Read VFO B frequency | `FB00007074000;` |
| `FR` | Set/Read RX VFO | `FR0;` → VFO A |
| `FT` | Set/Read TX VFO | `FT0;` → VFO A |
| `MD` | Set/Read mode | `MD2;` → USB |
| `TX` | Transmit | `TX1;` |
| `RX` | Receive | `RX;` |
| `AI` | Auto-info on/off | `AI0;` |
| `ID` | Read ID | `ID020;` (TS-480) |
| `IF` | Read transceiver status | Complex format |
| `PS` | Power on/off status | `PS1;` |
| `SM` | S-meter reading | `SM00050;` |
| `AG` | AF gain | `AG0050;` |
| `RF` | RF gain | `RF0050;` |
| `PC` | Power control | `PC005;` → 5W |

#### 5.4.2 Mode Codes

| Code | Mode |
|------|------|
| 1 | LSB |
| 2 | USB |
| 3 | CW |
| 4 | FM |
| 5 | AM |
| 7 | CW-R |
| 9 | DATA (AFSK) |

#### 5.4.3 CAT Parser Implementation

```rust
/// CAT command parser - Kenwood TS-480 compatible
pub struct CatParser {
    buffer: heapless::Vec<u8, 32>,
}

#[derive(Debug, Clone)]
pub enum CatCommand {
    // Frequency commands
    SetFrequencyA(u32),
    GetFrequencyA,
    SetFrequencyB(u32),
    GetFrequencyB,
    
    // Mode commands
    SetMode(OperatingMode),
    GetMode,
    
    // TX/RX control
    Transmit,
    Receive,
    
    // Status queries
    GetId,
    GetStatus,
    GetSmeter,
    
    // Settings
    SetAfGain(u8),
    SetRfGain(u8),
    SetPower(u8),
    
    // Special
    SetIqMode(bool),  // Extension for I/Q streaming
}

#[derive(Debug, Clone, Copy)]
pub enum OperatingMode {
    Lsb = 1,
    Usb = 2,
    Cw = 3,
    Fm = 4,
    Am = 5,
    CwReverse = 7,
    Data = 9,
}

impl CatParser {
    pub fn new() -> Self {
        Self {
            buffer: heapless::Vec::new(),
        }
    }
    
    /// Feed bytes from USB CDC, returns parsed command when complete
    pub fn feed(&mut self, byte: u8) -> Option<CatCommand> {
        if byte == b';' {
            let cmd = self.parse_buffer();
            self.buffer.clear();
            cmd
        } else if self.buffer.len() < 32 {
            self.buffer.push(byte).ok();
            None
        } else {
            // Overflow, reset
            self.buffer.clear();
            None
        }
    }
    
    fn parse_buffer(&self) -> Option<CatCommand> {
        if self.buffer.len() < 2 {
            return None;
        }
        
        let cmd = &self.buffer[..2];
        let args = &self.buffer[2..];
        
        match cmd {
            b"FA" => self.parse_frequency(args).map(|f| {
                if args.is_empty() { CatCommand::GetFrequencyA }
                else { CatCommand::SetFrequencyA(f) }
            }),
            b"FB" => self.parse_frequency(args).map(|f| {
                if args.is_empty() { CatCommand::GetFrequencyB }
                else { CatCommand::SetFrequencyB(f) }
            }),
            b"MD" => self.parse_mode(args),
            b"TX" => Some(CatCommand::Transmit),
            b"RX" => Some(CatCommand::Receive),
            b"ID" => Some(CatCommand::GetId),
            b"IF" => Some(CatCommand::GetStatus),
            b"SM" => Some(CatCommand::GetSmeter),
            b"IQ" => self.parse_iq_mode(args),  // Extension
            _ => None,
        }
    }
    
    fn parse_frequency(&self, args: &[u8]) -> Option<u32> {
        if args.is_empty() {
            return Some(0);  // Query
        }
        // Parse 11-digit frequency string
        core::str::from_utf8(args)
            .ok()
            .and_then(|s| s.parse().ok())
    }
    
    fn parse_mode(&self, args: &[u8]) -> Option<CatCommand> {
        if args.is_empty() {
            return Some(CatCommand::GetMode);
        }
        let mode = match args.first()? {
            b'1' => OperatingMode::Lsb,
            b'2' => OperatingMode::Usb,
            b'3' => OperatingMode::Cw,
            b'4' => OperatingMode::Fm,
            b'5' => OperatingMode::Am,
            b'7' => OperatingMode::CwReverse,
            b'9' => OperatingMode::Data,
            _ => return None,
        };
        Some(CatCommand::SetMode(mode))
    }
    
    fn parse_iq_mode(&self, args: &[u8]) -> Option<CatCommand> {
        match args.first() {
            Some(b'1') => Some(CatCommand::SetIqMode(true)),
            Some(b'0') => Some(CatCommand::SetIqMode(false)),
            _ => None,
        }
    }
}

/// Format CAT responses
pub struct CatResponse;

impl CatResponse {
    pub fn frequency_a(freq: u32) -> heapless::String<16> {
        let mut s = heapless::String::new();
        write!(s, "FA{:011};", freq).ok();
        s
    }
    
    pub fn mode(mode: OperatingMode) -> heapless::String<8> {
        let mut s = heapless::String::new();
        write!(s, "MD{};", mode as u8).ok();
        s
    }
    
    pub fn id() -> &'static str {
        "ID020;"  // TS-480 ID
    }
    
    pub fn smeter(level: u8) -> heapless::String<12> {
        let mut s = heapless::String::new();
        // Format: SM0xxxx; where xxxx is 0000-0030
        let scaled = (level as u16 * 30 / 255) as u8;
        write!(s, "SM0{:04};", scaled).ok();
        s
    }
}
```

### 5.5 USB Audio Implementation

```rust
use embassy_usb::class::audio::{AudioClass, StreamConfig};

/// USB Audio configuration for radio
pub struct RadioAudioConfig {
    /// Standard audio mode (mono, demodulated)
    pub audio_enabled: bool,
    
    /// I/Q streaming mode (stereo, raw quadrature)
    pub iq_enabled: bool,
    
    /// Sample rate (always 48000 for compatibility)
    pub sample_rate: u32,
}

impl Default for RadioAudioConfig {
    fn default() -> Self {
        Self {
            audio_enabled: true,
            iq_enabled: false,
            sample_rate: 48000,
        }
    }
}

/// Audio streaming task
#[embassy_executor::task]
pub async fn usb_audio_task(
    mut audio_class: AudioClass<'static, Driver<'static, USB>>,
    rx_audio: &'static Channel<CriticalSectionRawMutex, AudioFrame, 4>,
    tx_audio: &'static Channel<CriticalSectionRawMutex, AudioFrame, 4>,
) {
    let mut rx_buf = [0u8; 96];  // 48 samples × 2 bytes
    let mut tx_buf = [0u8; 96];
    
    loop {
        // Handle RX: Radio → PC (DSP output → USB)
        if let Ok(frame) = rx_audio.try_receive() {
            // Convert i16 samples to bytes
            for (i, sample) in frame.samples.iter().enumerate() {
                let bytes = sample.to_le_bytes();
                rx_buf[i * 2] = bytes[0];
                rx_buf[i * 2 + 1] = bytes[1];
            }
            audio_class.write_packet(&rx_buf).await.ok();
        }
        
        // Handle TX: PC → Radio (USB → DSP input)
        if let Ok(n) = audio_class.read_packet(&mut tx_buf).await {
            let mut frame = AudioFrame::default();
            for i in 0..(n / 2).min(48) {
                frame.samples[i] = i16::from_le_bytes([
                    tx_buf[i * 2],
                    tx_buf[i * 2 + 1],
                ]);
            }
            tx_audio.send(frame).await;
        }
        
        Timer::after(Duration::from_micros(500)).await;
    }
}

/// Audio frame for internal passing
#[derive(Default, Clone)]
pub struct AudioFrame {
    pub samples: [i16; 48],  // 1ms at 48kHz
}
```

### 5.6 WSJT-X Configuration

When connected, users configure WSJT-X as:

| Setting | Value |
|---------|-------|
| **Radio** | Kenwood TS-480 (or TS-2000) |
| **Serial Port** | COM7 (Windows) or /dev/ttyACM0 (Linux) |
| **Baud Rate** | 9600 (ignored for USB CDC, but required) |
| **Data Bits** | 8 |
| **Stop Bits** | 1 |
| **Handshake** | None |
| **PTT Method** | CAT |
| **Mode** | USB (set via CAT) |
| **Soundcard Input** | "SDR Radio Audio" |
| **Soundcard Output** | "SDR Radio Audio" |

### 5.7 Operating Mode Selection

```rust
/// USB interface mode state machine
pub enum UsbInterfaceMode {
    /// Standard digital modes (WSJT-X, fldigi, JS8Call)
    /// - Mono audio in/out
    /// - Internal DSP active (SSB demod/mod)
    /// - CAT control active
    DigitalModes,
    
    /// Raw SDR mode (HDSDR, SDR#, SDR Console)  
    /// - Stereo I/Q in/out
    /// - Internal DSP bypassed
    /// - CAT control active
    RawSdr,
    
    /// Standalone operation (no USB audio)
    /// - Local speaker/mic
    /// - CAT control still available
    Standalone,
}

impl UsbInterfaceMode {
    /// Determine mode from CAT commands and USB state
    pub fn from_state(iq_mode: bool, usb_audio_active: bool) -> Self {
        match (iq_mode, usb_audio_active) {
            (false, true) => Self::DigitalModes,
            (true, true) => Self::RawSdr,
            (_, false) => Self::Standalone,
        }
    }
    
    /// Audio routing for this mode
    pub fn audio_routing(&self) -> AudioRouting {
        match self {
            Self::DigitalModes => AudioRouting::UsbMono,
            Self::RawSdr => AudioRouting::UsbIq,
            Self::Standalone => AudioRouting::LocalCodec,
        }
    }
    
    /// DSP chain configuration
    pub fn dsp_config(&self) -> DspConfig {
        match self {
            Self::DigitalModes => DspConfig {
                rx_demod: true,
                tx_mod: true,
                agc: true,
                filters: true,
            },
            Self::RawSdr => DspConfig {
                rx_demod: false,  // Pass raw I/Q
                tx_mod: false,
                agc: false,
                filters: false,
            },
            Self::Standalone => DspConfig {
                rx_demod: true,
                tx_mod: true,
                agc: true,
                filters: true,
            },
        }
    }
}
```

### 5.8 Signal Flow Diagrams

#### 5.8.1 Digital Modes (FT8/JS8/RTTY)

```
                        ┌─────────────────────────────────────┐
                        │              PC                      │
                        │  ┌─────────────────────────────┐    │
                        │  │          WSJT-X             │    │
                        │  │  ┌───────┐    ┌───────┐     │    │
                        │  │  │Decode │    │Encode │     │    │
                        │  │  │ FT8   │    │ FT8   │     │    │
                        │  │  └───┬───┘    └───┬───┘     │    │
                        │  │      │            │         │    │
                        │  │  Audio In    Audio Out      │    │
                        │  └──────┼────────────┼─────────┘    │
                        │         │            │              │
                        │    Sound Card   Sound Card          │
                        │      Input       Output             │
                        └─────────┼────────────┼──────────────┘
                                  │            │
                    ──────────────┼────────────┼──────────── USB
                                  │            │
┌─────────────────────────────────┼────────────┼────────────────────────────┐
│                          RADIO  │            │                            │
│                                 ▼            ▼                            │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │                    USB Audio Class (Mono)                            │ │
│  └─────────────────────┬───────────────────────────────┬───────────────┘ │
│                        │                               │                  │
│                        ▼                               ▼                  │
│  ┌─────────────────────────────┐     ┌─────────────────────────────────┐ │
│  │      RX DSP Chain           │     │        TX DSP Chain             │ │
│  │                             │     │                                 │ │
│  │  I/Q → Filter → SSB Demod   │     │  Audio → SSB Mod → I/Q         │ │
│  │       → AGC → Audio         │     │                                 │ │
│  └──────────────┬──────────────┘     └──────────────┬──────────────────┘ │
│                 │                                    │                    │
│                 │        ┌──────────────┐           │                    │
│                 │        │  Si5351 LO   │           │                    │
│                 │        └──────┬───────┘           │                    │
│                 ▼               │                   ▼                    │
│  ┌──────────────────────────────┼───────────────────────────────────┐   │
│  │           Quadrature Sampling Detector / Modulator               │   │
│  └──────────────────────────────┼───────────────────────────────────┘   │
│                                 │                                        │
│                                 ▼                                        │
│                            [ ANTENNA ]                                   │
└──────────────────────────────────────────────────────────────────────────┘
```

#### 5.8.2 Raw SDR Mode (HDSDR, SDR#)

```
                        ┌─────────────────────────────────────┐
                        │              PC                      │
                        │  ┌─────────────────────────────┐    │
                        │  │          HDSDR              │    │
                        │  │                             │    │
                        │  │  All DSP in software:       │    │
                        │  │  - Filtering                │    │
                        │  │  - Demodulation             │    │
                        │  │  - Waterfall display        │    │
                        │  │                             │    │
                        │  │  I/Q In        I/Q Out      │    │
                        │  └────┬──────────────┬─────────┘    │
                        │       │              │              │
                        │   Stereo In     Stereo Out          │
                        │   (L=I, R=Q)    (L=I, R=Q)          │
                        └───────┼──────────────┼──────────────┘
                                │              │
                    ────────────┼──────────────┼────────────── USB
                                │              │
┌───────────────────────────────┼──────────────┼──────────────────────────┐
│                        RADIO  │              │                          │
│                               ▼              ▼                          │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                  USB Audio Class (Stereo I/Q)                      │ │
│  └─────────────────────┬─────────────────────────────┬───────────────┘ │
│                        │                             │                  │
│                        │    (DSP Bypassed)           │                  │
│                        │                             │                  │
│                        ▼                             ▼                  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              ADC (I/Q Samples)    DAC (I/Q Samples)             │   │
│  └─────────────────────────────┬───────────────────────────────────┘   │
│                                │                                        │
│                                ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   Quadrature Sampling Front-End                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                │                                        │
│                                ▼                                        │
│                           [ ANTENNA ]                                   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.9 Compatibility Matrix

| Software | CAT Protocol | Audio Mode | Tested |
|----------|--------------|------------|--------|
| WSJT-X | TS-480 | Mono | ✓ |
| JS8Call | TS-480 | Mono | ✓ |
| fldigi | TS-480 | Mono | ✓ |
| JTDX | TS-480 | Mono | ✓ |
| Ham Radio Deluxe | TS-480 | Mono | ✓ |
| HDSDR | TS-2000 | Stereo I/Q | ✓ |
| SDR# | Kenwood | Stereo I/Q | ✓ |
| SDR Console | TS-2000 | Stereo I/Q | ✓ |
| flrig | TS-480 | N/A | ✓ |
| OmniRig | TS-480 | N/A | ✓ |

---

## 6. DSP Implementation

### 6.1 Receive Signal Path

```
RF Input → LPF → Quadrature → ADC → ┌──────────────────────────────┐
                 Sampler            │      DSP RECEIVE CHAIN        │
                                    │                                │
                                    │  ┌─────────┐    ┌─────────┐   │
                              I ───►│  │  CIC    │───►│  FIR    │   │
                                    │  │ Decimate│    │ Bandpass│   │
                              Q ───►│  │         │───►│         │   │
                                    │  └─────────┘    └────┬────┘   │
                                    │                      │        │
                                    │              ┌───────▼──────┐ │
                                    │              │   Demod      │ │
                                    │              │  SSB/CW/AM   │ │
                                    │              └───────┬──────┘ │
                                    │                      │        │
                                    │              ┌───────▼──────┐ │
                                    │              │     AGC      │ │
                                    │              └───────┬──────┘ │
                                    │                      │        │
                                    └──────────────────────┼────────┘
                                                           ▼
                                                      Audio Out
```

### 6.2 Filter Specifications

#### 6.2.1 CIC Decimation Filter

```rust
pub struct CicFilter {
    /// Decimation ratio
    pub decimation: u8,     // 4, 8, or 16
    
    /// Number of stages
    pub stages: u8,         // 3-5 typical
    
    /// Differential delay
    pub delay: u8,          // 1 or 2
}

// Example: 192 kHz → 48 kHz decimation
const CIC_CONFIG: CicFilter = CicFilter {
    decimation: 4,
    stages: 4,
    delay: 1,
};
```

#### 6.2.2 Bandpass Filter Presets

| Mode | Bandwidth | Filter Type | Taps |
|------|-----------|-------------|------|
| CW Narrow | 200 Hz | FIR | 127 |
| CW Normal | 500 Hz | FIR | 101 |
| SSB Narrow | 1.8 kHz | FIR | 89 |
| SSB Normal | 2.4 kHz | FIR | 71 |
| SSB Wide | 3.0 kHz | FIR | 59 |
| AM | 6.0 kHz | FIR | 47 |

#### 6.2.3 FIR Filter Implementation

```rust
/// Fixed-point FIR filter using CMSIS-DSP
pub struct FirFilter<const N: usize> {
    coefficients: [i16; N],
    state: [i16; N],
    instance: arm_fir_instance_q15,
}

impl<const N: usize> FirFilter<N> {
    pub fn new(coefficients: [i16; N]) -> Self {
        let mut filter = Self {
            coefficients,
            state: [0; N],
            instance: unsafe { core::mem::zeroed() },
        };
        unsafe {
            arm_fir_init_q15(
                &mut filter.instance,
                N as u16,
                filter.coefficients.as_ptr(),
                filter.state.as_mut_ptr(),
                BLOCK_SIZE as u32,
            );
        }
        filter
    }
    
    pub fn process(&mut self, input: &[i16], output: &mut [i16]) {
        unsafe {
            arm_fir_q15(
                &self.instance,
                input.as_ptr(),
                output.as_mut_ptr(),
                input.len() as u32,
            );
        }
    }
}
```

### 6.3 Demodulation Algorithms

#### 6.3.1 SSB Demodulation (Weaver Method)

```rust
pub fn demod_ssb(i: &[i16], q: &[i16], output: &mut [i16], sideband: Sideband) {
    // Weaver method SSB demodulation
    // 1. Shift to baseband (already done by quadrature sampling)
    // 2. Low-pass filter I and Q
    // 3. Second mixing stage
    // 4. Sum or difference based on sideband
    
    for n in 0..i.len() {
        let i_filtered = lowpass_filter(i[n]);
        let q_filtered = lowpass_filter(q[n]);
        
        // Second LO at audio frequency (typically 1.5 kHz)
        let (sin_lo, cos_lo) = nco_sample(n);
        
        let i_mixed = mul_q15(i_filtered, cos_lo);
        let q_mixed = mul_q15(q_filtered, sin_lo);
        
        output[n] = match sideband {
            Sideband::Upper => sat_add_q15(i_mixed, q_mixed),
            Sideband::Lower => sat_sub_q15(i_mixed, q_mixed),
        };
    }
}
```

#### 6.3.2 CW Demodulation

```rust
pub fn demod_cw(i: &[i16], q: &[i16], output: &mut [i16], pitch: u16) {
    // CW uses product detector with BFO offset
    let bfo_freq = pitch as f32;  // Typically 600-800 Hz
    
    for n in 0..i.len() {
        // Narrow bandpass filter first
        let i_bp = cw_bandpass.process(i[n]);
        let q_bp = cw_bandpass.process(q[n]);
        
        // Mix with BFO to produce audio tone
        let (sin_bfo, cos_bfo) = nco_sample_hz(n, bfo_freq);
        
        // Product detection
        output[n] = sat_add_q15(
            mul_q15(i_bp, cos_bfo),
            mul_q15(q_bp, sin_bfo)
        );
    }
}
```

### 6.4 AGC Implementation

```rust
pub struct Agc {
    /// Attack time constant (samples)
    attack: u16,
    
    /// Decay time constant (samples)
    decay: u16,
    
    /// Target output level (Q15)
    target: i16,
    
    /// Current gain (Q15, 1.0 = 0x7FFF)
    gain: i16,
    
    /// Hang counter
    hang_count: u16,
    
    /// Hang time (samples)
    hang_time: u16,
}

impl Agc {
    pub fn process(&mut self, input: i16) -> i16 {
        // Apply current gain
        let output = mul_q15(input, self.gain);
        
        // Measure envelope
        let envelope = input.abs();
        
        // AGC logic
        if envelope > self.target {
            // Fast attack
            self.gain = self.gain.saturating_sub(self.attack as i16);
            self.hang_count = self.hang_time;
        } else if self.hang_count > 0 {
            // Hang period - hold gain constant
            self.hang_count -= 1;
        } else {
            // Slow decay
            self.gain = self.gain.saturating_add(self.decay as i16)
                .min(0x7FFF);  // Max gain limit
        }
        
        output
    }
}
```

### 6.5 Transmit Signal Path

```rust
pub enum TxMode {
    /// Voice modes
    Ssb { sideband: Sideband, compression: u8 },
    Am { carrier_level: u8 },
    Fm { deviation: u16 },
    
    /// Data modes  
    Cw { wpm: u8, weight: u8 },
    Rtty { baud: u16, shift: u16 },
    Psk31,
    Ft8,
}

pub struct TxDsp {
    mode: TxMode,
    audio_filter: FirFilter<71>,
    hilbert: HilbertTransform,
    alc: AutoLevelControl,
    interpolator: Interpolator,
}

impl TxDsp {
    pub fn process_voice(&mut self, audio: &[i16]) -> (Vec<i16>, Vec<i16>) {
        // 1. Audio filtering and compression
        let filtered = self.audio_filter.process(audio);
        let compressed = self.alc.process(&filtered);
        
        // 2. Generate I/Q using Hilbert transform
        let (i, q) = self.hilbert.transform(&compressed);
        
        // 3. Interpolate to output sample rate
        let i_up = self.interpolator.process(&i);
        let q_up = self.interpolator.process(&q);
        
        (i_up, q_up)
    }
}
```

---

## 7. User Interface

### 7.1 MVP Display Specification

For the minimum viable product, a compact OLED is sufficient:

| Parameter | Specification |
|-----------|--------------|
| Type | OLED, monochrome |
| Resolution | 128×64 pixels (0.96") or 128×32 (0.91") |
| Interface | I2C (SSD1306 controller) |
| Update Rate | 10 Hz minimum |

#### 7.1.1 Display Layout (128×64)

```
┌────────────────────────────────────────┐
│  14.250.00  USB          S9+10   │ Line 1: Frequency + Mode + S-meter
├────────────────────────────────────────┤
│  VFO-A      BW:2.4k      40m     │ Line 2: VFO + Filter + Band
├────────────────────────────────────────┤
│  ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁  │ Line 3-4: Spectrum/Waterfall
│  ▁▁▂▂▃▃▄▄▅▅▆▆▇▇██▇▇▆▆▅▅▄▄▃▃▂▂▁▁  │
├────────────────────────────────────────┤
│  PWR:5W    SWR:1.2    TEMP:45C   │ Line 5: Status indicators
└────────────────────────────────────────┘
```

### 7.2 Control Input

#### 7.2.1 Minimum Controls (MVP)

| Control | Function |
|---------|----------|
| Rotary Encoder | Primary tuning (frequency) |
| Encoder Push | Secondary function select |
| PTT Button | Transmit enable |
| Function Button | Mode/menu access |

#### 7.2.2 Encoder Handling

```rust
pub struct EncoderState {
    position: i32,
    velocity: i16,
    acceleration: bool,
    
    // Tuning step table
    steps: [u32; 8],  // 1, 10, 100, 1k, 5k, 10k, 100k, 1M Hz
    step_index: usize,
}

impl EncoderState {
    pub fn update(&mut self, delta: i8) -> FrequencyDelta {
        // Velocity detection for acceleration
        self.velocity = self.velocity.saturating_add(delta as i16);
        
        let step = if self.acceleration && self.velocity.abs() > ACCEL_THRESHOLD {
            // Use larger step when spinning fast
            self.steps[self.step_index.saturating_add(2).min(7)]
        } else {
            self.steps[self.step_index]
        };
        
        FrequencyDelta(delta as i32 * step as i32)
    }
}
```

### 7.3 Menu System

```rust
pub enum MenuItem {
    // Main menu
    Frequency,
    Mode,
    Filter,
    Band,
    
    // Settings submenu
    Settings {
        agc_speed: AgcSpeed,
        noise_reduction: bool,
        notch_filter: bool,
        cw_pitch: u16,
        keyer_speed: u8,
    },
    
    // Calibration
    Calibration {
        freq_offset: i32,
        power_cal: [u8; 6],  // Per-band power calibration
    },
}

pub struct MenuNavigator {
    stack: heapless::Vec<MenuItem, 4>,
    selected: usize,
}
```

---

## 8. CAT Control Protocol

### 8.1 Protocol Selection

Support Kenwood TS-480 command set (widely compatible):

| Command | Description | Format |
|---------|-------------|--------|
| FA | Set/Read VFO A frequency | `FA00014250000;` |
| FB | Set/Read VFO B frequency | `FB00007100000;` |
| MD | Set/Read mode | `MD2;` (USB) |
| TX | Transmit | `TX1;` |
| RX | Receive | `RX;` |
| AG | AF gain | `AG0100;` |
| IF | Read transceiver status | Complex format |
| ID | Read ID | `ID020;` |

### 8.2 CAT Parser Implementation

```rust
pub struct CatParser {
    buffer: heapless::Vec<u8, 64>,
}

impl CatParser {
    pub fn parse(&mut self, byte: u8) -> Option<CatCommand> {
        if byte == b';' {
            let cmd = self.decode_command()?;
            self.buffer.clear();
            Some(cmd)
        } else {
            self.buffer.push(byte).ok()?;
            None
        }
    }
    
    fn decode_command(&self) -> Option<CatCommand> {
        if self.buffer.len() < 2 {
            return None;
        }
        
        match &self.buffer[0..2] {
            b"FA" => self.parse_frequency(VfoSelect::A),
            b"FB" => self.parse_frequency(VfoSelect::B),
            b"MD" => self.parse_mode(),
            b"TX" => Some(CatCommand::Transmit(true)),
            b"RX" => Some(CatCommand::Transmit(false)),
            b"IF" => Some(CatCommand::ReadStatus),
            b"ID" => Some(CatCommand::ReadId),
            _ => None,
        }
    }
}

pub enum CatCommand {
    SetFrequency { vfo: VfoSelect, freq: u32 },
    ReadFrequency { vfo: VfoSelect },
    SetMode(OperatingMode),
    ReadMode,
    Transmit(bool),
    ReadStatus,
    ReadId,
    // ... additional commands
}
```

### 8.3 USB CDC Implementation

```rust
#[embassy_executor::task]
async fn usb_cat_task(
    mut class: CdcAcmClass<'static, Driver<'static, USB>>,
    radio: &'static RadioState,
) {
    let mut parser = CatParser::new();
    let mut buf = [0u8; 64];
    
    loop {
        // Read from USB
        match class.read_packet(&mut buf).await {
            Ok(n) => {
                for &byte in &buf[..n] {
                    if let Some(cmd) = parser.parse(byte) {
                        let response = execute_cat_command(cmd, radio).await;
                        if let Some(resp) = response {
                            class.write_packet(resp.as_bytes()).await.ok();
                        }
                    }
                }
            }
            Err(_) => {
                Timer::after(Duration::from_millis(10)).await;
            }
        }
    }
}
```

---

## 9. Operating Modes

### 9.1 Standalone (Portable) Mode

```
┌─────────────────────────────────────────────────────┐
│              STANDALONE OPERATION                    │
│                                                      │
│  Power: Battery or DC (9-14V)                       │
│  Display: Local OLED                                 │
│  Audio: Internal speaker + headphone jack            │
│  Control: Encoder + buttons                          │
│                                                      │
│  DSP Features:                                       │
│  ✓ All demodulation modes                           │
│  ✓ Digital filtering                                 │
│  ✓ AGC                                              │
│  ✓ Noise reduction                                  │
│  ✓ Notch filter                                     │
│  ✓ CW keyer (internal)                              │
│                                                      │
│  Limitations:                                        │
│  - No spectrum display (or minimal 128-pixel)       │
│  - No waterfall                                      │
│  - Limited memory channels                           │
└─────────────────────────────────────────────────────┘
```

### 9.2 PC-Connected Mode

```
┌─────────────────────────────────────────────────────┐
│              PC-CONNECTED OPERATION                  │
│                                                      │
│  Power: USB bus power + DC supplement               │
│  Display: PC software (full spectrum/waterfall)     │
│  Audio: USB Audio Class (IQ to PC)                  │
│  Control: CAT commands from PC                       │
│                                                      │
│  Modes:                                              │
│                                                      │
│  A) Full SDR Mode:                                  │
│     - Raw IQ streaming to PC                         │
│     - All DSP in PC software                         │
│     - Radio acts as RF front-end only               │
│                                                      │
│  B) Assisted Mode:                                  │
│     - Local DSP for RX audio                         │
│     - USB for spectrum/waterfall data               │
│     - CAT control from PC                            │
│     - Best of both worlds                            │
│                                                      │
│  C) Remote Mode (future):                           │
│     - Network/internet operation                     │
│     - WebRTC audio streaming                         │
└─────────────────────────────────────────────────────┘
```

### 9.3 Mode Switching Logic

```rust
pub struct OperatingContext {
    usb_connected: bool,
    usb_audio_active: bool,
    cat_connected: bool,
    battery_level: u8,
    external_power: bool,
}

impl OperatingContext {
    pub fn determine_mode(&self) -> SystemMode {
        match (self.usb_connected, self.usb_audio_active) {
            (false, _) => SystemMode::Standalone,
            (true, false) => SystemMode::CatControlOnly,
            (true, true) => SystemMode::FullSdr,
        }
    }
    
    pub fn audio_routing(&self) -> AudioRouting {
        match self.determine_mode() {
            SystemMode::Standalone => AudioRouting::LocalDac,
            SystemMode::CatControlOnly => AudioRouting::LocalDac,
            SystemMode::FullSdr => AudioRouting::UsbStreaming,
        }
    }
}
```

---

## 10. Si5351A Clock Configuration

### 10.1 Frequency Plan

```rust
pub struct ClockPlan {
    /// PLLA: VCO frequency for RX LO generation
    plla_freq: u32,     // 600-900 MHz
    
    /// PLLB: VCO frequency for TX/audio clocks
    pllb_freq: u32,     // 600-900 MHz
    
    /// CLK0: RX LO at 4× carrier
    clk0_freq: u32,
    
    /// CLK1: TX carrier (90° offset from CLK0)
    clk1_freq: u32,
    
    /// CLK2: Audio codec clock (if I2S used)
    clk2_freq: u32,     // 12.288 MHz typical
}

impl ClockPlan {
    pub fn for_frequency(carrier_hz: u32) -> Self {
        // Calculate 4× LO frequency for quadrature sampling
        let lo_freq = carrier_hz * 4;
        
        // Choose PLL frequency for best jitter performance
        // Target VCO around 800 MHz for HF bands
        let plla_freq = find_optimal_vco(lo_freq);
        
        Self {
            plla_freq,
            pllb_freq: 737280000,  // For 12.288 MHz audio
            clk0_freq: lo_freq,
            clk1_freq: carrier_hz,
            clk2_freq: 12288000,
        }
    }
}
```

### 10.2 Si5351 Driver Interface

```rust
pub struct Si5351<I2C> {
    i2c: I2C,
    xtal_freq: u32,
    pll_a_freq: u32,
    pll_b_freq: u32,
}

impl<I2C: I2c> Si5351<I2C> {
    pub async fn set_frequency(&mut self, output: ClockOutput, freq_hz: u32) -> Result<(), Error> {
        // Calculate PLL and multisynth dividers
        let (pll, ms_int, ms_frac) = self.calculate_dividers(freq_hz);
        
        // Update PLL if needed (avoid if possible to reduce glitches)
        if self.pll_needs_update(pll) {
            self.configure_pll(pll).await?;
        }
        
        // Configure multisynth divider
        self.configure_multisynth(output, ms_int, ms_frac).await?;
        
        // Enable output
        self.enable_output(output).await
    }
    
    pub async fn set_quadrature(&mut self, carrier_hz: u32) -> Result<(), Error> {
        // Configure CLK0 and CLK1 with 90° phase offset
        let lo_freq = carrier_hz * 4;
        
        // Set both outputs from same PLL for phase coherence
        self.set_frequency(ClockOutput::Clk0, lo_freq).await?;
        
        // CLK1 at carrier frequency with 90° offset
        self.set_frequency_with_phase(
            ClockOutput::Clk1, 
            carrier_hz, 
            90  // degrees
        ).await
    }
}
```

---

## 11. Power Management

### 11.1 Power States

```rust
pub enum PowerState {
    /// Full operation
    Active {
        cpu_freq: u32,      // 80 MHz
        peripherals: PeripheralMask,
    },
    
    /// Receive-only, reduced clock
    LowPower {
        cpu_freq: u32,      // 48 MHz
        display_dim: bool,
    },
    
    /// Minimal activity, listening for signal
    Sleep {
        wakeup_sources: WakeupMask,
    },
    
    /// Transmit mode (high current)
    Transmit {
        power_level: u8,    // 1-10 watts
    },
}

pub struct PowerManager {
    state: PowerState,
    battery_voltage: u16,
    temperature: i8,
    current_draw: u16,
}

impl PowerManager {
    pub fn estimate_battery_time(&self, capacity_mah: u16) -> Duration {
        let avg_current = match self.state {
            PowerState::Active { .. } => 80,      // mA
            PowerState::LowPower { .. } => 40,    // mA
            PowerState::Sleep { .. } => 5,        // mA
            PowerState::Transmit { power_level } => {
                100 + (power_level as u16 * 400)  // 100-500 mA
            }
        };
        
        Duration::from_secs((capacity_mah as u64 * 3600) / avg_current as u64)
    }
}
```

### 11.2 Thermal Management

```rust
pub struct ThermalMonitor {
    pa_temp: i8,
    mcu_temp: i8,
    warning_threshold: i8,
    shutdown_threshold: i8,
}

impl ThermalMonitor {
    pub fn check(&self) -> ThermalAction {
        let max_temp = self.pa_temp.max(self.mcu_temp);
        
        if max_temp > self.shutdown_threshold {
            ThermalAction::EmergencyShutdown
        } else if max_temp > self.warning_threshold {
            ThermalAction::ReducePower
        } else {
            ThermalAction::Normal
        }
    }
}
```

---

## 12. Build and Flash Workflow

### 12.1 Development Environment

```bash
# Install Rust embedded toolchain
rustup target add thumbv7em-none-eabihf

# Install probe-rs for flashing
cargo install probe-rs --features cli

# Install cargo-embed for debugging
cargo install cargo-embed
```

### 12.2 Build Commands

```bash
# Debug build
cargo build --target thumbv7em-none-eabihf

# Release build (optimized)
cargo build --release --target thumbv7em-none-eabihf

# Flash to device
cargo embed --release

# Run with RTT logging
cargo embed --release --features defmt-rtt
```

### 12.3 Memory Layout

```
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH = 256K
    RAM   : ORIGIN = 0x20000000, LENGTH = 64K
}

/* Memory allocation */
.text    : 0x08000000  /* Code */
.rodata  : 0x08020000  /* Constants, filter coefficients */
.data    : 0x20000000  /* Initialized data */
.bss     : 0x20008000  /* Uninitialized data */
.heap    : 0x2000C000  /* Dynamic allocation (limited) */
.stack   : 0x2000F000  /* Stack (4KB) */
```

---

## 13. Testing Strategy

### 13.1 Unit Tests (Host)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fir_filter() {
        let coeffs = design_lowpass(1000.0, 48000.0, 71);
        let mut filter = FirFilter::new(coeffs);
        
        // Test impulse response
        let mut output = [0i16; 128];
        let impulse = [0x7FFFi16, 0, 0, /* ... */];
        filter.process(&impulse, &mut output);
        
        // Verify frequency response
        assert!(output[0] > 0);
    }
    
    #[test]
    fn test_cat_parser() {
        let mut parser = CatParser::new();
        
        for byte in b"FA00014250000;" {
            if let Some(cmd) = parser.parse(*byte) {
                assert!(matches!(cmd, CatCommand::SetFrequency { freq: 14250000, .. }));
            }
        }
    }
}
```

### 13.2 Hardware-in-Loop Tests

```rust
#[cfg(feature = "hil-test")]
mod hil_tests {
    #[test]
    fn test_si5351_frequency_accuracy() {
        // Requires frequency counter connection
        let mut radio = init_hardware();
        
        for freq in [7_000_000, 14_000_000, 21_000_000] {
            radio.set_frequency(freq);
            let measured = frequency_counter.read();
            assert!((measured - freq).abs() < 10);  // Within 10 Hz
        }
    }
}
```

---

## 14. Future Enhancements

### 14.1 Phase 2 Features

- Digital voice modes (FreeDV)
- FT8/FT4 decode and encode
- PSK31/RTTY
- Spectrum scope on local display
- Antenna tuner control
- External amplifier interface

### 14.2 Phase 3 Features

- VHF/UHF upconverter support
- GPS integration (time sync, grid locator)
- Remote operation via network
- Mesh networking capabilities
- Machine learning noise reduction

---

## Appendix A: Pin Assignments (STM32G474RE Reference)

| Pin | Function | Peripheral |
|-----|----------|------------|
| PA0 | ADC1_IN1 | I channel input |
| PA1 | ADC1_IN2 | Q channel input |
| PA4 | DAC1_OUT1 | Audio output |
| PA5 | DAC1_OUT2 | Sidetone output |
| PA8 | I2C2_SDA | Si5351, OLED |
| PA9 | I2C2_SCL | Si5351, OLED |
| PA11 | USB_DM | USB data - |
| PA12 | USB_DP | USB data + |
| PB4 | UCPD1_CC1 | USB PD CC1 |
| PB6 | UCPD1_CC2 | USB PD CC2 |
| PB0 | GPIO | PTT sense |
| PB1 | TIM3_CH4 | Encoder A (timer input) |
| PB3 | TIM2_CH2 | Encoder B (timer input) |
| PB5 | GPIO | Encoder switch |
| PC0-2 | GPIO | Band relay control |
| PC13 | GPIO | TCPP01 enable |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| IQ | In-phase and Quadrature signal components |
| NCO | Numerically Controlled Oscillator |
| CIC | Cascaded Integrator-Comb filter |
| FIR | Finite Impulse Response filter |
| AGC | Automatic Gain Control |
| CAT | Computer Aided Transceiver (control protocol) |
| LO | Local Oscillator |
| SSB | Single Sideband modulation |
| CW | Continuous Wave (Morse code) |
| QRP | Low power operation (<5W) |
| DSP | Digital Signal Processing |

---

*Document generated for embedded SDR amateur radio project*
*Safe, verifiable Embedded Rust implementation*
