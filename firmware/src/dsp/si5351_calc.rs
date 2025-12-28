//! Si5351 Frequency Calculation
//!
//! Provides fractional-N PLL and multisynth divider calculations
//! for precise frequency synthesis. This module is testable on the host.
//!
//! # Theory of Operation
//!
//! The Si5351 uses a two-stage frequency synthesis:
//! 1. PLL stage: FVCO = FXTAL × (a + b/c) where 15 ≤ a ≤ 90
//! 2. Multisynth stage: FOUT = FVCO / (d + e/f) where 4 ≤ d ≤ 1800
//!
//! For best phase noise and jitter, we prefer:
//! - Higher VCO frequencies (closer to 900 MHz)
//! - Integer multisynth divisors when possible
//! - Even multisynth divisors for quadrature operation

/// PLL parameters for frequency calculation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PllParams {
    /// Integer part (15-90)
    pub a: u32,
    /// Numerator (0 to c-1)
    pub b: u32,
    /// Denominator (1-1048575)
    pub c: u32,
}

impl PllParams {
    /// Minimum PLL multiplier
    pub const MIN_A: u32 = 15;
    /// Maximum PLL multiplier
    pub const MAX_A: u32 = 90;
    /// Maximum denominator (20 bits)
    pub const MAX_C: u32 = 1_048_575;

    /// Create integer PLL params (b=0, c=1)
    #[must_use]
    pub const fn integer(a: u32) -> Self {
        Self { a, b: 0, c: 1 }
    }

    /// Create fractional PLL params
    #[must_use]
    pub const fn fractional(a: u32, b: u32, c: u32) -> Self {
        Self { a, b, c }
    }

    /// Calculate the VCO frequency given crystal frequency
    #[must_use]
    pub fn vco_frequency(&self, xtal_hz: u64) -> u64 {
        // FVCO = FXTAL × (a + b/c)
        // To avoid floating point: FVCO = (FXTAL × a × c + FXTAL × b) / c
        (xtal_hz * u64::from(self.a) * u64::from(self.c) + xtal_hz * u64::from(self.b)) / u64::from(self.c)
    }

    /// Validate parameters are in range
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.a >= Self::MIN_A
            && self.a <= Self::MAX_A
            && self.c >= 1
            && self.c <= Self::MAX_C
            && self.b < self.c
    }

    /// Calculate P1, P2, P3 register values for Si5351
    #[must_use]
    pub fn to_registers(&self) -> (u32, u32, u32) {
        // From Si5351 datasheet:
        // P1 = 128 × a + floor(128 × b/c) - 512
        // P2 = 128 × b - c × floor(128 × b/c)
        // P3 = c
        let floor_128b_c = (128 * self.b) / self.c;
        let p1 = 128 * self.a + floor_128b_c - 512;
        let p2 = 128 * self.b - self.c * floor_128b_c;
        let p3 = self.c;
        (p1, p2, p3)
    }
}

/// Multisynth divider parameters
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MsParams {
    /// Integer part (4, 6-1800)
    pub a: u32,
    /// Numerator
    pub b: u32,
    /// Denominator
    pub c: u32,
    /// R divider power of 2 (0-7 for 1, 2, 4, 8, 16, 32, 64, 128)
    pub r_div: u8,
}

impl MsParams {
    /// Minimum integer divisor
    pub const MIN_A: u32 = 4;
    /// Maximum integer divisor
    pub const MAX_A: u32 = 1800;
    /// Maximum denominator (20 bits)
    pub const MAX_C: u32 = 1_048_575;

    /// Create integer multisynth params (b=0, c=1)
    #[must_use]
    pub const fn integer(a: u32) -> Self {
        Self {
            a,
            b: 0,
            c: 1,
            r_div: 0,
        }
    }

    /// Create integer multisynth with R divider
    #[must_use]
    pub const fn integer_with_r(a: u32, r_div: u8) -> Self {
        Self {
            a,
            b: 0,
            c: 1,
            r_div,
        }
    }

    /// Create fractional multisynth params
    #[must_use]
    pub const fn fractional(a: u32, b: u32, c: u32) -> Self {
        Self {
            a,
            b,
            c,
            r_div: 0,
        }
    }

