//! Turns optopus's `OptError` into the Python exception a caller expects.

use optopus::error::OptError;
use pyo3::exceptions::{PyOSError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// An error optopus returned for the input it was given: a file that does not parse, or a
/// solution that does not fit the instance. A `ValueError`, or an `OSError` for a failed read.
pub fn input_error(err: OptError) -> PyErr {
    match err {
        OptError::Io(io) => io.into(),
        other => PyValueError::new_err(other.to_string()),
    }
}

/// An error a heuristic returned in the middle of a run, such as an empty neighborhood. A
/// `RuntimeError`, since the arguments were valid and the search reached a state it cannot
/// continue from.
pub fn run_error(err: OptError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// Opens `path` once before optopus parses it. optopus reports a file it cannot open as a parse
/// error, so this raises the `OSError` subclass Python's own `open` would, `FileNotFoundError`
/// or `PermissionError`, with the path attached.
pub fn check_readable(path: &str) -> PyResult<()> {
    std::fs::File::open(path).map(drop).map_err(|err| {
        let Some(code) = err.raw_os_error() else {
            return PyErr::from(err);
        };
        // `OSError(errno, strerror, filename)` picks the subclass for `errno`, and its
        // `strerror` is the message alone, without the "(os error N)" Rust appends.
        let message = err.to_string();
        let strerror = message
            .strip_suffix(&format!(" (os error {code})"))
            .unwrap_or(&message)
            .to_string();
        PyOSError::new_err((code, strerror, path.to_string()))
    })
}
