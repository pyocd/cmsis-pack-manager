#![feature(test)]
extern crate cmsis_pack_manager;
extern crate minidom;
extern crate quick_xml;
extern crate test;

use cmsis_pack_manager::pack_index::{Vidx, Pidx, PdscRef};
use cmsis_pack_manager::parse::FromElem;
use test::Bencher;

TRAIT BenchParse: FromElem {
    const src:  &'static [u8];

    #[bench]
    fn parse(b: &mut Bencher){
        b.iter(|| {
            assert!(Self::from_string(String::from_utf8_lossy(Self::src).into_owned().as_str()).is_ok());
        });
    }
}

impl BenchParse for PdscRef{
    const src: &'static [u8] = include_bytes!("bench.pdsc");
}
impl BenchParse for Pidx{
    const src: &'static [u8] = include_bytes!("bench.pidx");
}
impl BenchParse for Vidx{
    const src: &'static [u8] = include_bytes!("bench.vidx");
}

#[bench]
fn parse_pdscref(b: &mut Bencher) {PdscRef::parse(b)}
#[bench]
fn parse_pidx(b: &mut Bencher) {Pidx::parse(b)}
#[bench]
fn parse_vidx(b: &mut Bencher) {Vidx::parse(b)}
