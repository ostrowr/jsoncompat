//! Construct Python strings from already validated Rust UTF-8.
//!
//! CPython's general UTF-8 decoder may widen and resize its output as it
//! discovers non-ASCII characters. Count and select the final representation
//! first, then fill one allocation without repeating UTF-8 validation.
use pyo3::prelude::*;
use pyo3::types::PyString;

pub(crate) fn from_utf8<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyString>> {
    #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
    if text.len() >= 64 {
        use pyo3::ffi;
        // Most JSON is ASCII. Probe only a bounded prefix so this path never
        // adds a full-output scan to the standard decoder's fast ASCII case.
        // Late non-ASCII characters also work through that standard decoder.
        if text.as_bytes()[..text.len().min(256)].is_ascii() {
            return Ok(PyString::new(py, text));
        }
        // UTF-8 continuation bytes are below every non-ASCII leading byte.
        // The largest byte therefore determines the exact storage width:
        // C2/C3 => Latin-1, C4..EF => BMP, F0..F4 => supplementary planes.
        // Both reductions can scan bytes in parallel instead of decoding the
        // entire string once just to discover its maximum character.
        let maximum = match text.bytes().max().unwrap_or(0) {
            0..=0xc3 => 0xff,
            0xc4..=0xef => 0xffff,
            _ => 0x10ffff,
        };
        let length = text.chars().count();
        // A str's character count cannot exceed its allocated byte length,
        // which is bounded by isize::MAX for a valid Rust allocation.
        let object = unsafe { ffi::PyUnicode_New(length as isize, maximum) };
        if object.is_null() {
            return Err(PyErr::fetch(py));
        }
        // SAFETY: PyUnicode_New allocates a fresh, unpublished compact Unicode
        // object with `length + 1` elements of the width selected by `maximum`.
        // Both quantities were computed from this same immutable, valid str.
        // Every character fits that width. Initialize all elements and the NUL
        // terminator before publishing the owned Python reference. No borrowed
        // references to uninitialized character storage are created.
        unsafe {
            let data = ffi::PyUnicode_DATA(object);
            if maximum <= 0xff {
                let data = data.cast::<u8>();
                for (i, c) in text.chars().enumerate() {
                    data.add(i).write(c as u8);
                }
                data.add(length).write(0);
            } else if maximum <= 0xffff {
                let data = data.cast::<u16>();
                for (i, c) in text.chars().enumerate() {
                    data.add(i).write(c as u16);
                }
                data.add(length).write(0);
            } else {
                let data = data.cast::<u32>();
                for (i, c) in text.chars().enumerate() {
                    data.add(i).write(u32::from(c));
                }
                data.add(length).write(0);
            }
            return Ok(Bound::from_owned_ptr(py, object).cast_into_unchecked());
        }
    }
    Ok(PyString::new(py, text))
}
