//! Core unit representation for the units system.
//!
//! A `ResolvedUnit` represents a physical unit as a product of base unit
//! dimensions (each raised to an integer exponent) multiplied by a scale
//! factor relative to those base units.
//!
//! # Examples
//!
//! - meter: `dims = {"m": 1}`, `scale = 1.0`
//! - millimeter: `dims = {"m": 1}`, `scale = 0.001`
//! - m²: `dims = {"m": 2}`, `scale = 1.0`
//! - m/s: `dims = {"m": 1, "s": -1}`, `scale = 1.0`
//! - dimensionless: `dims = {}`, `scale = 1.0`

use std::collections::BTreeMap;
use std::fmt;

// ============================================================================
// ResolvedUnit
// ============================================================================

/// A fully resolved physical unit with dimensional analysis support.
///
/// Internally, a unit is stored as:
/// - A sorted map of base unit names to integer exponents
/// - A scale factor that converts one unit-value to the product of base units
///
/// For example, `mm²` has `dims = {"m": 2}` and `scale = 1e-6`
/// because `(1 mm)² = (1e-3 m)² = 1e-6 m²`.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUnit {
    /// Base unit name → integer exponent.
    /// Empty means dimensionless.
    pub dims: BTreeMap<String, i32>,
    /// Conversion factor: `1 [this unit] = scale * product(base^exp)`.
    pub scale: f64,
}

#[allow(dead_code)]
impl ResolvedUnit {
    /// Create a dimensionless unit with no scale factor.
    pub fn dimensionless() -> Self {
        Self {
            dims: BTreeMap::new(),
            scale: 1.0,
        }
    }

    /// Create a base unit (scale = 1.0, single dimension with exponent 1).
    pub fn base(name: &str) -> Self {
        let mut dims = BTreeMap::new();
        dims.insert(name.to_string(), 1);
        Self { dims, scale: 1.0 }
    }

    /// Multiply two units: dims add, scales multiply.
    pub fn mul(&self, rhs: &Self) -> Self {
        let mut dims = self.dims.clone();
        for (name, exp) in &rhs.dims {
            let entry = dims.entry(name.clone()).or_insert(0);
            *entry += exp;
        }
        // Remove zero exponents (cancelled dimensions)
        dims.retain(|_, v| *v != 0);
        Self {
            dims,
            scale: self.scale * rhs.scale,
        }
    }

    /// Divide two units: dims subtract, scales divide.
    pub fn div(&self, rhs: &Self) -> Self {
        let mut dims = self.dims.clone();
        for (name, exp) in &rhs.dims {
            let entry = dims.entry(name.clone()).or_insert(0);
            *entry -= exp;
        }
        // Remove zero exponents
        dims.retain(|_, v| *v != 0);
        Self {
            dims,
            scale: self.scale / rhs.scale,
        }
    }

    /// Raise a unit to an integer power.
    pub fn pow(&self, n: i32) -> Self {
        if n == 0 {
            return Self::dimensionless();
        }
        let dims = self.dims.iter().map(|(k, v)| (k.clone(), v * n)).collect();
        Self {
            dims,
            scale: self.scale.powi(n),
        }
    }

    /// Returns true if this unit is dimensionless (all exponents zero).
    pub fn is_dimensionless(&self) -> bool {
        self.dims.is_empty()
    }

    /// Returns true if two units have the same dimensional signature
    /// (ignoring scale factor). Compatible units can be converted between.
    pub fn compatible(&self, rhs: &Self) -> bool {
        self.dims == rhs.dims
    }

    /// Compute the conversion factor from `self` to `target`.
    ///
    /// If `v` is a value in `self` units, then `v * conversion_factor(self, target)`
    /// gives the value in `target` units.
    ///
    /// Requires `self.compatible(target)`.
    ///
    /// Returns `None` if units are incompatible.
    pub fn conversion_factor(&self, target: &Self) -> Option<f64> {
        if !self.compatible(target) {
            return None;
        }
        // self.scale is "how many base units per 1 of self"
        // target.scale is "how many base units per 1 of target"
        // To convert from self to target: multiply by (self.scale / target.scale)
        Some(self.scale / target.scale)
    }

