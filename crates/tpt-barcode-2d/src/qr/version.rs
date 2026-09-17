//! QR Code version table (versions 1–40) and capacity/EC-block lookup.

use super::mode::Mode;
use tpt_barcode_core::traits::EcLevel;

/// EC block specification: (num_blocks, data_codewords_per_block, ec_codewords_per_block).
#[derive(Clone, Copy, Debug)]
pub struct EcBlock {
    /// Number of blocks in this group.
    pub count: u8,
    /// Data codewords per block.
    pub data_codewords: u8,
    /// Error-correction codewords per block.
    pub ec_codewords: u8,
}

/// Version parameters for a given (version, EcLevel) combination.
#[derive(Clone, Copy, Debug)]
pub struct VersionInfo {
    /// QR Code version (1–40).
    pub version: u8,
    /// Total codewords (data + EC) in the symbol.
    pub total_codewords: u16,
    /// Block group 1.
    pub group1: EcBlock,
    /// Block group 2 (count=0 means unused).
    pub group2: EcBlock,
}

impl VersionInfo {
    /// Total data codewords available.
    pub fn data_codewords(&self) -> usize {
        (self.group1.count as usize * self.group1.data_codewords as usize)
            + (self.group2.count as usize * self.group2.data_codewords as usize)
    }

    /// Total EC codewords per block (same for both groups per QR spec).
    pub fn ec_per_block(&self) -> usize {
        self.group1.ec_codewords as usize
    }
}

/// Module size of a version (21 + (version-1)*4).
pub const fn modules(version: u8) -> usize {
    21 + (version as usize - 1) * 4
}

/// Alignment pattern center positions for versions 2–40.
/// Index 0 = version 1 (empty), index n = version n+1.
pub const fn alignment_positions(version: u8) -> &'static [u8] {
    // From ISO 18004:2015 Annex E
    match version {
        1 => &[],
        2 => &[6, 18],
        3 => &[6, 22],
        4 => &[6, 26],
        5 => &[6, 30],
        6 => &[6, 34],
        7 => &[6, 22, 38],
        8 => &[6, 24, 42],
        9 => &[6, 26, 46],
        10 => &[6, 28, 50],
        11 => &[6, 30, 54],
        12 => &[6, 32, 58],
        13 => &[6, 34, 62],
        14 => &[6, 26, 46, 66],
        15 => &[6, 26, 48, 70],
        16 => &[6, 26, 50, 74],
        17 => &[6, 30, 54, 78],
        18 => &[6, 30, 56, 82],
        19 => &[6, 30, 58, 86],
        20 => &[6, 34, 62, 90],
        21 => &[6, 28, 50, 72, 94],
        22 => &[6, 26, 50, 74, 98],
        23 => &[6, 30, 54, 78, 102],
        24 => &[6, 28, 54, 80, 106],
        25 => &[6, 32, 58, 84, 110],
        26 => &[6, 30, 58, 86, 114],
        27 => &[6, 34, 62, 90, 118],
        28 => &[6, 26, 50, 74, 98, 122],
        29 => &[6, 30, 54, 78, 102, 126],
        30 => &[6, 26, 52, 78, 104, 130],
        31 => &[6, 30, 56, 82, 108, 134],
        32 => &[6, 34, 60, 86, 112, 138],
        33 => &[6, 30, 58, 86, 114, 142],
        34 => &[6, 34, 62, 90, 118, 146],
        35 => &[6, 30, 54, 78, 102, 126, 150],
        36 => &[6, 24, 50, 76, 102, 128, 154],
        37 => &[6, 28, 54, 80, 106, 132, 158],
        38 => &[6, 32, 58, 84, 110, 136, 162],
        39 => &[6, 26, 54, 82, 110, 138, 166],
        40 => &[6, 30, 58, 86, 114, 142, 170],
        _ => &[],
    }
}

