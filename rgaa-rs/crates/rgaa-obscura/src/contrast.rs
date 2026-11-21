//! Mesure déterministe du contraste et de la distinction des liens (#439).
//!
//! `obscura` n'est pas Chromium (#371) : `color-contrast` y sort `inapplicable` avec
//! zéro nœud (cause B — `measureText`/`fillText` approchés → `isIconLigature()` vrai pour
//! tout texte), et `link-in-text-block` y rend un faux `passes` (cause A —
//! `getComputedStyle().backgroundImage` vaut `""` au lieu de `none`, si bien qu'axe croit
//! que chaque élément a une image de fond et ne résout aucune couleur).
//!
//! Plutôt que de réparer le moteur maison (hors de ce dépôt) ou de changer de navigateur
//! (décision produit), ce module mesure ce que les deux règles devraient mesurer :
//! il lit les **styles calculés** via CDP (`getComputedStyle`) puis calcule le ratio WCAG 2.1
//! en Rust, et injecte une violation axe synthétique dans le résultat. Le navigateur
//! réellement utilisé reste le binaire Obscura épinglé (#430).
//!
//! Les seuils et la formule viennent de la spec
//! `docs/specs/plugins-analyse/01-contraste.md` et du RGAA 4.1.2 : 4,5:1 pour le texte
//! courant, 3:1 pour le grand texte (≥ 24 px, ou ≥ 18,66 px avec une graisse ≥ 700) et
//! pour la distinction d'un lien dans un bloc de texte.

use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::results::ObscuraError;

/// JS exécuté dans la page après `axe.run()`, sur la session CDP déjà ouverte.
///
/// Il retourne une **chaîne JSON** (plus robuste qu'un objet remote pour le `returnByValue`
/// d'obscura). Les couleurs sont lues telles que le moteur les calcule (`rgb(r, g, b)` /
/// `rgba(r, g, b, a)`) ; aucune conversion n'est faite côté JS, tout le jugement est en
/// Rust. Le fond est le premier ancestor opaque, exactement comme axe le ferait.
pub(crate) const COLLECT_STYLES_JS: &str = r#"
(function () {
  function effectiveBackground(el) {
    var node = el;
    while (node) {
      var colour = getComputedStyle(node).backgroundColor;
      if (colour && colour !== 'transparent' && colour !== 'rgba(0, 0, 0, 0)'
          && colour !== 'rgba(0,0,0,0)') {
        return colour;
      }
      node = node.parentElement;
    }
    return 'rgb(255, 255, 255)';
  }

  function visible(el) {
    var style = getComputedStyle(el);
    return style.display !== 'none' && style.visibility !== 'hidden'
      && el.getAttribute('aria-hidden') !== 'true';
  }

  function selector(el) {
    var parts = [], node = el;
    while (node && node.tagName) {
      var tag = node.tagName.toLowerCase();
      if (node.id) { parts.unshift(tag + '#' + node.id); break; }
      var index = 1, sibling = node.previousElementSibling;
      while (sibling) {
        if (sibling.tagName === node.tagName) { index++; }
        sibling = sibling.previousElementSibling;
      }
      parts.unshift(tag + ':nth-of-type(' + index + ')');
      node = node.parentElement;
    }
    return parts.join(' > ');
  }

  function directText(el) {
    var children = el.childNodes;
    for (var i = 0; i < children.length; i++) {
      if (children[i].nodeType === 3 && (children[i].textContent || '').trim().length) {
        return true;
      }
    }
    return false;
  }

  var texts = [], all = document.querySelectorAll('body *');
  for (var i = 0; i < all.length; i++) {
    var el = all[i];
    if (!visible(el) || !directText(el)) { continue; }
    var style = getComputedStyle(el);
    texts.push({
      tag: el.tagName,
      html: (el.outerHTML || '').slice(0, 200),
      target: selector(el),
      color: style.color,
      bg: effectiveBackground(el),
      fontSize: style.fontSize,
      fontWeight: style.fontWeight,
      disabled: !!el.disabled || el.getAttribute('aria-disabled') === 'true'
    });
  }

  var links = [], anchors = document.querySelectorAll('a[href]');
  for (var k = 0; k < anchors.length; k++) {
    var anchor = anchors[k];
    if (!visible(anchor)) { continue; }
    var parent = anchor.parentElement, around = '';
    if (parent) {
      var siblings = parent.childNodes;
      for (var m = 0; m < siblings.length; m++) {
        if (siblings[m].nodeType === 3) { around += (siblings[m].textContent || ''); }
      }
    }
    if (!(around || '').trim()) { continue; }
    var linkStyle = getComputedStyle(anchor), parentStyle = getComputedStyle(parent);
    links.push({
      tag: 'A',
      html: (anchor.outerHTML || '').slice(0, 200),
      target: selector(anchor),
      color: linkStyle.color,
      surroundingColor: parentStyle.color,
      decoration: linkStyle.textDecorationLine || linkStyle.textDecoration || '',
      fontWeight: linkStyle.fontWeight,
      parentFontWeight: parentStyle.fontWeight
    });
  }

  return JSON.stringify({ texts: texts, links: links });
})()
"#;

