//! Mesure déterministe du contraste et de la distinction des liens (#439, relecture).
//!
//! `obscura` n'est pas Chromium (#371) : `color-contrast` y sort `inapplicable` avec
//! zéro nœud (cause B — `measureText`/`fillText` approchés → `isIconLigature()` vrai pour
//! tout texte), et `link-in-text-block` y rend un faux `passes` (cause A —
//! `getComputedStyle().backgroundImage` vaut `""` au lieu de `none`, si bien qu'axe croit
//! que chaque élément a une image de fond et ne résout aucune couleur).
//!
//! Ce module lit les **styles calculés** via CDP puis calcule le ratio WCAG 2.1 en Rust.
//! Un point non négociable de la relecture : la mesure ne doit **jamais fabriquer de fail**.
//! Elle ne conclut que si le fond est connu de façon fiable (spec §3 étape A) :
//!
//! - un ancêtre opaque est atteint, les couches `rgba` étant composées les unes sur les
//!   autres au lieu d'être traitées comme opaques ;
//! - aucun `background-image` sur le chemin. Obscura renvoie `""` **aussi bien** pour
//!   « pas d'image » que pour une image déclarée en feuille de style : `""` est donc
//!   INCONNU, jamais `none`. On n'accepte l'absence d'image que sur preuve positive
//!   (déclarations inline + CSSOM), et on n'invente jamais un fond blanc par défaut ;
//! - ni `opacity < 1`, ni `filter`, ni `mix-blend-mode`, ni positionnement `absolute`/`fixed`
//!   sur le chemin (le texte peut recouvrir un autre contenu).
//!
//! Dans tous les autres cas, le nœud est ajouté aux `incomplete` d'axe — donc `review` —
//! avec la raison, exactement comme le ferait axe sur un fond image. Le navigateur
//! réellement utilisé reste le binaire Obscura épinglé (#430).

use rgaa_analysis::color::{self, Color};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::results::ObscuraError;