    /// Calculate output frequency given VCO frequency
    #[must_use]
    pub fn output_frequency(&self, vco_hz: u64) -> u64 {
        // FOUT = FVCO / (a + b/c) / R
        // To avoid floating point: FOUT = FVCO × c / (a × c + b) / R
        let divisor = u64::from(self.a) * u64::from(self.c) + u64::from(self.b);
        let r = 1u64 << self.r_div;
        (vco_hz * u64::from(self.c)) / divisor / r
    }

    /// Calculate effective divisor (including R divider)
    #[must_use]
    pub fn effective_divisor(&self) -> f64 {
        let base = f64::from(self.a) + f64::from(self.b) / f64::from(self.c);
        base * (1u64 << self.r_div) as f64
    }

    /// Validate parameters are in range
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        // Note: a=5 is not allowed
        let a_valid = self.a == 4 || (self.a >= 6 && self.a <= Self::MAX_A);
        let c_valid = self.c >= 1 && self.c <= Self::MAX_C;
        let b_valid = self.b < self.c;
        let r_valid = self.r_div <= 7;
        a_valid && c_valid && b_valid && r_valid
    }

    /// Check if this is an even integer divisor (required for quadrature)
    #[must_use]
    pub const fn is_even_integer(&self) -> bool {
        self.b == 0 && self.a.is_multiple_of(2) && self.r_div == 0
    }

    /// Calculate P1, P2, P3 register values
    #[must_use]
    pub fn to_registers(&self) -> (u32, u32, u32) {
        let floor_128b_c = (128 * self.b) / self.c;
        let p1 = 128 * self.a + floor_128b_c - 512;
        let p2 = 128 * self.b - self.c * floor_128b_c;
        let p3 = self.c;
        (p1, p2, p3)
    }
}

/// Minimum VCO frequency (600 MHz)
pub const VCO_MIN_HZ: u64 = 600_000_000;
/// Maximum VCO frequency (900 MHz)
pub const VCO_MAX_HZ: u64 = 900_000_000;

/// Default crystal frequency (25 MHz)
pub const DEFAULT_XTAL_HZ: u64 = 25_000_000;

/// Calculate frequency synthesis parameters for a target frequency
///
/// Returns (PLL params, Multisynth params, actual frequency in Hz, error in Hz)
#[must_use]
pub fn calculate_frequency(
    xtal_hz: u64,
    target_hz: u64,
) -> Option<(PllParams, MsParams, u64, i64)> {
    if target_hz == 0 {
        return None;
    }

    // Try to find optimal parameters
    // Strategy:
    // 1. Try integer multisynth first (best phase noise)
    // 2. Fall back to fractional multisynth if needed

    let mut best: Option<(PllParams, MsParams, u64, i64)> = None;

    // Calculate the range of valid multisynth divisors
    let ms_min = (VCO_MIN_HZ / target_hz).max(u64::from(MsParams::MIN_A));
    let ms_max = (VCO_MAX_HZ / target_hz).min(u64::from(MsParams::MAX_A));

    if ms_min > u64::from(MsParams::MAX_A) {
        // Target frequency too high, need R divider
        return calculate_with_r_divider(xtal_hz, target_hz);
    }

    // Try integer multisynth divisors first
    for ms_a in ms_min..=ms_max {
        let vco_required = target_hz * ms_a;

        if !(VCO_MIN_HZ..=VCO_MAX_HZ).contains(&vco_required) {
            continue;
        }

        // Calculate required PLL multiplier
        if let Some(pll) = calculate_pll_params(xtal_hz, vco_required) {
            let ms = MsParams::integer(ms_a as u32);
            let actual_vco = pll.vco_frequency(xtal_hz);
            let actual_freq = ms.output_frequency(actual_vco);
            let error = actual_freq as i64 - target_hz as i64;

            // Check if this is better than current best
            let should_update = match &best {
                None => true,
                Some((_, _, _, best_error)) => error.abs() < best_error.abs(),
            };

            if should_update {
                best = Some((pll, ms, actual_freq, error));

                // Perfect match, stop searching
                if error == 0 {
                    break;
                }
            }
        }
    }

    best
}

