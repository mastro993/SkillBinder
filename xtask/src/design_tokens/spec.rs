//! The token groups of `DESIGN.md`'s YAML front matter, in the order the file lists them.
use super::shadow::{self, Layer};
use anyhow::{Context, bail, ensure};
use yaml_rust2::{Yaml, YamlLoader, yaml::Hash};

/// A color role with its light value and its dark value: the `-dark` sibling, or the light
/// value again when there is none.
pub struct Color {
    pub name: String,
    pub light: u32,
    pub dark: u32,
}

/// A `typography` level.
pub struct Level {
    pub name: String,
    pub size: f32,
    pub weight: f32,
    pub line_height: f32,
    pub mono: bool,
}

/// The token groups the generator reads. `components` only refers to these.
pub struct Spec {
    pub colors: Vec<Color>,
    pub typography: Vec<Level>,
    pub rounded: Vec<(String, f32)>,
    pub spacing: Vec<(String, f32)>,
    /// Each level's shadow layers. The DESIGN.md format has no such group; its linter accepts
    /// one, and the Elevation & Depth prose describes it.
    pub elevation: Vec<(String, Vec<Layer>)>,
}

const DARK: &str = "-dark";

impl Spec {
    pub fn parse(design: &str) -> anyhow::Result<Self> {
        let front = design
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .map(|(front, _)| front)
            .context("DESIGN.md must start with YAML front matter between --- lines")?;
        let docs = YamlLoader::load_from_str(front).context("Invalid DESIGN.md front matter")?;
        let root = docs.first().context("Empty DESIGN.md front matter")?;
        Ok(Self {
            colors: colors(group(root, "colors")?)?,
            typography: entries(group(root, "typography")?)?
                .into_iter()
                .map(|(name, value)| level(name, value))
                .collect::<anyhow::Result<_>>()?,
            rounded: dimensions(group(root, "rounded")?)?,
            spacing: dimensions(group(root, "spacing")?)?,
            elevation: entries(group(root, "elevation")?)?
                .into_iter()
                .map(|(name, value)| {
                    let text = value.as_str().unwrap_or_default();
                    let layers =
                        shadow::parse(text).with_context(|| format!("elevation.{name}"))?;
                    Ok((name.to_owned(), layers))
                })
                .collect::<anyhow::Result<_>>()?,
        })
    }

    pub fn rounded(&self, name: &str) -> anyhow::Result<f32> {
        lookup(&self.rounded, name).with_context(|| format!("DESIGN.md has no rounded.{name}"))
    }
}

fn lookup(values: &[(String, f32)], name: &str) -> Option<f32> {
    values
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| *value)
}

fn group<'a>(root: &'a Yaml, key: &str) -> anyhow::Result<&'a Hash> {
    root[key]
        .as_hash()
        .with_context(|| format!("DESIGN.md front matter has no `{key}` map"))
}

fn entries(hash: &Hash) -> anyhow::Result<Vec<(&str, &Yaml)>> {
    hash.iter()
        .map(|(key, value)| Ok((key.as_str().context("Token names must be strings")?, value)))
        .collect()
}

fn colors(hash: &Hash) -> anyhow::Result<Vec<Color>> {
    let values: Vec<(&str, u32)> = entries(hash)?
        .into_iter()
        .map(|(name, value)| {
            let text = value.as_str().unwrap_or_default();
            Ok((name, hex(text).with_context(|| format!("colors.{name}"))?))
        })
        .collect::<anyhow::Result<_>>()?;
    for (name, _) in &values {
        if let Some(base) = name.strip_suffix(DARK) {
            ensure!(
                values.iter().any(|(other, _)| *other == base),
                "colors.{name} has no light sibling colors.{base}"
            );
        }
    }
    Ok(values
        .iter()
        .filter(|(name, _)| !name.ends_with(DARK))
        .map(|&(name, light)| {
            let dark = values
                .iter()
                .find(|(other, _)| other.strip_suffix(DARK) == Some(name))
                .map_or(light, |(_, dark)| *dark);
            Color {
                name: name.to_owned(),
                light,
                dark,
            }
        })
        .collect())
}

fn hex(text: &str) -> anyhow::Result<u32> {
    let digits = text
        .strip_prefix('#')
        .filter(|digits| digits.len() == 6)
        .with_context(|| format!("`{text}` is not a #RRGGBB color"))?;
    u32::from_str_radix(digits, 16).with_context(|| format!("`{text}` is not a #RRGGBB color"))
}

fn level(name: &str, value: &Yaml) -> anyhow::Result<Level> {
    let field = |key: &str| {
        let field = &value[key];
        (!field.is_badvalue())
            .then_some(field)
            .with_context(|| format!("typography.{name} has no {key}"))
    };
    let mono = match field("fontFamily")?.as_str() {
        Some("SF Pro") => false,
        Some("SF Mono") => true,
        other => bail!("typography.{name}.fontFamily {other:?} is neither SF Pro nor SF Mono"),
    };
    Ok(Level {
        name: name.to_owned(),
        size: dimension(field("fontSize")?).with_context(|| format!("typography.{name}"))?,
        weight: number(field("fontWeight")?).with_context(|| format!("typography.{name}"))?,
        line_height: number(field("lineHeight")?).with_context(|| format!("typography.{name}"))?,
        mono,
    })
}

fn dimensions(hash: &Hash) -> anyhow::Result<Vec<(String, f32)>> {
    entries(hash)?
        .into_iter()
        .map(|(name, value)| Ok((name.to_owned(), dimension(value).context(name.to_owned())?)))
        .collect()
}

/// A `px` dimension such as `12px`.
fn dimension(value: &Yaml) -> anyhow::Result<f32> {
    let text = value.as_str().unwrap_or_default();
    text.strip_suffix("px")
        .and_then(|number| number.parse().ok())
        .with_context(|| format!("`{text}` is not a px dimension"))
}

/// A unitless number, written as an integer or a real.
fn number(value: &Yaml) -> anyhow::Result<f32> {
    match value {
        Yaml::Integer(number) => Ok(*number as f32),
        Yaml::Real(number) => number.parse().context("Invalid number"),
        other => bail!("{other:?} is not a number"),
    }
}
