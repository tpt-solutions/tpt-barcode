//! Shared error types, format identifiers, and symbology traits.

/// Format identifier for a decoded or encoded barcode symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// QR Code (ISO/IEC 18004).
    QrCode,
    /// Data Matrix (ISO/IEC 16022).
    DataMatrix,
    /// PDF417 (ISO/IEC 15438).
    Pdf417,
    /// Code 128 (ISO/IEC 15417).
    Code128,
    /// EAN-13 (ISO/IEC 15420).
    Ean13,
    /// UPC-A (subset of EAN-13).
    UpcA,
    /// Code 39 (ISO/IEC 16388).
    Code39,
}

impl core::fmt::Display for Format {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Format::QrCode => "QR Code",
            Format::DataMatrix => "Data Matrix",
            Format::Pdf417 => "PDF417",
            Format::Code128 => "Code 128",
            Format::Ean13 => "EAN-13",
            Format::UpcA => "UPC-A",
            Format::Code39 => "Code 39",
        };
        f.write_str(name)
    }
}

/// Error-correction level (used by QR Code and PDF417).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EcLevel {
    /// ~7% recovery capacity.
    L,
    /// ~15% recovery capacity.
    M,
    /// ~25% recovery capacity.
    Q,
    /// ~30% recovery capacity.
    H,
}

impl EcLevel {
    /// The 2-bit format-information encoding: L=01, M=00, Q=11, H=10.
    pub fn bits(self) -> u8 {
        match self {
            EcLevel::L => 0b01,
            EcLevel::M => 0b00,
            EcLevel::Q => 0b11,
            EcLevel::H => 0b10,
        }
    }
}

impl core::fmt::Display for EcLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            EcLevel::L => "L",
            EcLevel::M => "M",
            EcLevel::Q => "Q",
            EcLevel::H => "H",
        })
    }
}

/// Errors that can occur during barcode encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// The input data is too long for the selected version / capacity.
    DataTooLong,
    /// The input contains a character not supported by the selected mode/symbology.
    InvalidCharacter,
    /// The requested version or format is not supported.
    Unsupported,
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EncodeError::DataTooLong => {
                f.write_str("data too long for the selected symbology/capacity")
            }
            EncodeError::InvalidCharacter => {
                f.write_str("payload contains a character unsupported by the selected mode")
            }
            EncodeError::Unsupported => {
                f.write_str("the requested symbology, mode, or option is not implemented")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

/// Errors that can occur during barcode decoding / scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// More errors than the code's error-correction capacity.
    TooManyErrors,
    /// Structural format information is invalid.
    InvalidFormat,
    /// No barcode of the requested type was found in the image.
    NotFound,
    /// Image preprocessing failure.
    ImageError,
    /// The symbol uses a feature that is not implemented (e.g. an
    /// encodation mode this decoder does not support).
    Unsupported,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::TooManyErrors => {
                f.write_str("more errors than the error-correction capacity can correct")
            }
            DecodeError::InvalidFormat => f.write_str("malformed or unrecognized symbol structure"),
            DecodeError::NotFound => f.write_str("no barcode found in the image"),
            DecodeError::ImageError => f.write_str("image preprocessing failed"),
            DecodeError::Unsupported => {
                f.write_str("the symbol uses an unimplemented feature or encodation mode")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}