/// Calculate frequency with R divider for low frequencies
fn calculate_with_r_divider(xtal_hz: u64, target_hz: u64) -> Option<(PllParams, MsParams, u64, i64)> {
    // Try increasing R divider values
    for r_div in 1u8..=7 {
        let r = 1u64 << r_div;
        let effective_target = target_hz * r;

        // Check if effective target is in valid range
        let ms_min = (VCO_MIN_HZ / effective_target).max(u64::from(MsParams::MIN_A));
        let ms_max = (VCO_MAX_HZ / effective_target).min(u64::from(MsParams::MAX_A));

        if ms_min > u64::from(MsParams::MAX_A) {
            continue;
        }

        for ms_a in ms_min..=ms_max {
            let vco_required = effective_target * ms_a;

            if !(VCO_MIN_HZ..=VCO_MAX_HZ).contains(&vco_required) {
                continue;
            }

            if let Some(pll) = calculate_pll_params(xtal_hz, vco_required) {
                let ms = MsParams::integer_with_r(ms_a as u32, r_div);
                let actual_vco = pll.vco_frequency(xtal_hz);
                let actual_freq = ms.output_frequency(actual_vco);
                let error = actual_freq as i64 - target_hz as i64;

                return Some((pll, ms, actual_freq, error));
            }
        }
    }

    None
}

/// Calculate PLL parameters to achieve target VCO frequency
fn calculate_pll_params(xtal_hz: u64, target_vco: u64) -> Option<PllParams> {
    // PLL multiplier = VCO / XTAL
    // We want a + b/c where 15 ≤ a ≤ 90

    let a = target_vco / xtal_hz;

    if a < u64::from(PllParams::MIN_A) || a > u64::from(PllParams::MAX_A) {
        return None;
    }

    let remainder = target_vco - a * xtal_hz;

    if remainder == 0 {
        // Integer multiplier
        return Some(PllParams::integer(a as u32));
    }

    // Calculate fractional part: b/c = remainder / xtal_hz
    // Use continued fraction approximation for best precision
    let (b, c) = rational_approximation(remainder, xtal_hz, PllParams::MAX_C);

    if c > PllParams::MAX_C || b >= c {
        // Fall back to integer
        return Some(PllParams::integer(a as u32));
    }

    Some(PllParams::fractional(a as u32, b, c))
}

/// Find best rational approximation b/c ≈ num/den with c ≤ `max_c`
/// Uses the Stern-Brocot tree / mediants algorithm
fn rational_approximation(num: u64, den: u64, max_c: u32) -> (u32, u32) {
    if num == 0 {
        return (0, 1);
    }

    let mut a_num = 0u64;
    let mut a_den = 1u64;
    let mut b_num = 1u64;
    let mut b_den = 0u64;

    let target = num as f64 / den as f64;
    let mut best_b = 0u32;
    let mut best_c = 1u32;
    let mut best_error = f64::INFINITY;

    for _ in 0..64 {
        // Mediant
        let m_num = a_num + b_num;
        let m_den = a_den + b_den;

        if m_den > u64::from(max_c) {
            break;
        }

        let mediant = m_num as f64 / m_den as f64;
        let error = (mediant - target).abs();

        if error < best_error {
            best_error = error;
            best_b = m_num as u32;
            best_c = m_den as u32;
        }

        if error < 1e-12 {
            break;
        }

        if mediant < target {
            a_num = m_num;
            a_den = m_den;
        } else {
            b_num = m_num;
            b_den = m_den;
        }
    }

    (best_b, best_c)
}

