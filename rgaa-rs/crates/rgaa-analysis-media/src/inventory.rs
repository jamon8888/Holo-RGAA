//! DOM inventory of media, tracks and third-party players (spec §3.1).
//!
//! `DomSnapshot::json` is a serialised document: an object with a `nodes`
//! array in document order. Each node has a `tag`, an `attrs` map and optional
//! `text`. This is a small, stable stand-in for a CDP `DOMSnapshot`; plugins
//! receive what the orchestrator already captured and never fetch a third-party
//! player (R-P3).

use rgaa_analysis::{analysis_error, AxTree, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One `video` or `audio` element found in the DOM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MediaItem {
    pub tag: String,
    pub src: Option<String>,
    pub autoplay: bool,
    pub muted: bool,
    pub controls: bool,
    pub tracks: Vec<TrackItem>,
    pub transcript_url: Option<String>,
}

/// A `track` child of a media element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrackItem {
    pub kind: String,
    pub src: String,
    pub srclang: Option<String>,
}

/// An iframe pointing at a known third-party player. Never downloaded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ThirdParty {
    pub url: String,
    pub plateforme: String,
}

/// All media-related nodes of one page, in document order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inventory {
    pub media: Vec<MediaItem>,
    pub third_party: Vec<ThirdParty>,
}

#[derive(Debug, Deserialize)]
struct DomJson {
    #[serde(default)]
    nodes: Vec<NodeJson>,
}

#[derive(Debug, Deserialize)]
struct NodeJson {
    tag: String,
    #[serde(default)]
    attrs: BTreeMap<String, String>,
    #[serde(default)]
    text: String,
}

