//! Checks shared by the constructors, each raising a `ValueError` that names the parameter.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Rejects a parameter that must lie within `[0, 1]`.
pub fn check_unit_interval(name: &str, value: f64) -> PyResult<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(PyValueError::new_err(format!(
            "'{name}' must be within [0.0, 1.0], got {value}"
        )));
    }
    Ok(())
}

/// Rejects a count below `min`.
pub fn check_at_least<T: PartialOrd + std::fmt::Display>(
    name: &str,
    value: T,
    min: T,
) -> PyResult<()> {
    if value < min {
        return Err(PyValueError::new_err(format!(
            "'{name}' must be at least {min}"
        )));
    }
    Ok(())
}

/// Rejects a parameter that must be strictly positive.
pub fn check_positive(name: &str, value: f64) -> PyResult<()> {
    if value <= 0.0 || value.is_nan() {
        return Err(PyValueError::new_err(format!(
            "'{name}' must be greater than 0, got {value}"
        )));
    }
    Ok(())
}

/// Rejects a parameter that must lie within the half-open interval ``(0, 1]``.
pub fn check_half_open_unit(name: &str, value: f64) -> PyResult<()> {
    if !(value > 0.0 && value <= 1.0) {
        return Err(PyValueError::new_err(format!(
            "'{name}' must be within (0.0, 1.0], got {value}"
        )));
    }
    Ok(())
}
