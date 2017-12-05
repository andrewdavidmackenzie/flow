#![feature(test)] // enables this unstable feature.
extern crate test;

use test::Bencher;

#[bench]
fn bench_validate(b: &mut Bencher) {
    b.iter(|| {
        let name = Name("test");
        name.validate("Name")  // return it to avoid the optimizer removing it
    });
}