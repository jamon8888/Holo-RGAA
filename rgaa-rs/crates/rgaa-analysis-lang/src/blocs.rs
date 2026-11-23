//! Segmentation of the plugin-specific DOM snapshot into text blocks (spec §2.1).
//!
//! The P-00 `DomSnapshot` carries a JSON string; P-06 documents the minimal
//! shape it consumes: `{ "nodes": [ { "id", "tag", "parent", "lang", "text",
//! "alt", "title", "selector" } ] }`. IDs are referenced by `parent`; the
//! closest ancestor `lang` wins, then the page's declared language.

use indexmap::IndexMap;
use rgaa_analysis::analysis_error;
use rgaa_analysis::Result;
use serde::Deserialize;

const MAX_DEPTH: usize = 64;
const MIN_CARACTÈRES: usize = 20;

#[derive(Debug, Clone, Deserialize)]
struct Node {
    id: String,
    tag: String,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    selector: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Snapshot {
    #[serde(default)]
    nodes: Vec<Node>,
}

/// One segment of visible text, with its inherited language and location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bloc {
    pub tag: String,
    pub selecteur: String,
    pub xpath: String,
    pub parent: Option<String>,
    /// Closest `lang` (own or inherited), normalised by the caller later.
    pub lang: Option<String>,
    /// Whether this block carries its own `lang` attribute (8.8, not 8.7).
    pub lang_explicite: bool,
    pub texte: String,
}

fn est_segmentable(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "li"
            | "td"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "blockquote"
            | "figcaption"
            | "button"
            | "a"
    )
}

fn indexer(nodes: &[Node]) -> IndexMap<&str, &Node> {
    let mut index = IndexMap::with_capacity(nodes.len());
    for node in nodes {
        index.entry(node.id.as_str()).or_insert(node);
    }
    index
}

fn lang_héritée<'a>(
    node: &'a Node,
    index: &IndexMap<&'a str, &'a Node>,
    declaree: Option<&str>,
) -> (Option<String>, bool) {
    if let Some(lang) = &node.lang {
        return (Some(lang.clone()), true);
    }
    let mut courant = node;
    for _ in 0..MAX_DEPTH {
        let parent_id = match courant.parent.as_deref() {
            Some(id) => id,
            None => break,
        };
        let parent = match index.get(parent_id) {
            Some(parent) => *parent,
            None => break,
        };
        if let Some(lang) = &parent.lang {
            return (Some(lang.clone()), false);
        }
        courant = parent;
    }
    (declaree.map(str::to_owned), false)
}

fn xpath_de(node: &Node, index: &IndexMap<&str, &Node>) -> String {
    let mut segments = Vec::new();
    let mut courant = node;
    for _ in 0..MAX_DEPTH {
        let mut position = 1usize;
        if let Some(parent_id) = courant.parent.as_deref() {
            for sibling in index.values() {
                if sibling.parent.as_deref() == Some(parent_id) && sibling.tag == courant.tag {
                    if sibling.id == courant.id {
                        break;
                    }
                    position += 1;
                }
            }
        }
        segments.push(if position == 1 {
            courant.tag.clone()
        } else {
            format!("{}[{position}]", courant.tag)
        });
        match courant.parent.as_deref().and_then(|id| index.get(id)) {
            Some(parent) => courant = *parent,
            None => break,
        }
    }
    segments.reverse();
    format!("/{}", segments.join("/"))
}

fn pousser(
    blocs: &mut Vec<Bloc>,
    node: &Node,
    index: &IndexMap<&str, &Node>,
    declaree: Option<&str>,
    tag: &str,
    texte: &str,
) {
    let texte = texte.trim();
    if texte.is_empty() {
        return;
    }
    let (lang, lang_explicite) = lang_héritée(node, index, declaree);
    blocs.push(Bloc {
        tag: tag.to_owned(),
        selecteur: node
            .selector
            .clone()
            .unwrap_or_else(|| format!("#{}", node.id)),
        xpath: xpath_de(node, index),
        parent: node.parent.clone(),
        lang,
        lang_explicite,
        texte: texte.to_owned(),
    });
}

