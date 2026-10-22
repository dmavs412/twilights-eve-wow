use std::collections::BTreeMap;

use serde::Deserialize;

use crate::settings::Channel;
use crate::{Error, Result};

pub const REPO: &str = "dmavs412/twilights-eve-wow";
const USER_AGENT: &str = "twilights-launcher";
const ACCEPT: &str = "application/vnd.github+json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub notes: String,
    /// Asset name to `browser_download_url`.
    pub assets: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Deserialize)]
struct RawAsset {
    name: String,
    browser_download_url: String,
}

impl From<RawRelease> for Release {
    fn from(raw: RawRelease) -> Release {
        Release {
            tag: raw.tag_name,
            notes: raw.body.unwrap_or_default(),
            assets: raw
                .assets
                .into_iter()
                .map(|a| (a.name, a.browser_download_url))
                .collect(),
        }
    }
}

/// Live answers with one release object.
pub fn parse_latest(json: &str) -> Result<Release> {
    let raw: RawRelease = serde_json::from_str(json).map_err(|e| Error::Parse(e.to_string()))?;
    Ok(raw.into())
}

/// Testing answers with a list; the first element is the newest, pre-release or not.
pub fn parse_list(json: &str) -> Result<Release> {
    let raw: Vec<RawRelease> =
        serde_json::from_str(json).map_err(|e| Error::Parse(e.to_string()))?;
    raw.into_iter()
        .next()
        .map(Release::from)
        .ok_or_else(|| Error::Parse("the release list is empty".into()))
}

pub fn channel_url(channel: Channel) -> String {
    match channel {
        Channel::Live => format!("https://api.github.com/repos/{REPO}/releases/latest"),
        Channel::Testing => format!("https://api.github.com/repos/{REPO}/releases?per_page=1"),
    }
}

fn get(url: &str) -> Result<ureq::http::Response<ureq::Body>> {
    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", ACCEPT)
        .call()
        .map_err(|e| Error::Network(e.to_string()))
}

pub fn fetch_release(channel: Channel) -> Result<Release> {
    let mut resp = get(&channel_url(channel))?;
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::Network(e.to_string()))?;
    match channel {
        Channel::Live => parse_latest(&text),
        Channel::Testing => parse_list(&text),
    }
}

/// Opens a download. Returns the Content-Length when the server sent one, and the body reader.
pub fn open_download(url: &str) -> Result<(Option<u64>, Box<dyn std::io::Read + Send>)> {
    let resp = get(url)?;
    let total = resp.body().content_length();
    Ok((total, Box::new(resp.into_body().into_reader())))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"{
        "tag_name": "b-2db0441",
        "body": "Fixed the bank.",
        "prerelease": false,
        "assets": [
            {"name": "manifest.json", "browser_download_url": "https://x/manifest.json", "size": 1},
            {"name": "twilights-eve.exe", "browser_download_url": "https://x/twilights-eve.exe"}
        ]
    }"#;

    #[test]
    fn live_object() {
        let r = parse_latest(ONE).unwrap();
        assert_eq!(r.tag, "b-2db0441");
        assert_eq!(r.notes, "Fixed the bank.");
        assert_eq!(r.assets["manifest.json"], "https://x/manifest.json");
        assert_eq!(r.assets.len(), 2);
    }

    #[test]
    fn testing_list_takes_first_even_if_prerelease() {
        let json = format!(
            r#"[{{"tag_name": "t-1", "body": null, "prerelease": true, "assets": []}}, {ONE}]"#
        );
        let r = parse_list(&json).unwrap();
        assert_eq!(r.tag, "t-1");
        assert_eq!(r.notes, "");
        assert!(r.assets.is_empty());
    }

    #[test]
    fn empty_list_is_an_error() {
        assert!(parse_list("[]").is_err());
    }

    #[test]
    fn urls() {
        assert!(channel_url(Channel::Live).ends_with("/releases/latest"));
        assert!(channel_url(Channel::Testing).ends_with("/releases?per_page=1"));
    }
}
