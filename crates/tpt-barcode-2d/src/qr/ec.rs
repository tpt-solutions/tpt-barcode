//! QR Code error-correction codeword generation.
//!
//! Wraps `tpt-barcode-core`'s Reed-Solomon encoder, handling QR's multi-block
//! interleaving scheme for versions with two block groups.

use tpt_barcode_core::reed_solomon;

use super::version::VersionInfo;

/// Build the final interleaved codeword stream for a QR Code symbol.
///
/// Returns a `Vec<u8>` of all data codewords (interleaved across blocks) followed
/// by all EC codewords (interleaved across blocks), as required by the QR spec.
#[cfg(feature = "alloc")]
pub fn interleave_blocks(data: &[u8], info: &VersionInfo) -> alloc::vec::Vec<u8> {
    extern crate alloc;
    use alloc::vec::Vec;

    let g1 = &info.group1;
    let g2 = &info.group2;
    let ec_pw = g1.ec_codewords as usize;

    // Split data into blocks
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    let mut offset = 0;
    for _ in 0..g1.count {
        let len = g1.data_codewords as usize;
        blocks.push(data[offset..offset + len].to_vec());
        offset += len;
    }
    for _ in 0..g2.count {
        let len = g2.data_codewords as usize;
        blocks.push(data[offset..offset + len].to_vec());
        offset += len;
    }

    // Compute EC codewords for each block
    let mut ec_blocks: Vec<Vec<u8>> = Vec::new();
    for block in &blocks {
        let mut ec = alloc::vec![0u8; ec_pw];
        reed_solomon::encode(block, ec_pw, &mut ec);
        ec_blocks.push(ec);
    }

    // Interleave data codewords
    let max_data = blocks.iter().map(|b| b.len()).max().unwrap_or(0);
    let mut out: Vec<u8> = Vec::new();
    for i in 0..max_data {
        for block in &blocks {
            if i < block.len() {
                out.push(block[i]);
            }
        }
    }

    // Interleave EC codewords
    for i in 0..ec_pw {
        for ec_block in &ec_blocks {
            if i < ec_block.len() {
                out.push(ec_block[i]);
            }
        }
    }

    out
}
