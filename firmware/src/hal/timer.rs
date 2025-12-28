//! Timer Abstractions
//!
//! Provides timing services for sample rate generation, encoder reading,
//! and general-purpose delays.

use embassy_time::{Duration, Instant, Timer};

/// Periodic timer for sample rate generation
#[derive(Clone, Copy, Debug)]
pub struct SampleClock {
    /// Period between samples in microseconds
    period_us: u32,
    /// Last tick time
    last_tick: Option<Instant>,
}

impl SampleClock {
    /// Create a sample clock from sample rate
    #[must_use]
    pub const fn from_rate(sample_rate: u32) -> Self {
        let period_us = 1_000_000 / sample_rate;
        Self {
            period_us,
            last_tick: None,
        }
    }

    /// Create a sample clock from period in microseconds
    #[must_use]
    pub const fn from_period_us(period_us: u32) -> Self {
        Self {
            period_us,
            last_tick: None,
        }
    }

    /// Get the sample rate in Hz
    #[must_use]
    pub const fn rate_hz(&self) -> u32 {
        1_000_000 / self.period_us
    }

    /// Get period duration
    #[must_use]
    pub const fn period(&self) -> Duration {
        Duration::from_micros(self.period_us as u64)
    }

    /// Wait for next sample period
    pub async fn tick(&mut self) {
        Timer::after(self.period()).await;
        self.last_tick = Some(Instant::now());
    }

    /// Reset the clock
    pub fn reset(&mut self) {
        self.last_tick = None;
    }
}

impl defmt::Format for SampleClock {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "SampleClock({}Hz)", self.rate_hz());
    }
}

/// Stopwatch for timing measurements
#[derive(Clone, Copy, Debug)]
pub struct Stopwatch {
    start: Option<Instant>,
}

impl Stopwatch {
    /// Create a new stopped stopwatch
    #[must_use]
    pub const fn new() -> Self {
        Self { start: None }
    }

    /// Start the stopwatch
    pub fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    /// Get elapsed time (returns zero if not started)
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.start
            .map_or(Duration::from_ticks(0), |s| Instant::now() - s)
    }

    /// Get elapsed time in microseconds
    #[must_use]
    pub fn elapsed_us(&self) -> u64 {
        self.elapsed().as_micros()
    }

    /// Get elapsed time in milliseconds
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis()
    }

    /// Check if a duration has elapsed
    #[must_use]
    pub fn has_elapsed(&self, duration: Duration) -> bool {
        self.elapsed() >= duration
    }

    /// Stop and return elapsed time
    pub fn stop(&mut self) -> Duration {
        let elapsed = self.elapsed();
        self.start = None;
        elapsed
    }

    /// Restart and return previous elapsed time
    pub fn restart(&mut self) -> Duration {
        let elapsed = self.elapsed();
        self.start = Some(Instant::now());
        elapsed
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

impl defmt::Format for Stopwatch {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Stopwatch({}us)", self.elapsed_us());
    }
}

/// Timeout helper
pub struct Timeout {
    deadline: Instant,
}

impl Timeout {
    /// Create a new timeout from duration
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self {
            deadline: Instant::now() + duration,
        }
    }

    /// Create a timeout from milliseconds
    #[must_use]
    pub fn from_ms(ms: u64) -> Self {
        Self::new(Duration::from_millis(ms))
    }

    /// Create a timeout from microseconds
    #[must_use]
    pub fn from_us(us: u64) -> Self {
        Self::new(Duration::from_micros(us))
    }

    /// Check if timeout has expired
    #[must_use]
    pub fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    /// Get remaining time
    #[must_use]
    pub fn remaining(&self) -> Duration {
        let now = Instant::now();
        if now >= self.deadline {
            Duration::from_ticks(0)
        } else {
            self.deadline - now
        }
    }

    /// Wait until timeout expires
    pub async fn wait(self) {
        let remaining = self.remaining();
        if remaining.as_ticks() > 0 {
            Timer::after(remaining).await;
        }
    }
}

/// Rate limiter for periodic operations
pub struct RateLimiter {
    period: Duration,
    last: Option<Instant>,
}

impl RateLimiter {
    /// Create a rate limiter from period
    #[must_use]
    pub const fn new(period: Duration) -> Self {
        Self { period, last: None }
    }

    /// Create a rate limiter from frequency
    #[must_use]
    pub fn from_hz(hz: u32) -> Self {
        let period = Duration::from_micros(1_000_000 / u64::from(hz));
        Self::new(period)
    }

    /// Check if enough time has passed (and update if so)
    pub fn check(&mut self) -> bool {
        let now = Instant::now();

        match self.last {
            None => {
                self.last = Some(now);
                true
            }
            Some(last) if now - last >= self.period => {
                self.last = Some(now);
                true
            }
            Some(_) => false,
        }
    }

    /// Get time until next allowed operation
    #[must_use]
    pub fn time_until_ready(&self) -> Duration {
        match self.last {
            None => Duration::from_ticks(0),
            Some(last) => {
                let elapsed = Instant::now() - last;
                if elapsed >= self.period {
                    Duration::from_ticks(0)
                } else {
                    self.period - elapsed
                }
            }
        }
    }

    /// Wait until ready, then proceed
    pub async fn wait_ready(&mut self) {
        let wait_time = self.time_until_ready();
        if wait_time.as_ticks() > 0 {
            Timer::after(wait_time).await;
        }
        self.last = Some(Instant::now());
    }
}

/// Encoder position counter using timer in quadrature mode
#[derive(Clone, Copy, Debug, Default)]
pub struct EncoderPosition {
    /// Raw counter value
    count: i32,
    /// Last read value for delta calculation
    last_count: i32,
}

impl EncoderPosition {
    /// Create a new encoder position
    #[must_use]
    pub const fn new() -> Self {
        Self {
            count: 0,
            last_count: 0,
        }
    }

    /// Update with new counter value
    pub fn update(&mut self, new_count: u16) {
        // Handle 16-bit wraparound
        let delta = i32::from(new_count).wrapping_sub(self.count);
        if delta > 32768 {
            self.count -= 65536 - delta;
        } else if delta < -32768 {
            self.count += 65536 + delta;
        } else {
            self.count += delta;
        }
    }

    /// Get position change since last call
    pub fn delta(&mut self) -> i32 {
        let delta = self.count - self.last_count;
        self.last_count = self.count;
        delta
    }

    /// Get absolute position
    #[must_use]
    pub const fn position(&self) -> i32 {
        self.count
    }

    /// Reset position to zero
    pub fn reset(&mut self) {
        self.count = 0;
        self.last_count = 0;
    }
}

impl defmt::Format for EncoderPosition {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Enc({})", self.count);
    }
}