/// JS exécuté dans la page après `axe.run()`, sur la session CDP déjà ouverte.
///
/// Il retourne une chaîne JSON. Le jugement est entièrement côté Rust : le JS se contente
/// de collecter les couleurs calculées, les couches de fond jusqu'au premier fond opaque,
/// et les raisons qui rendent la mesure peu fiable.
pub(crate) const COLLECT_STYLES_JS: &str = r#"
(function () {
  // Obscura renvoie "" pour background-image, y compris quand une image est déclarée
  // dans une feuille de style. "" n'est donc PAS la valeur spec "none" : on croise les
  // déclarations auteur pour établir positivement l'absence d'image / de filtre, etc.
  var tainted = [], cssomAvailable = true;
  // @media/@supports/@layer rules have neither selectorText nor style: their
  // declarations live in nested cssRules. Skipping them would make an image-only
  // background look image-free (Obscura reports "" for background-image) and
  // fabricate a contrast violation.
  function scanRules(rules) {
    for (var ri = 0; ri < rules.length; ri++) {
      var rule = rules[ri], nested = null;
      try { nested = rule.cssRules || rule.rules || null; }
      catch (e) { cssomAvailable = false; }
      if (!nested) {
        try {
          if (rule.styleSheet && rule.styleSheet.cssRules) nested = rule.styleSheet.cssRules;
        } catch (e) { cssomAvailable = false; }
      }
      if (nested && nested.length) { scanRules(nested); }
      var style = rule.style;
      if (!rule.selectorText || !style) { continue; }
      var img = style.backgroundImage || '';
      var filter = style.filter || '';
      var blend = style.mixBlendMode || (style.getPropertyValue ?
        style.getPropertyValue('mix-blend-mode') : '') || '';
      var opacity = style.opacity || '';
      var position = style.position || '';
      if ((img && img !== 'none') || filter || blend || (opacity && opacity !== '1')
          || position === 'absolute' || position === 'fixed') {
        tainted.push(rule.selectorText);
      }
    }
  }
  try {
    var sheets = document.styleSheets;
    for (var si = 0; si < sheets.length; si++) {
      var rules;
      try { rules = sheets[si].cssRules || sheets[si].rules || []; }
      catch (e) { cssomAvailable = false; continue; }
      scanRules(rules);
    }
  } catch (e) { cssomAvailable = false; }

  function inlineRisk(el) {
    var s = el.style;
    if (!s) { return null; }
    if (s.backgroundImage && s.backgroundImage !== 'none') { return 'background-image'; }
    if (s.filter) { return 'filter'; }
    var blend = s.mixBlendMode || (s.getPropertyValue ?
      s.getPropertyValue('mix-blend-mode') : '') || '';
    if (blend && blend !== 'normal') { return 'mix-blend-mode'; }
    if (s.opacity && s.opacity !== '1') { return 'opacity'; }
    if (s.position === 'absolute' || s.position === 'fixed') {
      return 'position ' + s.position;
    }
    return null;
  }

  function declaredRisk(el) {
    var inline = inlineRisk(el);
    if (inline) { return inline; }
    for (var i = 0; i < tainted.length; i++) {
      try {
        if (el.matches && el.matches(tainted[i])) { return 'déclaration CSS à risque'; }
      } catch (e) { /* sélecteur non compris : on reste conservateur */ }
    }
    return null;
  }

  function isOpaque(colour) {
    var match = /^rgba?\(([^)]*)\)$/.exec(colour || '');
    if (!match) { return false; }
    var parts = match[1].split(',');
    return parts.length === 3 || (parts.length === 4 && parseFloat(parts[3]) >= 1);
  }

  // Remonte du nœud jusqu'au premier fond opaque : on collecte les couches (du plus
  // proche du texte vers le fond) et les raisons interdisant une mesure fiable.
  function pathInfo(el) {
    var layers = [], issues = [], node = el;
    while (node && node.nodeType === 1) {
      var style = getComputedStyle(node);
      if (style.opacity && style.opacity !== '1') { issues.push('opacity < 1'); }
      if (style.filter) { issues.push('filter'); }
      if (style.mixBlendMode && style.mixBlendMode !== 'normal') {
        issues.push('mix-blend-mode');
      }
      if (style.position === 'absolute' || style.position === 'fixed') {
        issues.push('position ' + style.position);
      }
      var image = style.backgroundImage;
      if (image && image !== 'none') {
        issues.push('background-image');
      } else if (image === '') {
        if (!cssomAvailable) {
          issues.push('background-image indéterminé');
        } else {
          var risk = declaredRisk(node);
          if (risk) { issues.push(risk); }
        }
      }
      var background = style.backgroundColor;
      if (background && background !== 'transparent'
          && background !== 'rgba(0, 0, 0, 0)' && background !== 'rgba(0,0,0,0)') {
        layers.push(background);
        if (isOpaque(background)) { break; }
      }
      node = node.parentElement;
    }
    return { layers: layers, issues: issues };
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

  // display:none is not inherited in computed style: a child of a hidden container
  // still reports its own display (e.g. "block"). Walk the ancestors so invisible
  // text never turns into a synthetic contrast violation.
  function isRendered(el) {
    var node = el;
    while (node && node.nodeType === 1) {
      var s = getComputedStyle(node);
      if (s.display === 'none') { return false; }
      if (node.getAttribute && node.getAttribute('aria-hidden') === 'true') { return false; }
      node = node.parentElement;
    }
    return true;
  }

  var texts = [], all = document.querySelectorAll('body *');
  for (var i = 0; i < all.length; i++) {
    var el = all[i], style = getComputedStyle(el);
    if (style.visibility === 'hidden' || !isRendered(el) || !directText(el)) { continue; }
    var info = pathInfo(el);
    texts.push({
      tag: el.tagName,
      html: (el.outerHTML || '').slice(0, 200),
      target: selector(el),
      color: style.color,
      layers: info.layers,
      issues: info.issues,
      fontSize: style.fontSize,
      fontWeight: style.fontWeight,
      disabled: !!el.disabled || el.getAttribute('aria-disabled') === 'true'
    });
  }

  var links = [], anchors = document.querySelectorAll('a[href]');
  for (var k = 0; k < anchors.length; k++) {
    var anchor = anchors[k], anchorStyle = getComputedStyle(anchor);
    if (anchorStyle.visibility === 'hidden' || !isRendered(anchor)) { continue; }
    var parent = anchor.parentElement, around = '';
    if (parent) {
      var siblings = parent.childNodes;
      for (var m = 0; m < siblings.length; m++) {
        if (siblings[m].nodeType === 3) { around += (siblings[m].textContent || ''); }
      }
    }
    if (!(around || '').trim()) { continue; }
    var parentStyle = getComputedStyle(parent), linkInfo = pathInfo(anchor);
    links.push({
      tag: 'A',
      html: (anchor.outerHTML || '').slice(0, 200),
      target: selector(anchor),
      color: anchorStyle.color,
      surroundingColor: parentStyle.color,
      decoration: anchorStyle.textDecorationLine || anchorStyle.textDecoration || '',
      fontWeight: anchorStyle.fontWeight,
      parentFontWeight: parentStyle.fontWeight,
      issues: linkInfo.issues
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
        .map_err(|error| error.to_string())?;
    augment_axe_json(&axe_json, &styles).map_err(|error| error.to_string())
}

/// Adds deterministic violations **only** where the background is reliably known, and
/// sends every doubtful node to axe's `incomplete` (→ `review`) with its reason. Never
/// turns a measurement gap into a fabricated `Fail`.
pub(crate) fn augment_axe_json(axe_json: &str, styles: &Value) -> Result<String, ObscuraError> {
    let mut run: Value =
        serde_json::from_str(axe_json).map_err(|error| ObscuraError::Json(error.to_string()))?;
    let Some(object) = run.as_object_mut() else {
        return Err(ObscuraError::Json("résultat axe attendu en objet".into()));
    };
    let mut violations = object
        .get_mut("violations")
        .and_then(Value::as_array_mut)
        .map(std::mem::take)
        .ok_or_else(|| ObscuraError::Json("résultat axe sans tableau 'violations'".into()))?;
    let mut incomplete = object
        .get_mut("incomplete")
        .and_then(Value::as_array_mut)
        .map(std::mem::take)
        .unwrap_or_default();

    let texts = styles.get("texts").and_then(Value::as_array);
    let links = styles.get("links").and_then(Value::as_array);

    let mut text_failures = Vec::new();
    let mut text_reviews = Vec::new();
    for text in texts.into_iter().flatten() {
        if text
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        match text_verdict(text) {
            Verdict::Fail(reason) => text_failures.push(node_for(text, &reason)),
            Verdict::Review(reason) => text_reviews.push(node_for(text, &reason)),
            Verdict::Clean => {}
        }
    }

    let mut link_failures = Vec::new();
    let mut link_reviews = Vec::new();
    for link in links.into_iter().flatten() {
        match link_verdict(link) {
            Verdict::Fail(reason) => link_failures.push(node_for(link, &reason)),
            Verdict::Review(reason) => link_reviews.push(node_for(link, &reason)),
            Verdict::Clean => {}
        }
    }

    if !has_rule(&violations, "color-contrast") && !text_failures.is_empty() {
        violations.push(synthetic_violation(
            "color-contrast",
            "Éléments textuels dont le contraste est insuffisant (mesure #439)",
            text_failures,
        ));
    }
    if !has_rule(&violations, "link-in-text-block") && !link_failures.is_empty() {
        violations.push(synthetic_violation(
            "link-in-text-block",
            "Liens en bloc de texte non distinguables sans la couleur (mesure #439)",
            link_failures,
        ));
    }
    if !text_reviews.is_empty() {
        incomplete.push(synthetic_incomplete(
            "color-contrast",
            "Contraste non mesurable de façon fiable (fond image, fond inconnu, opacité, \
             filtre, mix-blend-mode ou positionnement)",
            text_reviews,
        ));
    }
    if !link_reviews.is_empty() {
        incomplete.push(synthetic_incomplete(
            "link-in-text-block",
            "Distinction du lien non mesurable de façon fiable (opacité, filtre, \
             mix-blend-mode ou positionnement)",
            link_reviews,
        ));
    }

    object.insert("violations".into(), Value::Array(violations));
    object.insert("incomplete".into(), Value::Array(incomplete));
    serde_json::to_string(&run).map_err(|error| ObscuraError::Json(error.to_string()))
}

enum Verdict {
    Fail(String),
    Review(String),
    Clean,
}

/// Reasons collected in the JS disqualify a reliable measurement. An empty list is not
/// enough: the background chain itself must end on a known opaque colour.
fn issues_of(entry: &Value) -> Vec<String> {
    entry
        .get("issues")
        .and_then(Value::as_array)
        .map(|issues| {
            issues
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn text_verdict(text: &Value) -> Verdict {
    let issues = issues_of(text);
    if !issues.is_empty() {
        return Verdict::Review(format!("mesure non fiable : {}", issues.join(" ; ")));
    }
    let layers = text.get("layers").and_then(Value::as_array);
    let Some(background) = layers.and_then(|layers| compose_background(layers)) else {
        return Verdict::Review("aucun fond opaque connu sur le chemin".into());
    };
    let Some(foreground) = text
        .get("color")
        .and_then(Value::as_str)
        .and_then(parse_color)
    else {
        return Verdict::Review("couleur du texte illisible".into());
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
    if ratio < threshold {
        Verdict::Fail(format!(
            "ratio {ratio:.2}:1 < {threshold:.1}:1 (texte {size_px}px, graisse {weight})"
        ))
    } else {
        Verdict::Clean
    }
}

fn link_verdict(link: &Value) -> Verdict {
    let issues = issues_of(link);
    if !issues.is_empty() {
        return Verdict::Review(format!("mesure non fiable : {}", issues.join(" ; ")));
    }
    let (Some(link_colour), Some(around_colour)) = (
        link.get("color")
            .and_then(Value::as_str)
            .and_then(parse_color),
        link.get("surroundingColor")
            .and_then(Value::as_str)
            .and_then(parse_color),
    ) else {
        return Verdict::Review("couleur du lien ou du texte environnant illisible".into());
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
    if decoration.contains("underline")
        || decoration.contains("overline")
        || decoration.contains("line-through")
        || weight != parent_weight
    {
        return Verdict::Clean;
    }
    let ratio = contrast_ratio(link_colour, around_colour);
    if ratio < 3.0 {
        Verdict::Fail(format!(
            "lien à {ratio:.2}:1 du texte environnant (< 3:1) et sans soulignement \
             ni différence de graisse"
        ))
    } else {
        Verdict::Clean
    }
}

/// Composites the background layers (element → ancestor order, stopping at the first
/// opaque one) into the single colour actually seen behind the text. `None` means the
/// background is unknown — the caller turns that into `review`, never into a white guess.
fn compose_background(layers: &[Value]) -> Option<(u8, u8, u8, u8)> {
    let parsed: Vec<(u8, u8, u8, u8)> = layers
        .iter()
        .filter_map(Value::as_str)
        .filter_map(parse_color)
        .collect();
    let opaque_index = parsed.iter().rposition(|colour| colour.3 == 255)?;
    let mut base = parsed[opaque_index];
    for top in parsed[..opaque_index].iter().rev() {
        let alpha = f64::from(top.3) / 255.0;
        base = (
            blend(top.0, base.0, alpha),
            blend(top.1, base.1, alpha),
            blend(top.2, base.2, alpha),
            255,
        );
    }
    Some(base)
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

fn synthetic_incomplete(id: &str, description: &str, nodes: Vec<Value>) -> Value {
    json!({
        "id": id,
        "impact": "incomplete",
        "description": description,
        "nodes": nodes,
    })
}

fn node_for(entry: &Value, reason: &str) -> Value {
    json!({
        "target": [entry.get("target").and_then(Value::as_str).unwrap_or("")],
        "html": entry.get("html").and_then(Value::as_str).unwrap_or(""),
        "failureSummary": reason,
    })
}

/// `rgb(r, g, b)` / `rgba(r, g, b, a)` → `(r, g, b, alpha)` with alpha on 0..=255.
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

/// Converts the local `(r, g, b, a)` tuple to the shared [`Color`] type.
///
/// The tuple API is kept so the CDP collection code and the #456 E2E tests do
/// not change; only the arithmetic is shared with `rgaa-analysis`.
fn as_color(colour: (u8, u8, u8, u8)) -> Color {
    Color {
        r: colour.0,
        g: colour.1,
        b: colour.2,
        a: colour.3,
    }
}

/// Ratio `(L1 + 0,05) / (L2 + 0,05)`, the foreground alpha being composited on the
/// already-composited background, via the shared `rgaa_analysis::color` module.
fn contrast_ratio(foreground: (u8, u8, u8, u8), background: (u8, u8, u8, u8)) -> f64 {
    color::contrast_ratio(as_color(foreground), as_color(background))
}

fn blend(foreground: u8, background: u8, alpha: f64) -> u8 {
    (f64::from(foreground) * alpha + f64::from(background) * (1.0 - alpha)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_with(layers: &str, issues: &str, colour: &str) -> Value {
        serde_json::from_str(&format!(
            r#"{{
                "tag": "P", "html": "<p>x</p>", "target": "body > p:nth-of-type(1)",
                "color": "{colour}", "layers": {layers}, "issues": {issues},
                "fontSize": "18px", "fontWeight": "400", "disabled": false
            }}"#
        ))
        .expect("text fixture")
    }

    fn run(styles: &str) -> Value {
        let styles: Value = serde_json::from_str(styles).expect("styles fixture");
        let result =
            augment_axe_json(r#"{"violations":[],"incomplete":[]}"#, &styles).expect("augment");
        serde_json::from_str(&result).expect("json")
    }

    /// External oracles: ratios published by WebAIM / recorded in the fixtures, never
    /// recomputed from the implementation under test.
    #[test]
    fn wcag_ratios_match_independent_reference_values() {
        let white = parse_color("rgb(255, 255, 255)").expect("white");
        let light_grey = parse_color("rgb(204, 204, 204)").expect("#ccc");
        let ratio = contrast_ratio(light_grey, white);
        assert!((ratio - 1.61).abs() < 0.01, "ratio #ccc/#fff = {ratio}");

        let mid_grey = parse_color("rgb(119, 119, 119)").expect("#777");
        let ratio = contrast_ratio(mid_grey, white);
        assert!((ratio - 4.48).abs() < 0.01, "ratio #777/#fff = {ratio}");
    }

    /// The local tuple wrapper must be bit-identical to a direct call into the
    /// shared module (which also owns the WCAG relative luminance): this is what
    /// guarantees the #456 E2E results are unchanged.
    #[test]
    fn tuple_wrapper_matches_the_shared_color_calculation() {
        assert_eq!(color::relative_luminance(Color::rgb(255, 255, 255)), 1.0);
        assert_eq!(color::relative_luminance(Color::rgb(0, 0, 0)), 0.0);
        let samples = [
            (0, 0, 0, 255),
            (255, 255, 255, 255),
            (204, 204, 204, 255),
            (119, 119, 119, 255),
            (128, 128, 128, 128),
            (17, 34, 51, 64),
        ];
        for &foreground in &samples {
            for &background in &samples {
                assert_eq!(
                    contrast_ratio(foreground, background),
                    color::contrast_ratio(as_color(foreground), as_color(background))
                );
            }
        }
    }

    #[test]
    fn link_ratio_matches_independent_reference_value() {
        let link = parse_color("rgb(128, 128, 128)").expect("#808080");
        let around = parse_color("rgb(102, 102, 102)").expect("#666666");
        let ratio = contrast_ratio(link, around);
        assert!((ratio - 1.46).abs() < 0.01, "ratio lien/texte = {ratio}");
    }

    #[test]
    fn rgba_layer_is_composited_over_the_opaque_ancestor() {
        let layers: Vec<Value> =
            serde_json::from_str(r#"["rgba(0, 0, 0, 0.5019608)", "rgb(255, 255, 255)"]"#)
                .expect("layers");
        let composed = compose_background(&layers).expect("opaque ancestor");
        // 0,5019608 = 128/255 : noir à ce taux sur blanc donne exactement 255 - 128 = 127.
        assert_eq!(composed, (127, 127, 127, 255));

        // WebAIM : #ffffff sur #7f7f7f → 4,00:1 (sous le seuil 4,5 du texte courant).
        let white = parse_color("rgb(255, 255, 255)").expect("white");
        let ratio = contrast_ratio(white, composed);
        assert!((ratio - 4.00).abs() < 0.02, "ratio blanc/composé = {ratio}");
    }

    #[test]
    fn white_text_on_background_image_is_incomplete_never_a_violation() {
        // Obscura renvoie "" pour background-image ; le JS collecte la raison "background-image".
        let result = run(r#"{"texts":[{
                "tag":"P","html":"<p>x</p>","target":"p",
                "color":"rgb(255, 255, 255)","layers":[],"issues":["background-image"],
                "fontSize":"18px","fontWeight":"400","disabled":false
            }],"links":[]}"#);
        assert!(result["violations"].as_array().expect("array").is_empty());
        let incomplete = result["incomplete"].as_array().expect("array");
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0]["id"], "color-contrast");
        assert_eq!(incomplete[0]["nodes"][0]["target"][0], "p");
    }

    #[test]
    fn no_opaque_ancestor_is_incomplete_never_a_white_guess() {
        let result = run(r#"{"texts":[{
                "tag":"P","html":"<p>x</p>","target":"p",
                "color":"rgb(204, 204, 204)","layers":[],"issues":[],
                "fontSize":"18px","fontWeight":"400","disabled":false
            }],"links":[]}"#);
        assert!(result["violations"].as_array().expect("array").is_empty());
        assert_eq!(result["incomplete"].as_array().expect("array").len(), 1);
    }

    #[test]
    fn opacity_filter_blend_and_position_are_incomplete() {
        for reason in [
            "opacity < 1",
            "filter",
            "mix-blend-mode",
            "position absolute",
        ] {
            let result = run(&format!(
                r#"{{"texts":[{{
                    "tag":"P","html":"<p>x</p>","target":"p",
                    "color":"rgb(255, 255, 255)","layers":["rgb(0, 0, 0)"],
                    "issues":["{reason}"],
                    "fontSize":"18px","fontWeight":"400","disabled":false
                }}],"links":[]}}"#
            ));
            assert!(
                result["violations"].as_array().expect("array").is_empty(),
                "{reason} a fabriqué un fail"
            );
            assert_eq!(result["incomplete"].as_array().expect("array").len(), 1);
        }
    }

    #[test]
    fn reliable_low_contrast_text_still_fails() {
        let result = run(&format!(
            r#"{{"texts":[{}],"links":[]}}"#,
            serde_json::to_string(&text_with(
                r#"["rgb(255, 255, 255)"]"#,
                "[]",
                "rgb(204, 204, 204)"
            ))
            .expect("serialize")
        ));
        let violations = result["violations"].as_array().expect("array");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0]["id"], "color-contrast");
    }

    #[test]
    fn augment_adds_both_rules_for_reliable_fixture_data() {
        let result = run(r#"{
                "texts": [{
                    "tag":"P","html":"<p>x</p>","target":"body > p:nth-of-type(1)",
                    "color":"rgb(204, 204, 204)","layers":["rgb(255, 255, 255)"],"issues":[],
                    "fontSize":"18px","fontWeight":"400","disabled":false
                }],
                "links": [{
                    "tag":"A","html":"<a href=\"/cible\">lien</a>","target":"body > p > a:nth-of-type(1)",
                    "color":"rgb(128, 128, 128)","surroundingColor":"rgb(102, 102, 102)",
                    "decoration":"","fontWeight":"400","parentFontWeight":"400","issues":[]
                }]
            }"#);
        let ids: Vec<&str> = result["violations"]
            .as_array()
            .expect("array")
            .iter()
            .map(|entry| entry["id"].as_str().expect("id"))
            .collect();
        assert_eq!(ids, vec!["color-contrast", "link-in-text-block"]);
        assert!(result["incomplete"].as_array().expect("array").is_empty());
    }

    #[test]
    fn augment_does_not_duplicate_a_rule_axe_already_reported() {
        let styles: Value = serde_json::from_str(
            r#"{"texts":[{
                "tag":"P","html":"<p>x</p>","target":"p",
                "color":"rgb(204, 204, 204)","layers":["rgb(255, 255, 255)"],"issues":[],
                "fontSize":"18px","fontWeight":"400","disabled":false
            }],"links":[]}"#,
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
        let result: Value =
            serde_json::from_str(&augment_axe_json(axe, &styles).expect("augment")).expect("json");
        assert_eq!(result["violations"].as_array().expect("array").len(), 1);
    }
}