/// Parses a snapshot and segments its visible text.
///
/// # Errors
///
/// Returns [`rgaa_analysis::RgaaError::Analysis`] when the snapshot is not the
/// documented JSON object.
pub fn segmenter(json: &str, declaree: Option<&str>) -> Result<Vec<Bloc>> {
    let snapshot: Snapshot = serde_json::from_str(json)
        .map_err(|error| analysis_error("lang", format!("snapshot DOM invalide : {error}")))?;
    let index = indexer(&snapshot.nodes);
    let mut blocs = Vec::with_capacity(snapshot.nodes.len());

    for node in &snapshot.nodes {
        if est_segmentable(&node.tag) {
            pousser(&mut blocs, node, &index, declaree, &node.tag, &node.text);
        }
        if let Some(alt) = &node.alt {
            pousser(&mut blocs, node, &index, declaree, "alt", alt);
        }
        if let Some(title) = &node.title {
            pousser(&mut blocs, node, &index, declaree, "title", title);
        }
    }

    Ok(grouper_blocs_courts(blocs))
}

/// Groups blocks shorter than 20 characters by parent (spec §2.2).
fn grouper_blocs_courts(blocs: Vec<Bloc>) -> Vec<Bloc> {
    let mut groupes: IndexMap<String, (usize, String)> = IndexMap::new();
    for (position, bloc) in blocs.iter().enumerate() {
        if bloc.texte.chars().count() < MIN_CARACTÈRES {
            let cle = bloc.parent.clone().unwrap_or_default();
            groupes
                .entry(cle)
                .and_modify(|(_, texte)| {
                    texte.push(' ');
                    texte.push_str(&bloc.texte);
                })
                .or_insert((position, bloc.texte.clone()));
        }
    }

    let mut resultat = Vec::with_capacity(blocs.len());
    for (position, bloc) in blocs.into_iter().enumerate() {
        if bloc.texte.chars().count() >= MIN_CARACTÈRES {
            resultat.push(bloc);
            continue;
        }
        let cle = bloc.parent.clone().unwrap_or_default();
        if let Some((premier, texte)) = groupes.get(&cle) {
            if *premier == position {
                let mut groupe = bloc;
                groupe.texte = texte.clone();
                resultat.push(groupe);
            }
        }
    }
    resultat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_inherited_lang_comes_from_the_closest_ancestor() {
        let json = r#"{"nodes":[
            {"id":"html","tag":"html","lang":"fr"},
            {"id":"div","tag":"div","parent":"html","lang":"en"},
            {"id":"p","tag":"p","parent":"div","text":"This is a long enough sentence to be segmented."}
        ]}"#;
        let blocs = segmenter(json, None).expect("blocks");
        assert_eq!(blocs.len(), 1);
        assert_eq!(blocs[0].lang.as_deref(), Some("en"));
        assert!(!blocs[0].lang_explicite);
        assert_eq!(blocs[0].xpath, "/html/div/p");
    }

    #[test]
    fn an_own_lang_marks_the_block_as_explicit() {
        let json = r#"{"nodes":[
            {"id":"html","tag":"html","lang":"fr"},
            {"id":"p","tag":"p","parent":"html","lang":"de","text":"Une phrase française suffisamment longue."}
        ]}"#;
        let blocs = segmenter(json, None).expect("blocks");
        assert!(blocs[0].lang_explicite);
        assert_eq!(blocs[0].lang.as_deref(), Some("de"));
    }

    #[test]
    fn short_siblings_are_grouped_by_parent() {
        let json = r#"{"nodes":[
            {"id":"html","tag":"html","lang":"fr"},
            {"id":"a","tag":"a","parent":"html","text":"Bonjour"},
            {"id":"b","tag":"a","parent":"html","text":"le monde"}
        ]}"#;
        let blocs = segmenter(json, None).expect("blocks");
        assert_eq!(blocs.len(), 1);
        assert_eq!(blocs[0].texte, "Bonjour le monde");
    }

    #[test]
    fn alt_and_title_attributes_become_blocks() {
        let json = r#"{"nodes":[
            {"id":"html","tag":"html","lang":"fr"},
            {"id":"img","tag":"img","parent":"html","alt":"Une image suffisamment longue pour être lue"},
            {"id":"link","tag":"a","parent":"html","title":"Un titre suffisamment long pour être lu"}
        ]}"#;
        let blocs = segmenter(json, None).expect("blocks");
        assert_eq!(blocs.len(), 2);
        assert_eq!(blocs[0].tag, "alt");
        assert_eq!(blocs[1].tag, "title");
    }

    #[test]
    fn malformed_json_is_an_analysis_error() {
        let error = segmenter("not json", None).expect_err("invalid snapshot");
        assert!(error.to_string().contains("snapshot DOM invalide"));
    }
}
