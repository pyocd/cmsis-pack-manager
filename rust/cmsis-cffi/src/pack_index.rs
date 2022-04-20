#![allow(clippy::missing_safety_doc)]
use std::borrow::{Borrow, BorrowMut};
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use anyhow::{anyhow, Error};

use crate::config::{read_vidx_list, ConfigBuilder, DEFAULT_VIDX_LIST};
use crate::utils::set_last_error;
use cmsis_pack::update::update;
use cmsis_pack::update::DownloadProgress;

pub struct UpdateReturn(pub(crate) Vec<PathBuf>);

pub struct RunningUpdateContext {
    pub(crate) thread_handle: thread::JoinHandle<Result<UpdateReturn, Error>>,
    pub(crate) done_flag: Arc<AtomicBool>,
    pub(crate) result_stream: Receiver<DownloadUpdate>,
}

#[repr(C)]
pub struct DownloadUpdate {
    pub is_size: bool,
    pub size: usize,
}

pub(crate) struct DownloadSender(Sender<DownloadUpdate>);

impl DownloadSender {
    pub(crate) fn from_sender(from: Sender<DownloadUpdate>) -> Self {
        DownloadSender(from)
    }
}

impl DownloadProgress for DownloadSender {
    fn size(&self, size: usize) {
        let _ = self.0.send(DownloadUpdate {
            is_size: true,
            size,
        });
    }

    fn progress(&self, _: usize) {
        /* not implemented */
    }

    fn complete(&self) {
        let _ = self.0.send(DownloadUpdate {
            is_size: false,
            size: 0,
        });
    }

    fn for_file(&self, _: &str) -> Self {
        DownloadSender(self.0.clone())
    }
}

/* Turns out that this enum is not UnwindSafe, so you may not call panic_unwind
 * on any closure that takes an UpdatePoll as an argument */
pub enum UpdatePoll {
    Running(RunningUpdateContext),
    Complete(Result<UpdateReturn, Error>),
    Drained,
}

impl UpdateReturn {
    pub fn from_vec(inner: Vec<PathBuf>) -> Self {
        UpdateReturn(inner)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.0.iter()
    }
}

cffi! {
    fn update_pdsc_index(
        pack_store: *const c_char,
        vidx_list: *const c_char,
    ) -> Result<*mut UpdatePoll> {
        let conf_bld = ConfigBuilder::default();
        let conf_bld = if !pack_store.is_null() {
            let pstore = unsafe { CStr::from_ptr(pack_store) }.to_string_lossy();
            conf_bld.with_pack_store(pstore.into_owned())
        } else {
            conf_bld
        };
        let vidx_list = if !vidx_list.is_null() {
            let vlist = unsafe { CStr::from_ptr(vidx_list) }.to_string_lossy();
            read_vidx_list(vlist.as_ref().as_ref())
        } else {
            DEFAULT_VIDX_LIST.iter().map(|s| String::from(*s)).collect()
        };
        let conf = conf_bld.build()?;
        let (send, recv) = channel();
        let done_flag = Arc::new(AtomicBool::new(false));
        let threads_done_flag = done_flag.clone();
        let thread = thread::Builder::new()
            .name("update".to_string())
            .spawn(move || {
                let res = update(
                    &conf,
                    vidx_list,
                    DownloadSender::from_sender(send)
                ).map(UpdateReturn);
                threads_done_flag.store(true, Ordering::Release);
                res
            })?;
        Ok(Box::into_raw(Box::new(UpdatePoll::Running(RunningUpdateContext{
            thread_handle: thread,
            done_flag,
            result_stream: recv,
        }))))
    }
}

