// (c) Copyright 2019-2023 MIT
use crate::bindings;
use crate::bindings::{VipsArrayDouble, VipsArrayImage, VipsArrayInt};
use crate::error::Error;
use crate::ops::ForeignKeep;
use crate::Result;
use crate::VipsImage;
use std::ffi::c_char;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;

pub(crate) struct VipsArrayIntWrapper {
    pub ctx: *mut VipsArrayInt,
}
pub(crate) struct VipsArrayDoubleWrapper {
    pub ctx: *mut VipsArrayDouble,
}
pub(crate) struct VipsArrayImageWrapper {
    pub ctx: *mut VipsArrayImage,
}

impl Drop for VipsArrayIntWrapper {
    fn drop(&mut self) {
        unsafe {
            bindings::vips_area_unref(self.ctx as *mut bindings::VipsArea);
        }
    }
}

impl Drop for VipsArrayDoubleWrapper {
    fn drop(&mut self) {
        unsafe {
            bindings::vips_area_unref(self.ctx as *mut bindings::VipsArea);
        }
    }
}

impl Drop for VipsArrayImageWrapper {
    fn drop(&mut self) {
        unsafe {
            bindings::vips_area_unref(self.ctx as *mut bindings::VipsArea);
        }
    }
}

impl From<&[i32]> for VipsArrayIntWrapper {
    #[inline]
    fn from(array: &[i32]) -> Self {
        VipsArrayIntWrapper {
            ctx: unsafe { bindings::vips_array_int_new(array.as_ptr(), array.len() as i32) },
        }
    }
}

impl From<&[f64]> for VipsArrayDoubleWrapper {
    #[inline]
    fn from(array: &[f64]) -> Self {
        VipsArrayDoubleWrapper {
            ctx: unsafe { bindings::vips_array_double_new(array.as_ptr(), array.len() as i32) },
        }
    }
}

impl From<&[VipsImage]> for VipsArrayImageWrapper {
    #[inline]
    fn from(array: &[VipsImage]) -> Self {
        let len = array.len() as i32;
        let mut vips_ptrs: Vec<*mut bindings::VipsImage> = array.iter().map(|v| v.ctx).collect();
        VipsArrayImageWrapper {
            ctx: unsafe { bindings::vips_array_image_new(vips_ptrs.as_mut_ptr(), len) },
        }
    }
}

#[inline]
pub fn result<T>(res: i32, output: T, error: Error) -> Result<T> {
    if res == 0 {
        Ok(output)
    } else {
        Err(error)
    }
}

#[inline]
pub(crate) fn new_c_string(string: &str) -> Result<CString> {
    CString::new(string).map_err(|_| Error::InitializationError("Error initializing C string."))
}

/// True when the linked libvips is at least `major.minor`.
pub(crate) fn libvips_at_least(major: i32, minor: i32) -> bool {
    let linked = unsafe { (bindings::vips_version(0), bindings::vips_version(1)) };
    linked >= (major, minor)
}

/// Name of an optional libvips argument, for the C name/value varargs.
///
/// `as_ptr` is NULL when the argument is not `used` (the linked libvips lacks
/// it, or its value is unset). A NULL name ends libvips' list, so everything
/// after it is ignored: put such arguments last, oldest first, so skipping one
/// only ever skips arguments that are newer still.
pub(crate) struct OptName {
    name: CString,
    used: bool,
}

impl OptName {
    pub(crate) fn new(name: &str, used: bool) -> Result<Self> {
        Ok(OptName {
            name: new_c_string(name)?,
            used,
        })
    }

    pub(crate) fn as_ptr(&self) -> *const c_char {
        if self.used {
            self.name.as_ptr()
        } else {
            std::ptr::null()
        }
    }
}

/// The metadata argument of a saver: `keep` (libvips 8.15 and later) or its
/// predecessor `strip`. Both take an int; older libvips can only drop all
/// metadata or keep all of it.
pub(crate) fn keep_option(keep: ForeignKeep) -> (&'static CStr, i32) {
    if libvips_at_least(8, 15) {
        (c"keep", keep as i32)
    } else {
        (c"strip", matches!(keep, ForeignKeep::None) as i32)
    }
}

#[inline]
pub(crate) unsafe fn new_byte_array(buf: *mut c_void, size: u64) -> Vec<u8> {
    Vec::from_raw_parts(buf as *mut u8, size as usize, size as usize)
}

#[inline]
pub unsafe fn new_int_array(array: *mut i32, size: u64) -> Vec<i32> {
    Vec::from(std::slice::from_raw_parts(array as *mut i32, size as usize))
}

#[inline]
pub unsafe fn new_double_array(array: *mut f64, size: u64) -> Vec<f64> {
    Vec::from(std::slice::from_raw_parts(array as *mut f64, size as usize))
}
