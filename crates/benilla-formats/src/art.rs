//! The PNG art the sheet names (`data/twilights-eve/art/`), embedded in the binary the way
//! `client_dbc_patch.json` is (ADR 0011). A spell's custom icon is the path
//! `Interface\Icons\Twilight\<name>`; the asset reader serves it from here before it asks the MPQs.

use std::collections::HashMap;
use std::sync::OnceLock;

/// `{name: base64 PNG}`, written by `build_spells.py` from the PNGs the master CSV's `icon` cells name.
const CLIENT_ART: &str = include_str!("../../../data/twilights-eve/client_art.json");

/// The folder, lowercased with `/`, embedded art lives under: a spell's or an item's icon.
const ART_DIRS: [&str; 1] = ["interface/icons/twilight/"];

/// The PNG bytes behind an art path (any case, `/` or `\`, with or without `.blp`), or `None`
/// when the path is not embedded art.
pub fn embedded_icon_png(path: &str) -> Option<Vec<u8>> {
    embedded(path).cloned()
}

/// Whether [`embedded_icon_png`] answers for `path`, without copying the bytes: the UI's texture
/// probe asks this on every `SetTexture`.
pub fn has_embedded_icon(path: &str) -> bool {
    embedded(path).is_some()
}

fn embedded(path: &str) -> Option<&'static Vec<u8>> {
    static ART: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();
    let art = ART.get_or_init(|| parse_art(CLIENT_ART));
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    let name = ART_DIRS.iter().find_map(|dir| lower.strip_prefix(dir))?;
    let name = name.strip_suffix(".blp").unwrap_or(name);
    art.get(name)
}

fn parse_art(json: &str) -> HashMap<String, Vec<u8>> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str(json) else {
        return HashMap::new();
    };
    map.iter()
        .filter_map(|(name, b64)| Some((name.to_ascii_lowercase(), decode_base64(b64.as_str()?)?)))
        .collect()
}

/// Standard-alphabet base64, padding optional; `None` on a byte outside the alphabet.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for b in text.bytes().filter(|b| *b != b'=') {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Whether `bytes` start with the PNG signature.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
}

/// Decode a PNG to RGBA8: `(width, height, pixels)`.
pub fn png_to_rgba(bytes: &[u8]) -> anyhow::Result<(u32, u32, Vec<u8>)> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| anyhow::anyhow!("decoding PNG: {e}"))?
        .to_rgba8();
    Ok((image.width(), image.height(), image.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decodes_with_and_without_padding() {
        assert_eq!(decode_base64("TWFu").unwrap(), b"Man");
        assert_eq!(decode_base64("TWE=").unwrap(), b"Ma");
        assert_eq!(decode_base64("TQ").unwrap(), b"M");
        assert!(decode_base64("T*").is_none());
    }

    #[test]
    fn an_icon_path_finds_its_png_in_any_spelling() {
        let art = parse_art(r#"{"Ember_Lash": "iVBORw0KGgo="}"#);
        assert!(is_png(&art["ember_lash"]));
        assert!(embedded_icon_png("Interface\\Icons\\Twilight\\nothing").is_none());
        assert!(embedded_icon_png("Interface\\Icons\\Spell_Fire_Fireball").is_none());
    }

    /// A painted item icon (#406) as the sprite loader asks for it, with `.blp`, decodes.
    #[test]
    fn a_painted_item_icon_decodes_from_the_loaders_path() {
        let path = "interface/icons/twilight/wriggling_worms.blp";
        assert!(has_embedded_icon(path));
        let png = embedded_icon_png(path).expect("the worms icon is embedded");
        let (w, h, rgba) = crate::png_to_rgba(&png).expect("it decodes");
        assert_eq!((w, h), (128, 128));
        assert_eq!(rgba.len(), 128 * 128 * 4);
    }
}