#[no_mangle]
pub unsafe extern "C" fn update_pdsc_poll(ptr: *mut UpdatePoll) -> bool {
    if !ptr.is_null() {
        with_from_raw!(let mut boxed = ptr,{
            let (ret, next_state) = match mem::replace(boxed.borrow_mut(), UpdatePoll::Drained) {
                UpdatePoll::Complete(inner) => (true, UpdatePoll::Complete(inner)),
                UpdatePoll::Drained => (true, UpdatePoll::Drained),
                UpdatePoll::Running(cont) => {
                    if cont.done_flag.load(Ordering::Acquire) {
                        let response = cont.thread_handle.join();
                        let response = match response {
                            Ok(inner) => inner,
                            Err(_) => Err(anyhow!("thread paniced"))
                        };
                        (true, UpdatePoll::Complete(response))
                    } else {
                        (false, UpdatePoll::Running(cont))
                    }
                }
            };
            mem::replace(boxed.borrow_mut(), next_state);
            ret
        })
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn update_pdsc_get_status(ptr: *mut UpdatePoll) -> *mut DownloadUpdate {
    if !ptr.is_null() {
        with_from_raw!(let boxed = ptr,{
            match boxed.borrow() {
                UpdatePoll::Complete(_) => null_mut(),
                UpdatePoll::Drained => null_mut(),
                UpdatePoll::Running(ref cont) => {
                    let response = cont.result_stream.try_recv();
                    match response {
                        Ok(inner) => Box::into_raw(Box::new(inner)),
                        Err(_) => null_mut()
                    }
                }
            }
        })
    } else {
        null_mut()
    }
}

cffi! {
    fn update_pdsc_status_free(ptr: *mut DownloadUpdate) {
        if !ptr.is_null() {
            drop(unsafe { Box::from_raw(ptr) })
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn update_pdsc_result(ptr: *mut UpdatePoll) -> *mut UpdateReturn {
    if !ptr.is_null() {
        with_from_raw!(let mut boxed = ptr,{
            let (ret, next_state) = match mem::replace(boxed.borrow_mut(), UpdatePoll::Drained) {
                UpdatePoll::Complete(inner) => (Some(inner), UpdatePoll::Drained),
                UpdatePoll::Drained => (None, UpdatePoll::Drained),
                UpdatePoll::Running(cont) => (None, UpdatePoll::Running(cont))
            };
            mem::replace(boxed.borrow_mut(), next_state);
            match ret {
                Some(Ok(inner)) => Box::into_raw(Box::new(inner)),
                Some(Err(inner)) => {
                    println!("{:?}", inner);
                    set_last_error(inner);
                    null_mut()
                },
                None => null_mut()
            }
        })
    } else {
        null_mut()
    }
}

#[no_mangle]
pub extern "C" fn update_pdsc_index_new() -> *mut UpdateReturn {
    Box::into_raw(Box::new(UpdateReturn(Vec::new())))
}

cffi! {
    fn update_pdsc_index_next(ptr: *mut UpdateReturn) -> Result<*const c_char> {
        if !ptr.is_null() {
            with_from_raw!(let mut boxed = ptr, {
                if let Some(osstr) = boxed.0.pop().map(|p| p.into_os_string()){
                    match osstr.to_str() {
                        Some(osstr) => {
                            Ok(CString::new(osstr).map(|cstr| cstr.into_raw())?)
                        },
                        None => Err(anyhow!("Could not create a C string from a Rust String"))
                    }
                } else {
                    Ok(null_mut())
                }
            })
        } else {
            Err(anyhow!("update pdsc index next called with null"))
        }
    }
}

cffi! {
    fn update_pdsc_index_push(ptr: *mut UpdateReturn, cstr: *mut c_char) -> Result<()> {
        if !ptr.is_null() && !cstr.is_null() {
            with_from_raw!(let mut boxed = ptr, {
                let pstore = unsafe { CStr::from_ptr(cstr) }.to_string_lossy();
                boxed.0.push(pstore.into_owned().into());
                Ok(())
            })
        } else {
            Err(anyhow!("update pdsc index push called with null"))
        }
    }
}

cffi! {
    fn cstring_free(ptr: *mut c_char) {
        if !ptr.is_null() {
            drop(unsafe { CString::from_raw(ptr) })
        }
    }
}

cffi! {
    fn update_pdsc_index_free(ptr: *mut UpdateReturn) {
        if !ptr.is_null() {
            drop(unsafe { Box::from_raw(ptr) })
        }
    }
}
