#![no_std]
#![deny(warnings)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Minimal compression & decompression library for embedded use
//! Implements the Heatshrink compression algorithm
//! described here <https://github.com/atomicobject/heatshrink>
//! and here <https://spin.atomicobject.com/2013/03/14/heatshrink-embedded-data-compression/>

mod decoder;
mod encoder;

pub use decoder::{decode, DecodeError};
pub use encoder::{encode, EncodeError};

/// Structure holding the configuration parameters
/// These can be tuned to improve compression ratio
/// But they must be the same for encode() & decode()
/// calls to be able to produce the original data
#[derive(Debug, Copy, Clone)]
pub struct Config {
    pub(crate) window_sz2: u8,
    pub(crate) lookahead_sz2: u8,
}

impl Default for Config {
    fn default() -> Self {
        let window_sz2 = 11;
        let lookahead_sz2 = 4;
        Config {
            window_sz2,
            lookahead_sz2,
        }
    }
}

impl Config {
    /// Creates a new configuration object with default values
    pub fn new(window_sz2: u8, lookahead_sz2: u8) -> Result<Self, &'static str> {
        Config::default()
            .with_window(window_sz2)
            .and_then(|c| c.with_lookahead(lookahead_sz2))
    }

    /// Modifies the configuration with a desired window size ( in range 1 - 16 )
    pub fn with_window(mut self, window_sz2: u8) -> Result<Self, &'static str> {
        if window_sz2 > 16 {
            Err("Window is too large")
        } else if window_sz2 == 0 {
            Err("Window is too small")
        } else {
            self.window_sz2 = window_sz2;
            Ok(self)
        }
    }

    /// Modifies the configuration with the desired lookahead
    pub fn with_lookahead(mut self, lookahead_sz2: u8) -> Result<Self, &'static str> {
        if lookahead_sz2 > 16 {
            Err("Window is too large")
        } else if lookahead_sz2 == 0 {
            Err("Window is too small")
        } else {
            self.lookahead_sz2 = lookahead_sz2;
            Ok(self)
        }
    }
}

#[cfg(test)]
mod test {
    use super::{decoder, encoder, Config};

    fn compare(src: &[u8]) {
        let mut dst1 = [0; 100];
        let mut dst2 = [0; 100];
        let cfg = Config::new(11, 4).unwrap();
        let out1 = encoder::encode(src, &mut dst1, &cfg).unwrap();
        let out2 = decoder::decode(out1, &mut dst2, &cfg).unwrap();
        assert_eq!(src, out2);
    }

    #[test]
    fn alpha() {
        let src = [
            33, 82, 149, 84, 52, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 147, 2, 0, 0, 0, 0, 0, 0, 242, 2, 241, 2, 240,
            2, 0, 0, 0, 0, 0, 0, 47, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0,
        ];
        compare(&src);
    }

    #[test]
    fn alpha2() {
        let src = [
            33, 82, 149, 84, 52, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 147, 2, 0, 0, 0, 0, 0, 0, 242, 2, 241, 2, 240,
            2, 0, 0, 0, 0, 0, 0, 47, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            12, 17,
        ];
        compare(&src);
    }

    #[test]
    fn beta() {
        let src = [
            189, 160, 51, 163, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 199, 0, 0, 0, 0, 0, 0, 0, 166, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 154, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0,
        ];
        compare(&src);
    }

    #[test]
    fn short_encode() {
        let src = [
            189, 160, 51, 163, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 199, 0, 0, 0, 0, 0, 0, 0, 166, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 154, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0,
        ];
        let mut dst = [0; 10];
        let cfg = Config::new(11, 4).unwrap();
        assert!(encoder::encode(&src, &mut dst, &cfg).is_err());
    }

    #[test]
    fn short_decode() {
        let src = [
            189, 160, 51, 163, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 199, 0, 0, 0, 0, 0, 0, 0, 166, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 154, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0,
        ];
        let mut dst1 = [0; 100];
        let mut dst2 = [0; 30];
        let cfg = Config::new(11, 4).unwrap();
        let out1 = encoder::encode(&src, &mut dst1, &cfg).unwrap();
        assert!(decoder::decode(out1, &mut dst2, &cfg).is_err());
    }

    #[test]
    fn short_output_buffer() {
        let src = [6];
        let mut out = [0];
        let cfg: Config = Default::default();
        assert!(encoder::encode(&src, &mut out, &cfg).is_err());
    }

    #[test]
    fn clib_compatibility() {
        let src = hex_literal::hex!("90D4B2B549A408057C003E0100C9811B7CA05F1817C002DA5F04025F0005");
        let expected = hex_literal::hex!("215295543402000000000000000000000000000000000000000000000000000000000000000000009302000000000000F202F102F0020000000000002F0400000000000000000000000000000000000000000000");
        let cfg = Config::new(11, 4).unwrap();
        let mut dst1 = [0; 100];
        let decoded = decoder::decode(&src, &mut dst1, &cfg).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn random_fuzz_crash_1() {
        let src = [14, 64, 14, 64];
        let mut out = [0; 20];
        let cfg: Config = Default::default();
        let _ = decoder::decode(&src, &mut out, &cfg);
    }
}