/// Parses the DOM snapshot and the accessibility tree into an [`Inventory`].
///
/// # Errors
/// Returns [`rgaa_analysis::RgaaError::Analysis`] when the snapshot is not the
/// documented JSON shape.
pub fn parse_dom(json: &str, ax_tree: Option<&AxTree>) -> Result<Inventory> {
    let dom: DomJson = serde_json::from_str(json)
        .map_err(|error| analysis_error("media", format!("DOM snapshot illisible : {error}")))?;
    let ax_controls = has_accessible_controls(ax_tree);
    // "À moins de 2 blocs" counts text blocks, not <track>/<source> children.
    let mut block_index = 0usize;
    let blocks: Vec<Option<usize>> = dom
        .nodes
        .iter()
        .map(|node| {
            if is_block(&node.tag) {
                let index = block_index;
                block_index += 1;
                Some(index)
            } else {
                None
            }
        })
        .collect();
    let mut inventory = Inventory::default();

    for (index, node) in dom.nodes.iter().enumerate() {
        let tag = node.tag.to_ascii_lowercase();
        match tag.as_str() {
            "video" | "audio" => {
                let src = node.attrs.get("src").cloned().or_else(|| {
                    dom.nodes
                        .get(index + 1)
                        .filter(|next| next.tag.eq_ignore_ascii_case("source"))
                        .and_then(|next| next.attrs.get("src").cloned())
                });
                let tracks = dom
                    .nodes
                    .iter()
                    .skip(index + 1)
                    .take_while(|next| next.tag.eq_ignore_ascii_case("track"))
                    .filter_map(|next| {
                        let kind = next.attrs.get("kind")?.clone();
                        let src = next.attrs.get("src")?.clone();
                        Some(TrackItem {
                            kind,
                            src,
                            srclang: next.attrs.get("srclang").cloned(),
                        })
                    })
                    .collect();
                let transcript_url = dom
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(other, candidate)| {
                        (match (blocks[*other], blocks[index]) {
                            (Some(link_block), Some(media_block)) => {
                                link_block.abs_diff(media_block) <= 2
                            }
                            _ => false,
                        }) && candidate.tag.eq_ignore_ascii_case("a")
                            && is_transcript_label(&candidate.text)
                    })
                    .filter_map(|(_, candidate)| candidate.attrs.get("href").cloned())
                    .next();
                inventory.media.push(MediaItem {
                    tag,
                    src,
                    autoplay: node.attrs.contains_key("autoplay"),
                    muted: node.attrs.contains_key("muted"),
                    controls: node.attrs.contains_key("controls") || ax_controls,
                    tracks,
                    transcript_url,
                });
            }
            "iframe" => {
                if let Some((url, plateforme)) = node
                    .attrs
                    .get("src")
                    .and_then(|src| third_party_platform(src).map(|platform| (src, platform)))
                {
                    inventory.third_party.push(ThirdParty {
                        url: url.clone(),
                        plateforme,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(inventory)
}

fn is_block(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        "video"
            | "audio"
            | "p"
            | "a"
            | "div"
            | "span"
            | "li"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "figcaption"
            | "blockquote"
            | "td"
            | "th"
    )
}

fn is_transcript_label(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("transcription") || lower.contains("version texte")
}

fn third_party_platform(url: &str) -> Option<String> {
    let lower = url.to_ascii_lowercase();
    for platform in [
        "youtube-nocookie",
        "youtube",
        "youtu.be",
        "vimeo",
        "dailymotion",
        "peertube",
    ] {
        if lower.contains(platform) {
            return Some(platform.to_owned());
        }
    }
    None
}

fn has_accessible_controls(ax_tree: Option<&AxTree>) -> bool {
    let Some(tree) = ax_tree else {
        return false;
    };
    tree.nodes.iter().any(|node| {
        let role = node.role.to_ascii_lowercase();
        let name = node.name.to_ascii_lowercase();
        let is_control = matches!(role.as_str(), "button" | "slider" | "switch" | "checkbox");
        is_control
            && name.len() > 1
            && [
                "play", "pause", "lecture", "volume", "son", "mute", "player", "lire",
            ]
            .iter()
            .any(|word| name.contains(word))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_analysis::AxNode;

    const DOM: &str = r#"{"nodes":[
        {"tag":"video","attrs":{"src":"/media/v.mp4","autoplay":"","muted":""},"text":""},
        {"tag":"track","attrs":{"kind":"captions","srclang":"fr","src":"/media/fr.vtt"}},
        {"tag":"track","attrs":{"kind":"descriptions","src":"/media/ad.vtt"}},
        {"tag":"p","text":"une légende"},
        {"tag":"a","attrs":{"href":"/transcript.html"},"text":"Transcription"},
        {"tag":"iframe","attrs":{"src":"https://www.youtube.com/embed/abc"}}
    ]}"#;

    #[test]
    fn inventory_lists_media_tracks_and_autoplay_attributes() {
        let inventory = parse_dom(DOM, None).expect("inventory");
        assert_eq!(inventory.media.len(), 1);
        let media = &inventory.media[0];
        assert_eq!(media.tag, "video");
        assert_eq!(media.src.as_deref(), Some("/media/v.mp4"));
        assert!(media.autoplay);
        assert!(media.muted);
        assert!(!media.controls);
        assert_eq!(media.tracks.len(), 2);
        assert_eq!(media.tracks[0].kind, "captions");
        assert_eq!(media.tracks[1].kind, "descriptions");
    }

    #[test]
    fn an_adjacent_transcription_link_is_found() {
        let inventory = parse_dom(DOM, None).expect("inventory");
        assert_eq!(
            inventory.media[0].transcript_url.as_deref(),
            Some("/transcript.html")
        );
    }

    #[test]
    fn a_third_party_iframe_is_inventoried_not_downloaded() {
        let inventory = parse_dom(DOM, None).expect("inventory");
        assert_eq!(inventory.third_party.len(), 1);
        assert_eq!(inventory.third_party[0].plateforme, "youtube");
    }

    #[test]
    fn an_accessible_control_in_the_ax_tree_counts_as_a_control() {
        let tree = AxTree {
            nodes: vec![AxNode {
                role: "button".into(),
                name: "Mettre en pause".into(),
            }],
        };
        let inventory = parse_dom(DOM, Some(&tree)).expect("inventory");
        assert!(inventory.media[0].controls);
    }

    #[test]
    fn malformed_dom_is_an_error() {
        assert!(parse_dom("not json", None).is_err());
    }
}
