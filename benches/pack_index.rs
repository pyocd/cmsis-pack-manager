#![feature(test)]
extern crate cmsis_pack_manager;
extern crate minidom;
extern crate quick_xml;
extern crate test;

use test::{Bencher, black_box};
use cmsis_pack_manager::pack_index::{Vidx, Pidx, Pdsc, Error};
use self::minidom::Element;
use self::quick_xml::reader::Reader;

#[bench]
fn empty(b: &mut Bencher){
    b.iter(|| 1)
}

#[bench]
fn parse_pdsc(b: &mut Bencher){
    let src: &[u8] = include_bytes!("bench.pdsc");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false).check_comments(false);
        let root = Element::from_reader(&mut r).unwrap();
        black_box(Pdsc::from_elem(&root));
    });
}

#[bench]
fn parse_pidx(b: &mut Bencher){
    let src: &[u8] = include_bytes!("bench.pidx");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false).check_comments(false);
        let root = Element::from_reader(&mut r).unwrap();
        black_box(Pidx::from_elem(&root));
    });
}

#[bench]
fn parse_vidx(b: &mut Bencher){
    let src: &[u8] = include_bytes!("bench.vidx");
    b.iter(|| {
        black_box(Vidx::from_str(String::from_utf8_lossy(src).into_owned().as_str()));
    });
}
