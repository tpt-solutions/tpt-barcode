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
}
