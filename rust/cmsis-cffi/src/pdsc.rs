use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

use cmsis_pack::pdsc::{dump_devices, Package};
use cmsis_pack::utils::FromElem;
use cmsis_pack::utils::ResultLogExt;

use crate::pack_index::UpdateReturn;

cffi! {
    fn dump_pdsc_json(
        packs: *mut ParsedPacks,
        devices_dest: *const c_char,
        boards_dest: *const c_char,
    ) -> Result<()> {
        let dev_dest: Option<Cow<str>> = if !devices_dest.is_null() {
            let fname = unsafe { CStr::from_ptr(devices_dest) }.to_string_lossy();
            Some(fname)
        } else {
            None
        };
        let brd_dest: Option<Cow<str>> = if !boards_dest.is_null() {
            let fname = unsafe { CStr::from_ptr(boards_dest) }.to_string_lossy();
            Some(fname)
        } else {
            None
        };
        with_from_raw!(let filenames = packs, {
            dump_devices(&filenames.0,
                         dev_dest.map(|d| d.to_string()),
                         brd_dest.map(|d| d.to_string()),
            )
        })
    }
}

pub struct ParsedPacks(pub(crate) Vec<Package>);

impl ParsedPacks {
    pub fn iter(&self) -> impl Iterator<Item = &Package> {
        self.0.iter()
    }
}

cffi! {
    fn pack_from_path(ptr: *const c_char) -> Result<*mut UpdateReturn>{
        if !ptr.is_null() {
            let fname = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
            let mut pathbuf = PathBuf::new();
            pathbuf.push::<&str>(&fname);
            if pathbuf.exists() {
                Ok(Box::into_raw(Box::new(UpdateReturn::from_vec(vec![pathbuf]))))
            } else {
                Err(anyhow::anyhow!(format!("Could not find file {:?}", &pathbuf)))
            }
        } else {
            Err(anyhow::anyhow!("Null passed into pack_from_path"))
        }
    }
}

cffi! {
    fn parse_packs(ptr: *mut UpdateReturn) -> Result<*mut ParsedPacks>{
        if !ptr.is_null() {
            with_from_raw!(let boxed = ptr,{
                let pdsc_files = boxed.iter();
                Ok(Box::into_raw(Box::new(ParsedPacks(
                    pdsc_files
                        .filter_map(|input| Package::from_path(Path::new(input)).ok_warn())
                        .collect()))))
            })
        } else {
            Err(anyhow::anyhow!("Null Passed into parse packs."))
        }
    }
}

cffi! {
    fn parse_packs_free(ptr: *mut ParsedPacks) {
        if !ptr.is_null() {
            drop(unsafe { Box::from_raw(ptr) })
        }
    }
}

cffi! {
    fn dumps_components(ptr: *mut ParsedPacks) -> Result<*const c_char> {
        with_from_raw!(let boxed = ptr, {
            let pdscs = boxed.iter();
            let dumped_components = cmsis_pack::pdsc::dumps_components(pdscs)?;
            Ok(CString::new(dumped_components).unwrap().into_raw())
        })
    }
}
