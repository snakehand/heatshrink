#![no_main]

use heatshrink::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let sz = 2 * data.len() + 16;
    let mut out = Vec::with_capacity(sz);
    out.resize_with(sz, || 0);
    let mut res = Vec::with_capacity(sz);
    res.resize_with(sz, || 0);
    // eprintln!("{} {}", out.len(), res.len());

    let cfg: Config = Default::default();
    let encoded = encode(data, &mut out, &cfg).unwrap();
    let decoded = decode(encoded, &mut res, &cfg).unwrap();
    assert_eq!(data, decoded);
});