/// Reads the computed styles on the page already loaded in `ws`/`session_id`.
pub(crate) async fn collect_styles(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    session_id: &str,
) -> Result<Value, ObscuraError> {
    let response = crate::cdp_pool::cdp_send_session(
        ws,
        session_id,
        "Runtime.evaluate",
        json!({
            "expression": COLLECT_STYLES_JS,
            "returnByValue": true,
        }),
    )
    .await
    .map_err(ObscuraError::CdpTransport)?;

    if let Some(exception) = response.get("exceptionDetails") {
        return Err(ObscuraError::Evaluation(format!(
            "collecte des styles calculés : {exception}"
        )));
    }

    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .ok_or_else(|| ObscuraError::Evaluation("réponse CDP sans styles calculés".into()))?;

    match value {
        Value::String(text) => {
            serde_json::from_str(text).map_err(|error| ObscuraError::Json(error.to_string()))
        }
        Value::Object(_) => Ok(value.clone()),
        other => Err(ObscuraError::Evaluation(format!(
            "styles calculés inattendus : {other}"
        ))),
    }
}

/// Collects the styles then augments the axe result in one call, so both the pooled
/// session path and the direct-connection fallback stay in lockstep.
pub(crate) async fn augment_result(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    session_id: &str,
    axe_json: String,
) -> Result<String, String> {
    let styles = collect_styles(ws, session_id)
        .await
        .map_err(|e| e.to_string())?;
    augment_axe_json(&axe_json, &styles).map_err(|e| e.to_string())
}

/// Appends a `color-contrast` and/or a `link-in-text-block` violation to the axe run
/// object when the deterministic measurement finds one. A rule axe already reported is
/// never duplicated (matters when a real Chromium is used as the substrate instead).
pub(crate) fn augment_axe_json(axe_json: &str, styles: &Value) -> Result<String, ObscuraError> {
    let mut run: Value =
        serde_json::from_str(axe_json).map_err(|error| ObscuraError::Json(error.to_string()))?;
    let violations = run
        .get_mut("violations")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| ObscuraError::Json("résultat axe sans tableau 'violations'".into()))?;

    let texts = styles.get("texts").and_then(Value::as_array);
    let links = styles.get("links").and_then(Value::as_array);

    if !has_rule(violations, "color-contrast") {
        if let Some(nodes) = texts.and_then(|texts| low_contrast_text_nodes(texts)) {
            violations.push(synthetic_violation(
                "color-contrast",
                "Éléments textuels dont le contraste est insuffisant (mesure #439)",
                nodes,
            ));
        }
    }
    if !has_rule(violations, "link-in-text-block") {
        if let Some(nodes) = links.and_then(|links| undistinguished_link_nodes(links)) {
            violations.push(synthetic_violation(
                "link-in-text-block",
                "Liens en bloc de texte non distinguables sans la couleur (mesure #439)",
                nodes,
            ));
        }
    }

    serde_json::to_string(&run).map_err(|error| ObscuraError::Json(error.to_string()))
}

fn has_rule(violations: &[Value], id: &str) -> bool {
    violations
        .iter()
        .any(|violation| violation.get("id").and_then(Value::as_str) == Some(id))
}

fn synthetic_violation(id: &str, description: &str, nodes: Vec<Value>) -> Value {
    json!({
        "id": id,
        "impact": "serious",
        "description": description,
        "nodes": nodes,
    })
}

