#![allow(clippy::missing_safety_doc)]
use std::cell::RefCell;
use std::ffi::CString;
use std::mem;
use std::os::raw::c_char;
use std::panic;
use std::ptr;
use std::thread;

use anyhow::Error;

thread_local! {
    pub static LAST_ERROR: RefCell<Option<Error>> = RefCell::new(None);
}

pub(crate) fn set_last_error(err: Error) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = Some(err);
    });
}

#[no_mangle]
pub unsafe extern "C" fn err_get_last_message() -> *const c_char {
    LAST_ERROR.with(|e| {
        if let Some(ref err) = e.replace(None) {
            let msg = err.to_string();
            let cause = err.backtrace();
            CString::new(format!("{}\n{}", cause, msg))
                .unwrap()
                .into_raw()
        } else {
            ptr::null()
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn err_last_message_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr))
    }
}

pub unsafe fn set_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let thread = thread::current();
        let thread = thread.name().unwrap_or("unnamed");
        let message = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };

        let description = match info.location() {
            Some(location) => format!(
                "thread '{}' panicked with '{}' at {}:{}",
                thread,
                message,
                location.file(),
                location.line()
            ),
            None => format!("thread '{}' panicked with '{}'", thread, message),
        };

        set_last_error(anyhow::anyhow!(description))
    }));
}

pub unsafe fn landingpad<F, T>(f: F) -> T
where
    F: FnOnce() -> Result<T, Error> + panic::UnwindSafe,
{
    match panic::catch_unwind(f) {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => {
            set_last_error(err);
            mem::zeroed()
        }
        Err(_) => mem::zeroed(),
    }
}

macro_rules! cffi (
    // a function that catches patnics and returns a result (err goes to tls)
    (
        $(#[$attr:meta])*
        unsafe fn $name:ident($($aname:ident: $aty:ty),* $(,)*) -> Result<$rv:ty> $body:block
    ) => (
        #[no_mangle]
        $(#[$attr])*
        pub unsafe extern "C" fn $name($($aname: $aty,)*) -> $rv
        {
            $crate::utils::landingpad(|| $body)
        }
    );

    // a function that catches patnics and returns a result (err goes to tls)
    (
        $(#[$attr:meta])*
            fn $name:ident($($aname:ident: $aty:ty),* $(,)*) -> Result<$rv:ty> $body:block
    ) => (
        #[no_mangle]
        $(#[$attr])*
            pub extern "C" fn $name($($aname: $aty,)*) -> $rv
        {
            let thunk = || $body;
            unsafe { $crate::utils::landingpad(thunk) }
        }
    );

    // a function that catches patnics and returns nothing (err goes to tls)
    (
        $(#[$attr:meta])*
        unsafe fn $name:ident($($aname:ident: $aty:ty),* $(,)*) $(-> ())* $body:block
    ) => {
        #[no_mangle]
        $(#[$attr])*
        pub unsafe extern "C" fn $name($($aname: $aty,)*)
        {
            // this silences panics and stuff
            $crate::utils::landingpad(|| { $body; Ok(0 as ::std::os::raw::c_int) });
        }
    };

    // a function that catches patnics and returns nothing (err goes to tls)
    (
        $(#[$attr:meta])*
            fn $name:ident($($aname:ident: $aty:ty),* $(,)*) $(-> ())* $body:block
    ) => {
        #[no_mangle]
        $(#[$attr])*
            pub extern "C" fn $name($($aname: $aty,)*)
        {
            // this silences panics and stuff
            let thunk =|| { $body; Ok(0 as ::std::os::raw::c_int) };
            unsafe {$crate::utils::landingpad(thunk);}
        }
    }
);
