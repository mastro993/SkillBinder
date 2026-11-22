//! The icon's artwork in `assets/tray`: SkillBinder's mark as a one-color glyph, with a dot
//! variant for when something waits for the user (spec rule 14, D6).

use gpui_kit::WindowAppearance;

macro_rules! artwork {
    ($file:literal) => {
        include_bytes!(concat!("../../../../assets/tray/", $file, ".png"))
    };
}

/// macOS recolors this template for the menu bar.
const TEMPLATE: &[u8] = artwork!("tray-template");
const TEMPLATE_DOT: &[u8] = artwork!("tray-template-dot");
/// A black glyph for light panels, smallest first.
const BLACK: [&[u8]; 4] = [
    artwork!("tray-black-16"),
    artwork!("tray-black-20"),
    artwork!("tray-black-24"),
    artwork!("tray-black-32"),
];
const BLACK_DOT: [&[u8]; 4] = [
    artwork!("tray-black-dot-16"),
    artwork!("tray-black-dot-20"),
    artwork!("tray-black-dot-24"),
    artwork!("tray-black-dot-32"),
];
/// A white glyph for dark panels, smallest first.
const WHITE: [&[u8]; 4] = [
    artwork!("tray-white-16"),
    artwork!("tray-white-20"),
    artwork!("tray-white-24"),
    artwork!("tray-white-32"),
];
const WHITE_DOT: [&[u8]; 4] = [
    artwork!("tray-white-dot-16"),
    artwork!("tray-white-dot-20"),
    artwork!("tray-white-dot-24"),
    artwork!("tray-white-dot-32"),
];

/// One square image, as straight RGBA rows.
pub(super) struct Artwork {
    pub(super) rgba: Vec<u8>,
    pub(super) size: u32,
}

/// The images this platform shows, with the dot or without: the template on macOS, the 32 px
/// glyph on Windows, which scales it to the notification area, and every size on Linux, whose
/// host picks one. The glyph follows the system appearance, the nearest the app knows of the
/// panel's own.
pub(super) fn artwork(
    appearance: WindowAppearance,
    dot: bool,
) -> Result<Vec<Artwork>, png::DecodingError> {
    let dark = matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    );
    let glyph = match (dark, dot) {
        (true, false) => WHITE,
        (true, true) => WHITE_DOT,
        (false, false) => BLACK,
        (false, true) => BLACK_DOT,
    };
    let files: &[&[u8]] = if cfg!(target_os = "macos") {
        if dot { &[TEMPLATE_DOT] } else { &[TEMPLATE] }
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
    use super::{BLACK, BLACK_DOT, TEMPLATE, TEMPLATE_DOT, WHITE, WHITE_DOT, artwork, decode};
    use gpui_kit::WindowAppearance;

    #[test]
    fn every_file_is_a_square_rgba_image_of_its_size() -> Result<(), png::DecodingError> {
        let sized = [BLACK, WHITE, BLACK_DOT, WHITE_DOT]
            .iter()
            .flatten()
            .zip([16, 20, 24, 32].repeat(4))
            .chain([(&TEMPLATE, 44), (&TEMPLATE_DOT, 44)]);
        for (file, size) in sized {
            let image = decode(file)?;
            assert_eq!(image.size, size);
            assert_eq!(image.rgba.len(), (size * size * 4) as usize);
        }
        Ok(())
    }

    #[test]
    fn the_glyph_contrasts_with_the_appearance() -> Result<(), png::DecodingError> {
        for dot in [false, true] {
            glyph_contrasts(dot)?;
        }
        Ok(())
    }

    #[test]
    fn the_dot_variant_differs_only_by_its_dot() -> Result<(), png::DecodingError> {
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            let plain = artwork(appearance, false)?;
            let dot = artwork(appearance, true)?;
            for (plain, dot) in plain.iter().zip(&dot) {
                assert_eq!(plain.size, dot.size);
                assert_ne!(plain.rgba, dot.rgba);
                // The dot sits in the top right quarter; the rune's lower half is untouched.
                let half = plain.rgba.len() / 2;
                assert_eq!(plain.rgba[half..], dot.rgba[half..]);
            }
        }
        Ok(())
    }

    fn glyph_contrasts(dot: bool) -> Result<(), png::DecodingError> {
        let light = artwork(WindowAppearance::Light, dot)?;
        let dark = artwork(WindowAppearance::Dark, dot)?;
        let expected = if cfg!(target_os = "linux") { 4 } else { 1 };
        assert_eq!((light.len(), dark.len()), (expected, expected));
        // Only the stroke is opaque, the halo never is, so every opaque pixel is the glyph's color.
        let stroke = |image: &super::Artwork| {
            let opaque: Vec<_> = image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[3] == 255)
                .map(|pixel| pixel[..3].to_vec())
                .collect();
            assert!(!opaque.is_empty());
            opaque
                .iter()
                .all(|color| *color == opaque[0])
                .then(|| opaque[0].clone())
        };
        if cfg!(target_os = "macos") {
            assert_eq!(stroke(&light[0]), stroke(&dark[0]));
        } else {
            for image in &light {
                assert_eq!(stroke(image), Some(vec![0, 0, 0]));
            }
            for image in &dark {
                assert_eq!(stroke(image), Some(vec![255, 255, 255]));
            }
        }
        Ok(())
    }
}