/// 3.2 — tout texte visible dont le ratio est sous 4,5:1 (3:1 pour le grand texte).
fn low_contrast_text_nodes(texts: &[Value]) -> Option<Vec<Value>> {
    let mut nodes = Vec::new();
    for text in texts {
        if text
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let (Some(foreground), Some(background)) = (
            text.get("color").and_then(Value::as_str),
            text.get("bg").and_then(Value::as_str),
        ) else {
            continue;
        };
        let (Some(foreground), Some(background)) =
            (parse_color(foreground), parse_color(background))
        else {
            continue;
        };
        let size_px = text
            .get("fontSize")
            .and_then(Value::as_str)
            .and_then(parse_px)
            .unwrap_or(16.0);
        let weight = text
            .get("fontWeight")
            .and_then(Value::as_str)
            .map_or(400, parse_weight);
        let large = size_px >= 24.0 || (size_px >= 18.66 && weight >= 700);
        let threshold = if large { 3.0 } else { 4.5 };
        let ratio = contrast_ratio(foreground, background);
        if ratio >= threshold {
            continue;
        }
        let target = text.get("target").and_then(Value::as_str).unwrap_or("");
        let html = text.get("html").and_then(Value::as_str).unwrap_or("");
        nodes.push(json!({
            "target": [target],
            "html": html,
            "failureSummary": format!(
                "ratio {ratio:.2}:1 < {threshold:.1}:1 (texte {size_px}px, graisse {weight})"
            ),
        }));
    }
    (!nodes.is_empty()).then_some(nodes)
}

/// 10.6 — lien dont la couleur est à moins de 3:1 du texte environnant et qui n'a aucune
/// autre distinction visuelle (soulignement, autre graisse).
fn undistinguished_link_nodes(links: &[Value]) -> Option<Vec<Value>> {
    let mut nodes = Vec::new();
    for link in links {
        let (Some(link_colour), Some(around_colour)) = (
            link.get("color").and_then(Value::as_str),
            link.get("surroundingColor").and_then(Value::as_str),
        ) else {
            continue;
        };
        let (Some(link_colour), Some(around_colour)) =
            (parse_color(link_colour), parse_color(around_colour))
        else {
            continue;
        };
        let decoration = link
            .get("decoration")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let weight = link
            .get("fontWeight")
            .and_then(Value::as_str)
            .map_or(400, parse_weight);
        let parent_weight = link
            .get("parentFontWeight")
            .and_then(Value::as_str)
            .map_or(400, parse_weight);
        let visually_distinct = decoration.contains("underline")
            || decoration.contains("overline")
            || decoration.contains("line-through")
            || weight != parent_weight;
        if visually_distinct {
            continue;
        }
        let ratio = contrast_ratio(link_colour, around_colour);
        if ratio >= 3.0 {
            continue;
        }
        let target = link.get("target").and_then(Value::as_str).unwrap_or("");
        let html = link.get("html").and_then(Value::as_str).unwrap_or("");
        nodes.push(json!({
            "target": [target],
            "html": html,
            "failureSummary": format!(
                "lien à {ratio:.2}:1 du texte environnant (< 3:1) et sans soulignement \
                 ni différence de graisse"
            ),
        }));
    }
    (!nodes.is_empty()).then_some(nodes)
}

