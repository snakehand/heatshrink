use super::Config;

#[derive(Debug, Copy, Clone)]
enum HSDstate {
    HSDSTagBit,          /* tag bit */
    HSDSYieldLiteral,    /* ready to yield literal byte */
    HSDSBackrefIndexMsb, /* most significant byte of index */
    HSDSBackrefIndexLsb, /* least significant byte of index */
    HSDSBackrefCountMsb, /* most significant byte of count */
    HSDSBackrefCountLsb, /* least significant byte of count */
    HSDSYieldBackref,    /* ready to yield back-reference */
    HSDSNeedMoreData,    /* End of input buffer detected */
    OutputFull,          /* Abort due to full output */
    IllegalBackref,      /* Abort due to illegal backref */
}

/// Errors that can be encountered while decompressing data
#[derive(Debug)]
pub enum DecodeError {
    /// The output buffer was to small to hold the decompressed data
    OutputFull,
    /// The Backrefs points outside the start of fata
    IllegalBackref,
}

pub struct HeatshrinkDecoder<'a, 'b> {
    output_count: u16,
    output_index: u16,
    state: HSDstate,
    head_index: usize, // Output position
    bit_index: usize,  // Input index
    cfg: Config,
    input: &'a [u8],
    output: &'b mut [u8],
}

/// Basice decompression call. Source and destination must reside in memory,
/// and destination must be big enough to hold the decompressed data, or an error will be returned
pub fn decode<'a>(
    input: &[u8],
    output: &'a mut [u8],
    cfg: &Config,
) -> Result<&'a [u8], DecodeError> {
    let decoder = HeatshrinkDecoder::new(input, output, cfg);
    decoder.decode()
}

impl<'a, 'b> HeatshrinkDecoder<'a, 'b> {
    fn new(input: &'a [u8], output: &'b mut [u8], cfg: &Config) -> Self {
        let output_count = 0;
        let output_index = 0;
        let head_index = 0;
        let state = HSDstate::HSDSTagBit;
        let bit_index = 0;
        HeatshrinkDecoder {
            output_count,
            output_index,
            head_index,
            state,
            bit_index,
            cfg: *cfg,
            input,
            output,
        }
    }

    fn decode(mut self) -> Result<&'b [u8], DecodeError> {
        loop {
            self.state = match self.state {
                HSDstate::HSDSTagBit => self.st_tag_bit(),
                HSDstate::HSDSYieldLiteral => self.st_yield_literal(),
                HSDstate::HSDSBackrefIndexMsb => self.st_backref_index_msb(),
                HSDstate::HSDSBackrefIndexLsb => self.st_backref_index_lsb(),
                HSDstate::HSDSBackrefCountMsb => self.st_backref_count_msb(),
                HSDstate::HSDSBackrefCountLsb => self.st_backref_count_lsb(),
                HSDstate::HSDSYieldBackref => self.st_yield_backref(),
                HSDstate::HSDSNeedMoreData => {
                    break;
                }
                HSDstate::OutputFull => {
                    return Err(DecodeError::OutputFull);
                }
                HSDstate::IllegalBackref => {
                    return Err(DecodeError::IllegalBackref);
                }
            };
            // println!("State: {:?} {:?}", self.state, self.bit_index);
            if self.input.len() * 8 < self.bit_index {
                break;
            }
            if self.output.len() < self.head_index {
                return Err(DecodeError::OutputFull);
            }
        }
        Ok(&self.output[..self.head_index])
    }

    fn get_bits(&mut self, count: u8) -> Option<u16> {
        let end_pos = self.bit_index + count as usize;
        if end_pos > self.input.len() * 8 {
            return None;
        }
        let mut num = 8 - (self.bit_index % 8);
        let mut bitbuf = self.input[self.bit_index / 8] as u32;
        let count = count as usize;
        while num < count {
            self.bit_index += 8;
            bitbuf = (bitbuf << 8) | self.input[self.bit_index / 8] as u32;
            num += 8;
        }
        bitbuf >>= num - count;
        bitbuf &= (1 << count) - 1;
        self.bit_index = end_pos;
        Some(bitbuf as u16)
    }

    fn st_tag_bit(&mut self) -> HSDstate {
        match self.get_bits(1) {
            Some(0) => {
                if self.cfg.window_sz2 > 8 {
                    HSDstate::HSDSBackrefIndexMsb
                } else {
                    self.output_index = 0;
                    HSDstate::HSDSBackrefIndexLsb
                }
            }
            Some(_) => HSDstate::HSDSYieldLiteral,
            None => HSDstate::HSDSNeedMoreData,
        }
    }

    fn st_yield_literal(&mut self) -> HSDstate {
        let byte = match self.get_bits(8) {
            Some(b) => b,
            None => {
                return HSDstate::HSDSNeedMoreData;
            }
        };
        self.output[self.head_index] = byte as u8;
        self.head_index += 1;
        HSDstate::HSDSTagBit
    }

    fn st_backref_index_msb(&mut self) -> HSDstate {
        let bit_ct = self.cfg.window_sz2 - 8;
        self.output_index = match self.get_bits(bit_ct) {
            Some(idx) => idx << 8,
            None => {
                return HSDstate::HSDSNeedMoreData;
            }
        };
        HSDstate::HSDSBackrefIndexLsb
    }

    fn st_backref_index_lsb(&mut self) -> HSDstate {
        let bit_ct = self.cfg.window_sz2.min(8);
        self.output_index = match self.get_bits(bit_ct) {
            Some(idx) => self.output_index | idx,
            None => {
                return HSDstate::HSDSNeedMoreData;
            }
        };
        self.output_index += 1;
        self.output_count = 0;
        if self.cfg.lookahead_sz2 > 8 {
            HSDstate::HSDSBackrefCountMsb
        } else {
            HSDstate::HSDSBackrefCountLsb
        }
    }

    fn st_backref_count_msb(&mut self) -> HSDstate {
        let bit_ct = self.cfg.lookahead_sz2 - 8;
        self.output_count = match self.get_bits(bit_ct) {
            Some(idx) => idx << 8,
            None => {
                return HSDstate::HSDSNeedMoreData;
            }
        };
        HSDstate::HSDSBackrefIndexLsb
    }

    fn st_backref_count_lsb(&mut self) -> HSDstate {
        let bit_ct = self.cfg.lookahead_sz2.min(8);
        self.output_count = match self.get_bits(bit_ct) {
            Some(idx) => self.output_count | idx as u16,
            None => {
                return HSDstate::HSDSNeedMoreData;
            }
        };
        self.output_count += 1;
        HSDstate::HSDSYieldBackref
    }

    fn st_yield_backref(&mut self) -> HSDstate {
        /* println!(
            "Backref: idx:{}  count:{}",
            self.output_index, self.output_count
        ); */
        let count = self.output_count as usize;
        if self.output_index as usize > self.head_index {
            return HSDstate::IllegalBackref;
        }
        let start_in = self.head_index - self.output_index as usize;
        if self.head_index + count > self.output.len() {
            return HSDstate::OutputFull;
        }
        for i in 0..count {
            self.output[self.head_index] = self.output[start_in + i];
            self.head_index += 1;
        }
        HSDstate::HSDSTagBit
    }
}
