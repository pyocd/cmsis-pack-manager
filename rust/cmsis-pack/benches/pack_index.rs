// #![feature(test)]
// extern crate cmsis_pack;
// extern crate test;

// use cmsis_pack::pack_index::{Vidx, Pidx, PdscRef};
// use cmsis_pack::parse::FromElem;
// use test::Bencher;

// trait BenchParse: FromElem {
//     const SRC: &'static [u8];

//     fn parse(b: &mut Bencher) {
//         b.iter(|| {
//             assert!(
//                 Self::from_string(String::from_utf8_lossy(Self::SRC).into_owned().as_str()).is_ok()
//             );
//         });
//     }
// }

// impl BenchParse for PdscRef {
//     const SRC: &'static [u8] = include_bytes!("bench.pdsc");
// }
// impl BenchParse for Pidx {
//     const SRC: &'static [u8] = include_bytes!("bench.pidx");
// }
// impl BenchParse for Vidx {
//     const SRC: &'static [u8] = include_bytes!("bench.vidx");
// }

// #[bench]
// fn parse_pdscref(b: &mut Bencher) {
//     PdscRef::parse(b)
// }
// #[bench]
// fn parse_pidx(b: &mut Bencher) {
//     Pidx::parse(b)
// }
// #[bench]
// fn parse_vidx(b: &mut Bencher) {
//     Vidx::parse(b)
// }