/// `rgb(r, g, b)` / `rgba(r, g, b, a)` → `(r, g, b, alpha)` avec alpha sur 0..=255.
fn parse_color(raw: &str) -> Option<(u8, u8, u8, u8)> {
    let raw = raw.trim();
    let inner = raw
        .strip_prefix("rgba(")
        .or_else(|| raw.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let mut channels = inner.split(',');
    let red = parse_channel(channels.next()?)?;
    let green = parse_channel(channels.next()?)?;
    let blue = parse_channel(channels.next()?)?;
    let alpha = match channels.next() {
        Some(value) => (clamp(parse_scalar(value)?, 0.0, 1.0) * 255.0).round() as u8,
        None => 255,
    };
    Some((red, green, blue, alpha))
}

fn parse_channel(raw: &str) -> Option<u8> {
    let value = clamp(parse_scalar(raw)?, 0.0, 255.0);
    Some(value.round() as u8)
}

fn parse_scalar(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok()
}

fn clamp(value: f64, low: f64, high: f64) -> f64 {
    value.max(low).min(high)
}

fn parse_px(raw: &str) -> Option<f64> {
    parse_scalar(raw.trim().strip_suffix("px")?)
}

fn parse_weight(raw: &str) -> u16 {
    match raw.trim() {
        "bold" | "bolder" => 700,
        "normal" | "lighter" | "" => 400,
        other => other.parse::<u16>().unwrap_or(400),
    }
}

/// Luminance relative WCAG 2.1, avec le seuil de linéarisation sRGB à 0,04045.
fn relative_luminance(colour: (u8, u8, u8, u8)) -> f64 {
    fn linear(channel: u8) -> f64 {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(colour.0) + 0.7152 * linear(colour.1) + 0.0722 * linear(colour.2)
}

/// Ratio `(L1 + 0,05) / (L2 + 0,05)`, l'alpha du texte étant composé sur le fond.
fn contrast_ratio(foreground: (u8, u8, u8, u8), background: (u8, u8, u8, u8)) -> f64 {
    let alpha = f64::from(foreground.3) / 255.0;
    let composite = (
        blend(foreground.0, background.0, alpha),
        blend(foreground.1, background.1, alpha),
        blend(foreground.2, background.2, alpha),
        255,
    );
    let lighter = relative_luminance(composite).max(relative_luminance(background));
    let darker = relative_luminance(composite).min(relative_luminance(background));
    (lighter + 0.05) / (darker + 0.05)
}

fn blend(foreground: u8, background: u8, alpha: f64) -> u8 {
    (f64::from(foreground) * alpha + f64::from(background) * (1.0 - alpha)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Oracles externes : valeurs publiées dans la fixture et la spec 01-contraste.md,
    /// jamais recalculées par le code sous test.
    #[test]
    fn wcag_ratios_match_independent_reference_values() {
        let white = parse_color("rgb(255, 255, 255)").expect("white");
        let light_grey = parse_color("rgb(204, 204, 204)").expect("#ccc");

        // WebAIM : #cccccc sur #ffffff → 1.61:1 ; #777777 sur #ffffff → 4.48:1.
        let ratio = contrast_ratio(light_grey, white);
        assert!((ratio - 1.61).abs() < 0.01, "ratio #ccc/#fff = {ratio}");

        let mid_grey = parse_color("rgb(119, 119, 119)").expect("#777");
        let ratio = contrast_ratio(mid_grey, white);
        assert!((ratio - 4.48).abs() < 0.01, "ratio #777/#fff = {ratio}");
    }

    #[test]
    fn link_ratio_matches_independent_reference_value() {
        let link = parse_color("rgb(128, 128, 128)").expect("#808080");
        let around = parse_color("rgb(102, 102, 102)").expect("#666666");
        // Ratio calculé par axe/WebAIM et consigné dans la fixture 10.6.
        let ratio = contrast_ratio(link, around);
        assert!((ratio - 1.46).abs() < 0.01, "ratio lien/texte = {ratio}");
    }

    #[test]
    fn large_text_uses_the_three_to_one_threshold() {
        let white = parse_color("rgb(255, 255, 255)").expect("white");
        let grey = parse_color("rgb(204, 204, 204)").expect("#ccc");
        let ratio = contrast_ratio(grey, white);
        assert!(ratio < 3.0, "témoin sous le seuil grand texte");
        assert!(ratio < 4.5, "témoin sous le seuil texte courant");
    }

    #[test]
    fn augment_adds_both_rules_for_the_fixtures() {
        let styles: Value = serde_json::from_str(
            r#"{
                "texts": [{
                    "tag": "P",
                    "html": "<p>x</p>",
                    "target": "body > p:nth-of-type(1)",
                    "color": "rgb(204, 204, 204)",
                    "bg": "rgb(255, 255, 255)",
                    "fontSize": "18px",
                    "fontWeight": "400",
                    "disabled": false
                }],
                "links": [{
                    "tag": "A",
                    "html": "<a href=\"/cible\">lien</a>",
                    "target": "body > p > a:nth-of-type(1)",
                    "color": "rgb(128, 128, 128)",
                    "surroundingColor": "rgb(102, 102, 102)",
                    "decoration": "",
                    "fontWeight": "400",
                    "parentFontWeight": "400"
                }]
            }"#,
        )
        .expect("styles fixture");
        let augmented =
            augment_axe_json(r#"{"violations":[],"incomplete":[]}"#, &styles).expect("augment");
        let parsed: Value = serde_json::from_str(&augmented).expect("json");
        let ids: Vec<&str> = parsed["violations"]
            .as_array()
            .expect("array")
            .iter()
            .map(|v| v["id"].as_str().expect("id"))
            .collect();
        assert_eq!(ids, vec!["color-contrast", "link-in-text-block"]);
    }

    #[test]
    fn augment_does_not_duplicate_a_rule_axe_already_reported() {
        let styles: Value = serde_json::from_str(
            r#"{
                "texts": [{
                    "tag": "P", "html": "<p>x</p>", "target": "p",
                    "color": "rgb(204, 204, 204)", "bg": "rgb(255, 255, 255)",
                    "fontSize": "18px", "fontWeight": "400", "disabled": false
                }],
                "links": []
            }"#,
        )
        .expect("styles fixture");
        let axe = r#"{
            "violations": [{
                "id": "color-contrast", "impact": "serious",
                "description": "déjà rapporté par axe",
                "nodes": [{"target": ["p"], "html": "<p>x</p>"}]
            }],
            "incomplete": []
        }"#;
        let augmented = augment_axe_json(axe, &styles).expect("augment");
        let parsed: Value = serde_json::from_str(&augmented).expect("json");
        assert_eq!(parsed["violations"].as_array().expect("array").len(), 1);
    }
}
