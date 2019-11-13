use slog::{o, Logger};
use std::os::raw::c_char;
use std::ffi::CStr;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use failure::err_msg;

use cmsis_pack::cmsis_update::install;
use crate::config::ConfigBuilder;

use crate::pdsc::ParsedPacks;
use crate::pack_index::{DownloadSender, UpdatePoll, RunningUpdateContext, UpdateReturn};

cffi!{
    fn update_packs(
        pack_store: *const c_char,
        parsed_packs: *mut ParsedPacks
    ) -> Result<*mut UpdatePoll> {
        let conf_bld = ConfigBuilder::new();
        let conf_bld = if !pack_store.is_null() {
            let pstore = unsafe { CStr::from_ptr(pack_store) }.to_string_lossy();
            conf_bld.with_pack_store(pstore.into_owned())
        } else {
            conf_bld
        };
        let conf = conf_bld.build()?;
        let (send, recv) = channel();
        let done_flag = Arc::new(AtomicBool::new(false));
        let threads_done_flag = done_flag.clone();
        if !parsed_packs.is_null() {
            with_from_raw!(let mut packs = parsed_packs, {
                let size = packs.0.len();
                let packs: Vec<_> = packs.0.drain(0..size).collect();
                let thread = thread::Builder::new()
                    .name("update".to_string())
                    .spawn(move || {
                        extern crate slog_term;
                        extern crate slog_async;
                        use slog::Drain;
                        let decorator = slog_term::TermDecorator::new().build();
                        let drain = slog_term::FullFormat::new(decorator).build().fuse();
                        let drain = slog_async::Async::new(drain).build().fuse();
                        let log = Logger::root(drain, o!());
                        let res = install(
                            &conf, 
                            packs.iter(), 
                            &log, 
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
            })
        } else {
            Err(err_msg("update packs received a Null pointer"))
        }
    }
}
