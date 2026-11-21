//! The icon's artwork in `assets/tray`: SkillBinder's mark as a one-color glyph (spec rule 14).

use gpui_kit::WindowAppearance;

macro_rules! artwork {
    ($file:literal) => {
        include_bytes!(concat!("../../../../assets/tray/", $file, ".png"))
    };
}

/// macOS recolors this template for the menu bar.
const TEMPLATE: &[u8] = artwork!("tray-template");
/// A black glyph for light panels, smallest first.
const BLACK: [&[u8]; 4] = [
    artwork!("tray-black-16"),
    artwork!("tray-black-20"),
    artwork!("tray-black-24"),
    artwork!("tray-black-32"),
];
/// A white glyph for dark panels, smallest first.
const WHITE: [&[u8]; 4] = [
    artwork!("tray-white-16"),
    artwork!("tray-white-20"),
    artwork!("tray-white-24"),
    artwork!("tray-white-32"),
];

/// One square image, as straight RGBA rows.
pub(super) struct Artwork {
    pub(super) rgba: Vec<u8>,
    pub(super) size: u32,
}

/// The images this platform shows: the template on macOS, the 32 px glyph on Windows, which
/// scales it to the notification area, and every size on Linux, whose host picks one. The glyph
/// follows the system appearance, the nearest the app knows of the panel's own.
pub(super) fn artwork(appearance: WindowAppearance) -> Result<Vec<Artwork>, png::DecodingError> {
    let glyph = match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => WHITE,
        WindowAppearance::Light | WindowAppearance::VibrantLight => BLACK,
    };
    let files: &[&[u8]] = if cfg!(target_os = "macos") {
        &[TEMPLATE]
    } else if cfg!(windows) {
        &glyph[3..]
    } else {
        &glyph
    };
    files.iter().map(|file| decode(file)).collect()
}

fn decode(file: &[u8]) -> Result<Artwork, png::DecodingError> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let frame = reader.next_frame(&mut rgba)?;
    rgba.truncate(frame.buffer_size());
    Ok(Artwork {
        rgba,
        size: frame.width,
    })
}

#[cfg(test)]
mod tests {
    use super::{BLACK, TEMPLATE, WHITE, artwork, decode};
    use gpui_kit::WindowAppearance;

    #[test]
    fn every_file_is_a_square_rgba_image_of_its_size() -> Result<(), png::DecodingError> {
        let sized = BLACK
            .iter()
            .chain(&WHITE)
            .zip([16, 20, 24, 32, 16, 20, 24, 32])
            .chain([(&TEMPLATE, 44)]);
        for (file, size) in sized {
            let image = decode(file)?;
            assert_eq!(image.size, size);
            assert_eq!(image.rgba.len(), (size * size * 4) as usize);
        }
        Ok(())
    }

    #[test]
    fn the_glyph_contrasts_with_the_appearance() -> Result<(), png::DecodingError> {
        let light = artwork(WindowAppearance::Light)?;
        let dark = artwork(WindowAppearance::Dark)?;
        let expected = if cfg!(target_os = "linux") { 4 } else { 1 };
        assert_eq!((light.len(), dark.len()), (expected, expected));
        // The center of the page is the glyph's own color.
        let center = |image: &super::Artwork| {
            let at = ((image.size / 2) * image.size + image.size / 2) as usize * 4;
            image.rgba[at..at + 4].to_vec()
        };
        if cfg!(target_os = "macos") {
            assert_eq!(center(&light[0]), center(&dark[0]));
        } else {
            assert_eq!(center(&light[0]), [0, 0, 0, 255]);
            assert_eq!(center(&dark[0]), [255, 255, 255, 255]);
        }
        Ok(())
    }
}