/// Calculate quadrature output parameters
/// Returns parameters for generating I and Q clocks with 90° phase difference
///
/// For quadrature operation, the multisynth divisor must be an even integer.
/// The phase offset is set by programming the CLK1 phase register.
#[must_use]
pub fn calculate_quadrature(
    xtal_hz: u64,
    target_hz: u64,
) -> Option<(PllParams, MsParams, u64, i64, u8)> {
    // For quadrature, we need 4× the LO frequency and even integer divisor
    let target_4x = target_hz * 4;

    // Find even integer multisynth divisor
    let ms_min = ((VCO_MIN_HZ / target_4x).max(u64::from(MsParams::MIN_A)) | 1) + 1; // Round up to even
    let ms_max = (VCO_MAX_HZ / target_4x).min(u64::from(MsParams::MAX_A)) & !1; // Round down to even

    if ms_min > u64::from(MsParams::MAX_A) {
        return None;
    }

    let mut best: Option<(PllParams, MsParams, u64, i64, u8)> = None;

    // Step by 2 to only try even divisors
    let mut ms_a = ms_min;
    while ms_a <= ms_max {
        let vco_required = target_4x * ms_a;

        if (VCO_MIN_HZ..=VCO_MAX_HZ).contains(&vco_required) {
            if let Some(pll) = calculate_pll_params(xtal_hz, vco_required) {
                let ms = MsParams::integer(ms_a as u32);
                let actual_vco = pll.vco_frequency(xtal_hz);
                let actual_freq = ms.output_frequency(actual_vco) / 4;
                let error = actual_freq as i64 - target_hz as i64;

                // Phase offset = ms_a / 4 (for 90° at output frequency)
                let phase = (ms_a / 4) as u8;

                let should_update = match &best {
                    None => true,
                    Some((_, _, _, best_error, _)) => error.abs() < best_error.abs(),
                };

                if should_update {
                    best = Some((pll, ms, actual_freq, error, phase));

                    if error == 0 {
                        break;
                    }
                }
            }
        }
        ms_a += 2;
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pll_params_integer() {
        let pll = PllParams::integer(36);
        assert_eq!(pll.a, 36);
        assert_eq!(pll.b, 0);
        assert_eq!(pll.c, 1);
        assert!(pll.is_valid());

        // 25 MHz × 36 = 900 MHz
        let vco = pll.vco_frequency(25_000_000);
        assert_eq!(vco, 900_000_000);
    }

    #[test]
    fn pll_params_fractional() {
        let pll = PllParams::fractional(35, 1, 2);
        assert!(pll.is_valid());

        // 25 MHz × 35.5 = 887.5 MHz
        let vco = pll.vco_frequency(25_000_000);
        assert_eq!(vco, 887_500_000);
    }

    #[test]
    fn pll_params_validation() {
        assert!(!PllParams::integer(14).is_valid()); // Below min
        assert!(!PllParams::integer(91).is_valid()); // Above max
        assert!(PllParams::integer(15).is_valid());
        assert!(PllParams::integer(90).is_valid());
    }

    #[test]
    fn ms_params_integer() {
        let ms = MsParams::integer(100);
        assert_eq!(ms.a, 100);
        assert!(ms.is_valid());

        // 900 MHz / 100 = 9 MHz
        let freq = ms.output_frequency(900_000_000);
        assert_eq!(freq, 9_000_000);
    }

    #[test]
    fn ms_params_with_r_divider() {
        let ms = MsParams::integer_with_r(100, 3); // R = 8

        // 900 MHz / 100 / 8 = 1.125 MHz
        let freq = ms.output_frequency(900_000_000);
        assert_eq!(freq, 1_125_000);
    }

    #[test]
    fn ms_params_validation() {
        assert!(!MsParams::integer(3).is_valid()); // Below min
        assert!(MsParams::integer(4).is_valid()); // Min allowed
        // a=5 is not allowed per datasheet
        assert!(MsParams::integer(6).is_valid());
        assert!(!MsParams::integer(1801).is_valid()); // Above max
    }

    #[test]
    fn ms_params_even_integer() {
        assert!(MsParams::integer(100).is_even_integer());
        assert!(!MsParams::integer(101).is_even_integer());
        assert!(!MsParams::fractional(100, 1, 2).is_even_integer());
        assert!(!MsParams::integer_with_r(100, 1).is_even_integer());
    }

    #[test]
    fn calculate_7mhz() {
        let result = calculate_frequency(DEFAULT_XTAL_HZ, 7_000_000);
        assert!(result.is_some());

        let (pll, ms, _actual, error) = result.unwrap();
        assert!(pll.is_valid());
        assert!(ms.is_valid());
        assert!(error.abs() <= 100); // Within 100 Hz

        let vco = pll.vco_frequency(DEFAULT_XTAL_HZ);
        assert!(vco >= VCO_MIN_HZ && vco <= VCO_MAX_HZ);
    }

    #[test]
    fn calculate_14mhz() {
        let result = calculate_frequency(DEFAULT_XTAL_HZ, 14_000_000);
        assert!(result.is_some());

        let (pll, ms, _actual, error) = result.unwrap();
        assert!(pll.is_valid());
        assert!(ms.is_valid());

        // 14 MHz should be achievable with small error
        assert!(error.abs() <= 100);
    }

    #[test]
    fn calculate_28mhz() {
        let result = calculate_frequency(DEFAULT_XTAL_HZ, 28_000_000);
        assert!(result.is_some());

        let (pll, ms, _actual, _error) = result.unwrap();
        assert!(pll.is_valid());
        assert!(ms.is_valid());
    }

    #[test]
    fn calculate_low_frequency() {
        // 100 kHz should require R divider
        // VCO max = 900 MHz, MS max = 1800
        // 900 MHz / 1800 = 500 kHz, so below 500 kHz needs R divider
        let result = calculate_frequency(DEFAULT_XTAL_HZ, 100_000);
        assert!(result.is_some());

        let (pll, ms, _actual, _error) = result.unwrap();
        assert!(ms.r_div > 0, "Should use R divider for 100 kHz");
        assert!(pll.is_valid());
        assert!(ms.is_valid());
    }

    #[test]
    fn calculate_quadrature_7mhz() {
        let result = calculate_quadrature(DEFAULT_XTAL_HZ, 7_000_000);
        assert!(result.is_some());

        let (pll, ms, _actual, _error, phase) = result.unwrap();
        assert!(pll.is_valid());
        assert!(ms.is_valid());
        assert!(ms.is_even_integer()); // Required for quadrature
        assert!(phase > 0); // Phase offset should be non-zero
    }

    #[test]
    fn calculate_quadrature_14mhz() {
        let result = calculate_quadrature(DEFAULT_XTAL_HZ, 14_074_000);
        assert!(result.is_some());

        let (_pll, ms, _actual, error, _phase) = result.unwrap();
        assert!(ms.is_even_integer());

        // FT8 frequency should have reasonable accuracy
        assert!(error.abs() <= 10); // Within 10 Hz
    }

    #[test]
    fn pll_register_values() {
        // Test register calculation matches datasheet examples
        let pll = PllParams::integer(36);
        let (p1, p2, p3) = pll.to_registers();

        // For a=36, b=0, c=1:
        // P1 = 128×36 + 0 - 512 = 4608 - 512 = 4096
        // P2 = 0
        // P3 = 1
        assert_eq!(p1, 4096);
        assert_eq!(p2, 0);
        assert_eq!(p3, 1);
    }

    #[test]
    fn ms_register_values() {
        let ms = MsParams::integer(100);
        let (p1, p2, p3) = ms.to_registers();

        // For a=100, b=0, c=1:
        // P1 = 128×100 - 512 = 12288
        // P2 = 0
        // P3 = 1
        assert_eq!(p1, 12288);
        assert_eq!(p2, 0);
        assert_eq!(p3, 1);
    }

    #[test]
    fn rational_approximation_half() {
        let (b, c) = rational_approximation(1, 2, 1000);
        assert_eq!(b, 1);
        assert_eq!(c, 2);
    }

    #[test]
    fn rational_approximation_third() {
        let (b, c) = rational_approximation(1, 3, 1000);
        assert_eq!(b, 1);
        assert_eq!(c, 3);
    }

    #[test]
    fn rational_approximation_complex() {
        // π/4 ≈ 0.785398...
        // 355/452 ≈ 0.78539823... is a good approximation
        let num = 785398;
        let den = 1000000;
        let (b, c) = rational_approximation(num, den, 1000);

        let approx = b as f64 / c as f64;
        let actual = num as f64 / den as f64;
        assert!((approx - actual).abs() < 0.001);
    }

    #[test]
    fn vco_range_check() {
        // All generated VCO frequencies should be in valid range
        for freq in [3_500_000, 7_074_000, 14_074_000, 21_074_000] {
            if let Some((pll, _, _, _)) = calculate_frequency(DEFAULT_XTAL_HZ, freq) {
                let vco = pll.vco_frequency(DEFAULT_XTAL_HZ);
                assert!(
                    vco >= VCO_MIN_HZ && vco <= VCO_MAX_HZ,
                    "VCO {} Hz out of range for {} Hz",
                    vco,
                    freq
                );
            }
        }
    }
}
