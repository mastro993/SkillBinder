//! Workspace image icons: reading a picked file and recognizing the raster formats the UI decodes.

use skillbinder_proto::{AppError, AppResult};
use std::{fs, path::Path};

/// Largest image accepted as an icon.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Reads a picked image and returns its bytes with the canonical extension of its format.
///
/// SVG is refused because it can reference resources outside the stored copy; only raster
/// formats are copied into application storage.
pub(super) fn read(path: &Path) -> AppResult<(Vec<u8>, &'static str)> {
    let metadata =
        fs::metadata(path).map_err(|_| AppError::validation("Choose an accessible image file."))?;
    if !metadata.is_file() {
        return Err(AppError::validation("Choose an image file."));
    }
    if metadata.len() > MAX_BYTES {
        return Err(AppError::validation("The image must be 8 MB or smaller."));
    }
    let bytes =
        fs::read(path).map_err(|_| AppError::validation("Choose an accessible image file."))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(AppError::validation("The image must be 8 MB or smaller."));
    }
    let extension = extension(&bytes)
        .ok_or_else(|| AppError::validation("Choose a PNG, JPEG, GIF, WebP, BMP, or ICO image."))?;
    Ok((bytes, extension))
}

/// The canonical extension of a recognized raster image, or `None` for anything else.
fn extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"BM") {
        Some("bmp")
    } else if bytes.starts_with(&[0, 0, 1, 0]) {
        Some("ico")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::extension;

    #[test]
    fn recognized_formats_map_to_canonical_extensions() {
        for (bytes, expected) in [
            (b"\x89PNG\r\n\x1a\nrest".as_slice(), Some("png")),
            (b"\xff\xd8\xff\xe0".as_slice(), Some("jpg")),
            (b"GIF89a".as_slice(), Some("gif")),
            (b"RIFF\x00\x00\x00\x00WEBP".as_slice(), Some("webp")),
            (b"BM\x00\x00".as_slice(), Some("bmp")),
            (b"\x00\x00\x01\x00".as_slice(), Some("ico")),
            (
                b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".as_slice(),
                None,
            ),
            (b"".as_slice(), None),
        ] {
            assert_eq!(extension(bytes), expected, "{bytes:?}");
        }
    }
}
