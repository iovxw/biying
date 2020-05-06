use std::env;
use std::ffi::CStr;
use std::fmt;
use std::io::prelude::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use jemalloc_ctl::raw;
use jemalloc_sys::{mallctl, malloc_stats_print};
use libc::{self, ssize_t};

unsafe extern "C" fn p(_opaque: *mut c_void, buf: *const c_char) {
    let s = CStr::from_ptr(buf);
    print!("{}", s.to_string_lossy());
}

pub fn dump() {
    if env::var("WHY_YOU_EAT_SO_MUCH_MEMORY").is_ok() {
        unsafe {
            mallctl_call(b"prof.dump\0").unwrap_or_default(); // ignore error
            malloc_stats_print(Some(p), ptr::null_mut(), ptr::null_mut());
        }
    }
}

pub fn init_opts() {
    unsafe {
        // Set the decay time for any arenas that will be created in the future.
        raw::write::<ssize_t>(b"arenas.dirty_decay_ms\0", 0).unwrap();
        raw::write::<ssize_t>(b"arenas.muzzy_decay_ms\0", 0).unwrap();

        // Get the total number of arenas.
        let narenas = jemalloc_ctl::arenas::narenas::read().unwrap();

        // Change the decay on the already existing arenas.
        let mut buf = Vec::with_capacity("arena.999.dirty_decay_ms\0".len());
        for i in 0..narenas {
            write!(&mut buf, "arena.{}.dirty_decay_ms\0", i).unwrap();
            raw::write::<ssize_t>(&buf, 0).unwrap();
            write!(&mut buf, "arena.{}.muzzy_decay_ms\0", i).unwrap();
            raw::write::<ssize_t>(&buf, 0).unwrap();
        }
    }
}

pub fn release_memory_to_os() {
    unsafe {
        // #define MALLCTL_ARENAS_ALL 4096
        mallctl_call(b"arena.4096.purge\0").unwrap();
        mallctl_call(b"thread.tcache.flush\0").unwrap();
    }
}

unsafe fn mallctl_call(method: &[u8]) -> Result<(), Error> {
    let r = mallctl(
        method.as_ptr() as *const _,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        0,
    );
    if r == 0 {
        Ok(())
    } else {
        Err(Error(r))
    }
}

struct Error(c_int);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match description(self.0) {
            Some(m) => write!(f, "{}", m),
            None => write!(f, "Unknown error code: \"{}\".", self.0),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl std::error::Error for Error {}

fn description(code: c_int) -> Option<&'static str> {
    match code {
        libc::EINVAL => Some(
            "`newp` is not `NULL`, and `newlen` is too large or too \
             small. Alternatively, `*oldlenp` is too large or too \
             small; in this case as much data as possible are read \
             despite the error.",
        ),
        libc::ENOENT => Some("`name` or `mib` specifies an unknown/invalid value."),
        libc::EPERM => Some(
            "Attempt to read or write `void` value, or attempt to \
             write read-only value.",
        ),
        libc::EAGAIN => Some("A memory allocation failure occurred."),
        libc::EFAULT => Some(
            "An interface with side effects failed in some way not \
             directly related to `mallctl*()` read/write processing.",
        ),
        _ => None,
    }
}