/// Look up version info for the given version (1–40) and EC level.
/// Returns `None` if the version is out of range.
///
/// Block structure from ISO 18004:2015 Table 9:
/// `(total_codewords, g1_count, g1_data, ec_per_block, g2_count, g2_data)`.
pub fn version_info(version: u8, ec: EcLevel) -> Option<VersionInfo> {
    if !(1..=40).contains(&version) {
        return None;
    }
    let (tc, g1c, g1d, ec_pw, g2c, g2d) = match (version, ec) {
        (1, EcLevel::L) => (26, 1, 19, 7, 0, 0),
        (1, EcLevel::M) => (26, 1, 16, 10, 0, 0),
        (1, EcLevel::Q) => (26, 1, 13, 13, 0, 0),
        (1, EcLevel::H) => (26, 1, 9, 17, 0, 0),
        (2, EcLevel::L) => (44, 1, 34, 10, 0, 0),
        (2, EcLevel::M) => (44, 1, 28, 16, 0, 0),
        (2, EcLevel::Q) => (44, 1, 22, 22, 0, 0),
        (2, EcLevel::H) => (44, 1, 16, 28, 0, 0),
        (3, EcLevel::L) => (70, 1, 55, 15, 0, 0),
        (3, EcLevel::M) => (70, 1, 44, 26, 0, 0),
        (3, EcLevel::Q) => (70, 2, 17, 18, 0, 0),
        (3, EcLevel::H) => (70, 2, 13, 22, 0, 0),
        (4, EcLevel::L) => (100, 1, 80, 20, 0, 0),
        (4, EcLevel::M) => (100, 2, 32, 18, 0, 0),
        (4, EcLevel::Q) => (100, 2, 24, 26, 0, 0),
        (4, EcLevel::H) => (100, 4, 9, 16, 0, 0),
        (5, EcLevel::L) => (134, 1, 108, 26, 0, 0),
        (5, EcLevel::M) => (134, 2, 43, 24, 0, 0),
        (5, EcLevel::Q) => (134, 2, 15, 18, 2, 16),
        (5, EcLevel::H) => (134, 2, 11, 22, 2, 12),
        (6, EcLevel::L) => (172, 2, 68, 18, 0, 0),
        (6, EcLevel::M) => (172, 4, 27, 16, 0, 0),
        (6, EcLevel::Q) => (172, 4, 19, 24, 0, 0),
        (6, EcLevel::H) => (172, 4, 15, 28, 0, 0),
        (7, EcLevel::L) => (196, 2, 78, 20, 0, 0),
        (7, EcLevel::M) => (196, 4, 31, 18, 0, 0),
        (7, EcLevel::Q) => (196, 2, 14, 18, 4, 15),
        (7, EcLevel::H) => (196, 4, 13, 26, 1, 14),
        (8, EcLevel::L) => (242, 2, 97, 24, 0, 0),
        (8, EcLevel::M) => (242, 2, 38, 22, 2, 39),
        (8, EcLevel::Q) => (242, 4, 18, 22, 2, 19),
        (8, EcLevel::H) => (242, 4, 14, 26, 2, 15),
        (9, EcLevel::L) => (292, 2, 116, 30, 0, 0),
        (9, EcLevel::M) => (292, 3, 36, 22, 2, 37),
        (9, EcLevel::Q) => (292, 4, 16, 20, 4, 17),
        (9, EcLevel::H) => (292, 4, 12, 24, 4, 13),
        (10, EcLevel::L) => (346, 2, 68, 18, 2, 69),
        (10, EcLevel::M) => (346, 4, 43, 26, 1, 44),
        (10, EcLevel::Q) => (346, 6, 19, 24, 2, 20),
        (10, EcLevel::H) => (346, 6, 15, 28, 2, 16),
        (11, EcLevel::L) => (404, 4, 81, 20, 0, 0),
        (11, EcLevel::M) => (404, 1, 50, 30, 4, 51),
        (11, EcLevel::Q) => (404, 4, 22, 28, 4, 23),
        (11, EcLevel::H) => (404, 3, 12, 24, 8, 13),
        (12, EcLevel::L) => (466, 2, 92, 24, 2, 93),
        (12, EcLevel::M) => (466, 6, 36, 22, 2, 37),
        (12, EcLevel::Q) => (466, 4, 20, 26, 6, 21),
        (12, EcLevel::H) => (466, 7, 14, 28, 4, 15),
        (13, EcLevel::L) => (532, 4, 107, 26, 0, 0),
        (13, EcLevel::M) => (532, 8, 37, 22, 1, 38),
        (13, EcLevel::Q) => (532, 8, 20, 24, 4, 21),
        (13, EcLevel::H) => (532, 12, 11, 22, 4, 12),
        (14, EcLevel::L) => (581, 3, 115, 30, 1, 116),
        (14, EcLevel::M) => (581, 4, 40, 24, 5, 41),
        (14, EcLevel::Q) => (581, 11, 16, 20, 5, 17),
        (14, EcLevel::H) => (581, 11, 12, 24, 5, 13),
        (15, EcLevel::L) => (655, 5, 87, 22, 1, 88),
        (15, EcLevel::M) => (655, 5, 41, 24, 5, 42),
        (15, EcLevel::Q) => (655, 5, 24, 30, 7, 25),
        (15, EcLevel::H) => (655, 11, 12, 24, 7, 13),
        (16, EcLevel::L) => (733, 5, 98, 24, 1, 99),
        (16, EcLevel::M) => (733, 7, 45, 28, 3, 46),
        (16, EcLevel::Q) => (733, 15, 19, 24, 2, 20),
        (16, EcLevel::H) => (733, 3, 15, 30, 13, 16),
        (17, EcLevel::L) => (815, 1, 107, 28, 5, 108),
        (17, EcLevel::M) => (815, 10, 46, 28, 1, 47),
        (17, EcLevel::Q) => (815, 1, 22, 28, 15, 23),
        (17, EcLevel::H) => (815, 2, 14, 28, 17, 15),
        (18, EcLevel::L) => (901, 5, 120, 30, 1, 121),
        (18, EcLevel::M) => (901, 9, 43, 26, 4, 44),
        (18, EcLevel::Q) => (901, 17, 22, 28, 1, 23),
        (18, EcLevel::H) => (901, 2, 14, 28, 19, 15),
        (19, EcLevel::L) => (991, 3, 113, 28, 4, 114),
        (19, EcLevel::M) => (991, 3, 44, 26, 11, 45),
        (19, EcLevel::Q) => (991, 17, 21, 26, 4, 22),
        (19, EcLevel::H) => (991, 9, 13, 26, 16, 14),
        (20, EcLevel::L) => (1085, 3, 107, 28, 5, 108),
        (20, EcLevel::M) => (1085, 3, 41, 26, 13, 42),
        (20, EcLevel::Q) => (1085, 15, 24, 30, 5, 25),
        (20, EcLevel::H) => (1085, 15, 15, 28, 10, 16),
        (21, EcLevel::L) => (1156, 4, 116, 28, 4, 117),
        (21, EcLevel::M) => (1156, 17, 42, 26, 0, 0),
        (21, EcLevel::Q) => (1156, 17, 22, 28, 6, 23),
        (21, EcLevel::H) => (1156, 19, 16, 30, 6, 17),
        (22, EcLevel::L) => (1258, 2, 111, 28, 7, 112),
        (22, EcLevel::M) => (1258, 17, 46, 28, 0, 0),
        (22, EcLevel::Q) => (1258, 7, 24, 30, 16, 25),
        (22, EcLevel::H) => (1258, 34, 13, 24, 0, 0),
        (23, EcLevel::L) => (1364, 4, 121, 30, 5, 122),
        (23, EcLevel::M) => (1364, 4, 47, 28, 14, 48),
        (23, EcLevel::Q) => (1364, 11, 24, 30, 14, 25),
        (23, EcLevel::H) => (1364, 16, 15, 30, 14, 16),
        (24, EcLevel::L) => (1474, 6, 117, 30, 4, 118),
        (24, EcLevel::M) => (1474, 6, 45, 28, 14, 46),
        (24, EcLevel::Q) => (1474, 11, 24, 30, 16, 25),
        (24, EcLevel::H) => (1474, 30, 16, 30, 2, 17),
        (25, EcLevel::L) => (1588, 8, 106, 26, 4, 107),
        (25, EcLevel::M) => (1588, 8, 47, 28, 13, 48),
        (25, EcLevel::Q) => (1588, 7, 24, 30, 22, 25),
        (25, EcLevel::H) => (1588, 22, 15, 30, 13, 16),
        (26, EcLevel::L) => (1706, 10, 114, 28, 2, 115),
        (26, EcLevel::M) => (1706, 19, 46, 28, 4, 47),
        (26, EcLevel::Q) => (1706, 28, 22, 28, 6, 23),
        (26, EcLevel::H) => (1706, 33, 16, 30, 4, 17),
        (27, EcLevel::L) => (1828, 8, 122, 30, 4, 123),
        (27, EcLevel::M) => (1828, 22, 45, 28, 3, 46),
        (27, EcLevel::Q) => (1828, 8, 23, 30, 26, 24),
        (27, EcLevel::H) => (1828, 12, 15, 30, 28, 16),
        (28, EcLevel::L) => (1921, 3, 117, 30, 10, 118),
        (28, EcLevel::M) => (1921, 3, 45, 28, 23, 46),
        (28, EcLevel::Q) => (1921, 4, 24, 30, 31, 25),
        (28, EcLevel::H) => (1921, 11, 15, 30, 31, 16),
        (29, EcLevel::L) => (2051, 7, 116, 30, 7, 117),
        (29, EcLevel::M) => (2051, 21, 45, 28, 7, 46),
        (29, EcLevel::Q) => (2051, 1, 23, 30, 37, 24),
        (29, EcLevel::H) => (2051, 19, 15, 30, 26, 16),
        (30, EcLevel::L) => (2185, 5, 115, 30, 10, 116),
        (30, EcLevel::M) => (2185, 19, 47, 28, 10, 48),
        (30, EcLevel::Q) => (2185, 15, 24, 30, 25, 25),
        (30, EcLevel::H) => (2185, 23, 15, 30, 25, 16),
        (31, EcLevel::L) => (2323, 13, 115, 30, 3, 116),
        (31, EcLevel::M) => (2323, 2, 46, 28, 29, 47),
        (31, EcLevel::Q) => (2323, 42, 24, 30, 1, 25),
        (31, EcLevel::H) => (2323, 23, 15, 30, 28, 16),
        (32, EcLevel::L) => (2465, 17, 115, 30, 0, 0),
        (32, EcLevel::M) => (2465, 10, 46, 28, 23, 47),
        (32, EcLevel::Q) => (2465, 10, 24, 30, 35, 25),
        (32, EcLevel::H) => (2465, 19, 15, 30, 35, 16),
        (33, EcLevel::L) => (2611, 17, 115, 30, 1, 116),
        (33, EcLevel::M) => (2611, 14, 46, 28, 21, 47),
        (33, EcLevel::Q) => (2611, 29, 24, 30, 19, 25),
        (33, EcLevel::H) => (2611, 11, 15, 30, 46, 16),
        (34, EcLevel::L) => (2761, 13, 115, 30, 6, 116),
        (34, EcLevel::M) => (2761, 14, 46, 28, 23, 47),
        (34, EcLevel::Q) => (2761, 44, 24, 30, 7, 25),
        (34, EcLevel::H) => (2761, 59, 16, 30, 1, 17),
        (35, EcLevel::L) => (2876, 12, 121, 30, 7, 122),
        (35, EcLevel::M) => (2876, 12, 47, 28, 26, 48),
        (35, EcLevel::Q) => (2876, 39, 24, 30, 14, 25),
        (35, EcLevel::H) => (2876, 22, 15, 30, 41, 16),
        (36, EcLevel::L) => (3034, 6, 121, 30, 14, 122),
        (36, EcLevel::M) => (3034, 6, 47, 28, 34, 48),
        (36, EcLevel::Q) => (3034, 46, 24, 30, 10, 25),
        (36, EcLevel::H) => (3034, 2, 15, 30, 64, 16),
        (37, EcLevel::L) => (3196, 17, 122, 30, 4, 123),
        (37, EcLevel::M) => (3196, 29, 46, 28, 14, 47),
        (37, EcLevel::Q) => (3196, 49, 24, 30, 10, 25),
        (37, EcLevel::H) => (3196, 24, 15, 30, 46, 16),
        (38, EcLevel::L) => (3362, 4, 122, 30, 18, 123),
        (38, EcLevel::M) => (3362, 13, 46, 28, 32, 47),
        (38, EcLevel::Q) => (3362, 48, 24, 30, 14, 25),
        (38, EcLevel::H) => (3362, 42, 15, 30, 32, 16),
        (39, EcLevel::L) => (3532, 20, 117, 30, 4, 118),
        (39, EcLevel::M) => (3532, 40, 47, 28, 7, 48),
        (39, EcLevel::Q) => (3532, 43, 24, 30, 22, 25),
        (39, EcLevel::H) => (3532, 10, 15, 30, 67, 16),
        (40, EcLevel::L) => (3706, 19, 118, 30, 6, 119),
        (40, EcLevel::M) => (3706, 18, 47, 28, 31, 48),
        (40, EcLevel::Q) => (3706, 34, 24, 30, 34, 25),
        (40, EcLevel::H) => (3706, 20, 15, 30, 61, 16),
        _ => return None,
    };
    Some(VersionInfo {
        version,
        total_codewords: tc,
        group1: EcBlock {
            count: g1c,
            data_codewords: g1d,
            ec_codewords: ec_pw,
        },
        group2: EcBlock {
            count: g2c,
            data_codewords: g2d,
            ec_codewords: ec_pw,
        },
    })
}

