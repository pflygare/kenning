//! Label colors for tags and category values: a name from the palette, or a
//! custom `#rrggbb`. New labels get a color their neighbors don't use yet.

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

/// The palette, in the order new labels take colors: each next to ones that
/// look unlike it. Gray, the old default, comes last.
pub const PALETTE: [&str; 16] = [
    "blue", "green", "amber", "red", "purple", "teal", "pink", "orange", "indigo", "lime", "cyan",
    "fuchsia", "yellow", "sky", "brown", "gray",
];

/// A palette name ("teal") or a custom color ("#1e90ff", stored in lowercase).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, sqlx::Type, TS)]
#[sqlx(transparent)]
#[ts(export, rename = "TagColor", type = "string")]
pub struct Color(String);

impl Color {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Color {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.trim().to_ascii_lowercase();
        let custom = value.len() == 7
            && value.starts_with('#')
            && value[1..].bytes().all(|b| b.is_ascii_hexdigit());
        if custom || PALETTE.contains(&value.as_str()) {
            Ok(Color(value))
        } else {
            Err(format!(
                "{value:?} is not a palette color or a #rrggbb color"
            ))
        }
    }
}

/// Requests are checked here, so a handler only ever sees a valid color.
impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Color::try_from(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl From<Color> for String {
    fn from(color: Color) -> String {
        color.0
    }
}

/// The first palette color none of `used` has; once all are taken, the one
/// used least (earliest in the palette on a tie).
pub fn unused(used: &[Color]) -> Color {
    let count = |name: &str| used.iter().filter(|c| c.as_str() == name).count();
    let name = PALETTE
        .iter()
        .min_by_key(|name| count(name))
        .expect("palette is not empty");
    Color((*name).to_string())
}

/// Colors for `n` new labels next to `used`, each distinct from those before it.
pub fn unused_n(used: &[Color], n: usize) -> Vec<Color> {
    let mut used = used.to_vec();
    (0..n)
        .map(|_| {
            let color = unused(&used);
            used.push(color.clone());
            color
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Color {
        Color::try_from(s.to_string()).unwrap()
    }

    #[test]
    fn picks_the_first_unused_then_the_least_used() {
        assert_eq!(unused(&[]).as_str(), "blue");
        assert_eq!(unused(&[c("blue"), c("amber")]).as_str(), "green");
        let all: Vec<Color> = PALETTE.iter().map(|n| c(n)).collect();
        let mut more = all.clone();
        more.push(c("blue"));
        assert_eq!(unused(&more).as_str(), "green");
        assert_eq!(
            unused_n(&[], 3)
                .iter()
                .map(Color::as_str)
                .collect::<Vec<_>>(),
            ["blue", "green", "amber"]
        );
    }

    #[test]
    fn accepts_palette_names_and_hex() {
        assert_eq!(c("#1E90FF").as_str(), "#1e90ff");
        assert!(Color::try_from("#12345".to_string()).is_err());
        assert!(Color::try_from("chartreuse".to_string()).is_err());
    }
}
