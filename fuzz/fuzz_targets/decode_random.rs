#![no_main]

use heatshrink::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 2 {
        let sz = u16::from_le_bytes(data[0..2].try_into().unwrap()) as usize;
        let mut out = Vec::with_capacity(sz);
        out.resize_with(sz, || 0);

        let cfg: Config = Default::default();
        let decoded = decode(&data[2..], &mut out, &cfg);
    }
});