/// Find the smallest version that fits `data_len` bytes of payload in `mode` at `ec`.
///
/// Overhead is computed exactly per candidate version because the character-count
/// field width depends on the version group.
pub fn select_version(data_len: usize, mode: Mode, ec: EcLevel) -> Option<u8> {
    (1u8..=40).find(|&v| {
        version_info(v, ec).is_some_and(|info| {
            let used_bits = 4 + mode.char_count_bits(v) as usize + data_len * 8;
            info.data_codewords() * 8 >= used_bits
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_structure_sums_to_total() {
        for v in 1u8..=40 {
            for &ec in &[EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H] {
                let info = version_info(v, ec).unwrap_or_else(|| panic!("v{v}-{ec:?} missing"));
                let total = info.total_codewords as usize;
                let ec_total = (info.group1.count as usize + info.group2.count as usize)
                    * info.group1.ec_codewords as usize;
                assert_eq!(
                    info.data_codewords() + ec_total,
                    total,
                    "v{v}-{ec:?}: data+ec != total"
                );
                // Alignment positions must be inside the symbol
                let size = modules(v);
                for &p in alignment_positions(v) {
                    assert!(p < size as u8, "v{v} alignment {p} >= size {size}");
                }
            }
        }
    }

    #[test]
    fn module_sizes() {
        assert_eq!(modules(1), 21);
        assert_eq!(modules(2), 25);
        assert_eq!(modules(40), 177);
    }

    #[test]
    fn select_version_respects_char_count() {
        // 18 payload bytes in Byte mode fit exactly in v1-L (19 data codewords):
        // 4 + 8 + 18*8 = 156 > 152 bits → must NOT select v1.
        assert_eq!(select_version(18, Mode::Byte, EcLevel::L), Some(2));
        // 17 bytes: 4 + 8 + 136 = 148 <= 152 → v1 fits.
        assert_eq!(select_version(17, Mode::Byte, EcLevel::L), Some(1));
        // Alphanumeric counts use 9 bits at v1: 19 chars → 4+9+152 = 165 > 152 → v2.
        assert_eq!(select_version(19, Mode::Alphanumeric, EcLevel::L), Some(2));
    }
}
