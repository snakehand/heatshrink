use super::Config;

pub struct HeatshrinkEncoder<'a, 'b> {
    cfg: Config,
    bit_index: usize,
    bit_buf: u32,
    num_bits: u8,

    input: &'a [u8],
    output: &'b mut [u8],
}

/// Errors that may be encountered when compressing data
#[derive(Debug)]
pub enum EncodeError {
    /// The output buffer was not large enough to hold the compressed data
    OutputFull,
}

/// Basic compression call. Source and destination must reside in memory,
/// and destination must be large enough to hold the compressed data,
/// or an error will be returned
pub fn encode<'a>(
    input: &[u8],
    output: &'a mut [u8],
    cfg: &Config,
) -> Result<&'a [u8], EncodeError> {
    let encoder = HeatshrinkEncoder::new(input, output, cfg);
    encoder.encode()
}

impl<'a, 'b> HeatshrinkEncoder<'a, 'b> {
    fn new(input: &'a [u8], output: &'b mut [u8], cfg: &Config) -> Self {
        let bit_index = 0;
        let bit_buf = 0;
        let num_bits = 0;
        HeatshrinkEncoder {
            cfg: *cfg,
            bit_index,
            bit_buf,
            num_bits,
            input,
            output,
        }
    }

    fn cmp(&self, idx1: usize, idx2: usize) -> u32 {
        assert!(idx1 < idx2);
        let size = 1 << self.cfg.lookahead_sz2 as usize;
        let end = self.input.len().min(idx2 + size);
        let size = end - idx2;
        let mut matched = 0;
        let all_match = self.input[idx1..idx1 + size]
            .iter()
            .enumerate()
            .zip(self.input[idx2..idx2 + size].iter())
            .map(|((i, a), b)| {
                if *a != *b {
                    matched = i as u32;
                    false
                } else {
                    true
                }
            })
            .all(|bt| bt);
        if all_match {
            size as u32
        } else {
            matched
        }
    }

    fn search(&self, head: usize) -> (usize, u32) {
        let wsize = 1 << self.cfg.window_sz2;
        let start = if wsize > head { 0 } else { head - wsize };
        let mut best = (0, 0);
        for pos in start..head {
            let clen = self.cmp(pos, head);
            if clen >= best.1 {
                best = (pos, clen);
            }
        }
        best
    }

    fn encode(mut self) -> Result<&'b [u8], EncodeError> {
        let threshold = (1 + self.cfg.lookahead_sz2 + self.cfg.window_sz2) as u32 / 8;
        let mut pos = 0;
        while pos < self.input.len() {
            let (spos, len) = self.search(pos);
            if len > threshold {
                self.emit_bits(0, 1)?;
                let rel = pos - spos;
                // println!("Ref: {} len {}", rel, len);
                self.emit_bits((rel - 1) as u16, self.cfg.window_sz2)?;
                self.emit_bits((len - 1) as u16, self.cfg.lookahead_sz2)?;
                pos += len as usize;
            } else {
                let code = self.input[pos] as u16 | 0x0100;
                self.emit_bits(code, 9)?;
                pos += 1;
            }
        }

        self.flush();
        Ok(&self.output[..self.bit_index])
    }

    fn emit_bits(&mut self, val: u16, bit_cnt: u8) -> Result<(), EncodeError> {
        assert!(val < (1 << bit_cnt as u16));
        self.bit_buf = (self.bit_buf << bit_cnt) | val as u32;
        self.num_bits += bit_cnt;
        while self.num_bits >= 8 {
            if self.bit_index >= self.output.len() {
                return Err(EncodeError::OutputFull);
            }
            self.output[self.bit_index] = (self.bit_buf >> (self.num_bits - 8)) as u8;
            self.bit_index += 1;
            self.num_bits -= 8;
        }
        Ok(())
    }

    fn flush(&mut self) {
        // There are maximum 7 unwritten bits in the bitbuffer
        if self.num_bits > 0 {
            self.output[self.bit_index] = (self.bit_buf << (8 - self.num_bits)) as u8;
            self.bit_index += 1;
            self.num_bits = 0;
        }
    }
}
