//! `elevation` levels: CSS-like shadow lists such as `0 1px 2px rgba(0,0,0,0.08)`.
use anyhow::{Context, bail, ensure};

/// What a layer is drawn in.
pub enum Ink {
    /// Black at this alpha.
    Black(f32),
    /// The color token `name` at this alpha.
    Color { name: String, alpha: f32 },
}

/// One shadow layer, in points.
pub struct Layer {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub ink: Ink,
}

/// The layers of one level: `none`, or comma-separated `x y blur spread color`.
pub fn parse(text: &str) -> anyhow::Result<Vec<Layer>> {
    if text.trim() == "none" {
        return Ok(Vec::new());
    }
    split_layers(text)
        .into_iter()
        .map(|layer| self::layer(layer).with_context(|| format!("layer `{layer}`")))
        .collect()
}

/// Splits at the commas outside parentheses, so `rgba(0,0,0,0.1)` stays whole.
fn split_layers(text: &str) -> Vec<&str> {
    let (mut layers, mut depth, mut start) = (Vec::new(), 0_u32, 0);
    for (index, character) in text.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                layers.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    layers.push(text[start..].trim());
    layers
}

fn layer(text: &str) -> anyhow::Result<Layer> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let [x, y, blur, spread, ink] = parts[..] else {
        bail!("a layer is `x y blur spread color`");
    };
    Ok(Layer {
        x: length(x)?,
        y: length(y)?,
        blur: length(blur)?,
        spread: length(spread)?,
        ink: self::ink(ink)?,
    })
}

/// `0`, or a px length such as `-6px` or `0.5px`.
fn length(text: &str) -> anyhow::Result<f32> {
    if text == "0" {
        return Ok(0.);
    }
    text.strip_suffix("px")
        .and_then(|number| number.parse().ok())
        .with_context(|| format!("`{text}` is not 0 or a px length"))
}

/// `rgba(0,0,0,a)`, or `{colors.name}` with an optional `/N%` alpha.
fn ink(text: &str) -> anyhow::Result<Ink> {
    if let Some(channels) = text
        .strip_prefix("rgba(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let channels: Vec<&str> = channels.split(',').map(str::trim).collect();
        let ["0", "0", "0", alpha] = channels[..] else {
            bail!("`{text}` must be black, `rgba(0,0,0,a)`");
        };
        let alpha = alpha
            .parse()
            .with_context(|| format!("`{alpha}` is not a number"))?;
        return Ok(Ink::Black(unit(alpha)?));
    }
    let (token, alpha) = match text.split_once('/') {
        Some((token, percent)) => {
            let percent: f32 = percent
                .strip_suffix('%')
                .and_then(|number| number.parse().ok())
                .with_context(|| format!("`{percent}` is not a percentage"))?;
            (token, unit(percent / 100.)?)
        }
        None => (text, 1.),
    };
    let name = token
        .strip_prefix("{colors.")
        .and_then(|rest| rest.strip_suffix('}'))
        .with_context(|| format!("`{text}` is neither rgba(0,0,0,a) nor {{colors.name}}"))?;
    Ok(Ink::Color {
        name: name.to_owned(),
        alpha,
    })
}

/// An alpha, from 0 to 1.
fn unit(value: f32) -> anyhow::Result<f32> {
    ensure!((0. ..=1.).contains(&value), "alpha {value} is not 0 to 1");
    Ok(value)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{Ink, parse};

    #[test]
    fn layers_keep_their_order_and_inks() {
        let layers = parse("0 0 0 1px {colors.foreground}/10%, 0 6px 16px -6px rgba(0,0,0,0.08)")
            .expect("parses");
        assert_eq!(layers.len(), 2);
        assert!(
            matches!(&layers[0].ink, Ink::Color { name, alpha } if name == "foreground" && (*alpha - 0.1).abs() < 1e-6)
        );
        assert_eq!(
            (layers[1].y, layers[1].blur, layers[1].spread),
            (6., 16., -6.)
        );
        assert!(matches!(layers[1].ink, Ink::Black(alpha) if (alpha - 0.08).abs() < 1e-6));
    }

    #[test]
    fn none_has_no_layers() {
        assert!(parse("none").expect("parses").is_empty());
    }

    #[test]
    fn only_black_and_tokens_are_inks() {
        for broken in [
            "0 1px 2px rgba(255,0,0,0.1)",
            "0 1px 2px #000",
            "0 1px rgba(0,0,0,1)",
        ] {
            assert!(parse(broken).is_err(), "{broken}");
        }
    }
}