    /// Returns a human-readable string representation of this unit.
    ///
    /// Positive exponents are listed first, then negative (denominator).
    pub fn display_string(&self) -> String {
        if self.dims.is_empty() {
            return String::new();
        }

        let numerator: Vec<String> = self
            .dims
            .iter()
            .filter(|(_, e)| **e > 0)
            .map(|(name, e)| {
                let e = *e;
                if e == 1 {
                    name.clone()
                } else {
                    format!("{}^{}", name, e)
                }
            })
            .collect();

        let denominator: Vec<String> = self
            .dims
            .iter()
            .filter(|(_, e)| **e < 0)
            .map(|(name, e)| {
                let e = *e;
                if e == -1 {
                    name.clone()
                } else {
                    format!("{}^{}", name, -e)
                }
            })
            .collect();

        match (numerator.is_empty(), denominator.is_empty()) {
            (false, true) => numerator.join("*"),
            (true, false) => format!("1/{}", denominator.join("*")),
            (false, false) => format!("{}/{}", numerator.join("*"), denominator.join("*")),
            (true, true) => String::new(),
        }
    }
}

impl fmt::Display for ResolvedUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    fn meter() -> ResolvedUnit {
        ResolvedUnit::base("m")
    }

    fn second() -> ResolvedUnit {
        ResolvedUnit::base("s")
    }

    fn millimeter() -> ResolvedUnit {
        let mut dims = BTreeMap::new();
        dims.insert("m".to_string(), 1);
        ResolvedUnit { dims, scale: 0.001 }
    }

    #[test]
    fn test_base_unit() {
        let m = meter();
        assert_eq!(m.dims["m"], 1);
        assert_eq!(m.scale, 1.0);
        assert!(!m.is_dimensionless());
    }

    #[test]
    fn test_dimensionless() {
        let d = ResolvedUnit::dimensionless();
        assert!(d.is_dimensionless());
    }

    #[test]
    fn test_multiply_same_base() {
        let m = meter();
        let m2 = m.mul(&m);
        assert_eq!(m2.dims["m"], 2);
        assert_eq!(m2.scale, 1.0);
    }

    #[test]
    fn test_multiply_different_bases() {
        let m = meter();
        let s = second();
        let ms = m.mul(&s);
        assert_eq!(ms.dims["m"], 1);
        assert_eq!(ms.dims["s"], 1);
    }

    #[test]
    fn test_divide_cancels_dimensions() {
        let m = meter();
        let ratio = m.div(&m);
        assert!(ratio.is_dimensionless());
        assert_eq!(ratio.scale, 1.0);
    }

    #[test]
    fn test_divide_different_bases() {
        let m = meter();
        let s = second();
        let speed = m.div(&s);
        assert_eq!(speed.dims["m"], 1);
        assert_eq!(speed.dims["s"], -1);
    }

    #[test]
    fn test_pow() {
        let m = meter();
        let m2 = m.pow(2);
        assert_eq!(m2.dims["m"], 2);
        assert_eq!(m2.scale, 1.0);

        let m3 = m.pow(3);
        assert_eq!(m3.dims["m"], 3);
    }

    #[test]
    fn test_pow_zero() {
        let m = meter();
        let result = m.pow(0);
        assert!(result.is_dimensionless());
    }

    #[test]
    fn test_compatible_same_dims() {
        let m = meter();
        let mm = millimeter();
        assert!(m.compatible(&mm));
        assert!(mm.compatible(&m));
    }

    #[test]
    fn test_incompatible_dims() {
        let m = meter();
        let s = second();
        assert!(!m.compatible(&s));
    }

    #[test]
    fn test_conversion_factor_m_to_mm() {
        let m = meter();
        let mm = millimeter();
        // 1 m = 1000 mm → factor from m to mm = 1.0 / 0.001 = 1000
        let factor = m.conversion_factor(&mm).unwrap();
        assert!((factor - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_conversion_factor_mm_to_m() {
        let m = meter();
        let mm = millimeter();
        // 1 mm = 0.001 m → factor from mm to m = 0.001 / 1.0 = 0.001
        let factor = mm.conversion_factor(&m).unwrap();
        assert!((factor - 0.001).abs() < 1e-13);
    }

    #[test]
    fn test_conversion_factor_incompatible() {
        let m = meter();
        let s = second();
        assert_matches!(m.conversion_factor(&s), None);
    }

    #[test]
    fn test_display_base_unit() {
        let m = meter();
        assert_eq!(m.display_string(), "m");
    }

    #[test]
    fn test_display_compound_unit() {
        let m = meter();
        let s = second();
        let speed = m.div(&s);
        assert_eq!(speed.display_string(), "m/s");
    }

    #[test]
    fn test_display_power() {
        let m = meter();
        let m2 = m.pow(2);
        assert_eq!(m2.display_string(), "m^2");
    }

    #[test]
    fn test_mm_scale() {
        let mm = millimeter();
        let m = meter();
        let factor = mm.conversion_factor(&m).unwrap();
        assert!((factor - 0.001).abs() < 1e-13);
    }
}
