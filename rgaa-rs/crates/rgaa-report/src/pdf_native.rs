//! Native tagged PDF/UA output, without an external Chromium (spike for #152).
//!
//! Behind the `pdf-native` Cargo feature: the default build and the existing
//! `pdf::print_to_pdf_chromium` path are untouched.
//!
//! The document is composed from semantic [`Block`]s (title, headings,
//! paragraphs), never from rendered HTML, so the tag tree is built directly:
//! one tag group per block, one marked-content span per line, in reading order.
//! The PDF is exported through krilla's **PDF/UA-1 validator**, so a document
//! that is not a valid tagged PDF fails at export instead of shipping.
//!
//! Spike limits, deliberately left open:
//! - The caller supplies the font bytes. Every font a PDF/UA file uses must be
//!   embedded, and which font ships with the crate is a licensing decision.
//! - Layout is a plain greedy line wrap on a single column. No visual parity
//!   with the HTML declaration is claimed.
//! - Lists are drawn as dash-prefixed paragraphs, not as `L`/`LI`/`LBody`.
//! - [`declaration_fr_blocks`] repeats the wording of the HTML template; the
//!   legal sentences should live in one place before this replaces Chromium.

use std::num::NonZeroU16;
use std::sync::Arc;

use krilla::configure::{Accessibility, ConfigurationBuilder};
use krilla::destination::XyzDestination;
use krilla::geom::Point;
use krilla::metadata::Metadata;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::tagging::{ContentTag, SpanTag, Tag, TagGroup, TagTree};
use krilla::text::{Font, TextDirection};
use krilla::{Document, SerializeSettings};

use crate::declaration::DeclarationFrInput;
use crate::packs::{etat_fr, RECOURS_FR};
use crate::ReportError;

const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;
const MARGIN: f32 = 56.0;
const LINE_SPACING: f32 = 1.35;

/// One semantic unit of the document, in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// The document title: the single `H1`.
    Title(String),
    /// A heading of level 2 or more.
    Heading(u16, String),
    /// A paragraph of running text.
    Paragraph(String),
    /// One item of a list.
    ListItem(String),
}

impl Block {
    fn text(&self) -> &str {
        match self {
            Self::Title(t) | Self::Heading(_, t) | Self::Paragraph(t) | Self::ListItem(t) => t,
        }
    }

    fn font_size(&self) -> f32 {
        match self {
            Self::Title(_) => 20.0,
            Self::Heading(2, _) => 15.0,
            Self::Heading(..) => 12.5,
            Self::Paragraph(_) | Self::ListItem(_) => 11.0,
        }
    }

    fn space_before(&self) -> f32 {
        match self {
            Self::Title(_) => 0.0,
            Self::Heading(..) => 14.0,
            Self::Paragraph(_) | Self::ListItem(_) => 0.0,
        }
    }

    fn space_after(&self) -> f32 {
        match self {
            Self::Title(_) => 14.0,
            Self::Heading(..) => 6.0,
            Self::Paragraph(_) | Self::ListItem(_) => 7.0,
        }
    }

    /// The heading level, `None` for body text.
    fn heading_level(&self) -> Option<u16> {
        match self {
            Self::Title(_) => Some(1),
            Self::Heading(level, _) => Some(*level),
            Self::Paragraph(_) | Self::ListItem(_) => None,
        }
    }

    fn text_to_draw(&self) -> String {
        match self {
            Self::ListItem(t) => format!("– {t}"),
            other => other.text().to_string(),
        }
    }

    fn tag(&self) -> Result<krilla::tagging::TagKind, ReportError> {
        Ok(match self {
            Self::Title(text) => Tag::Hn(NonZeroU16::MIN, Some(text.clone())).into(),
            Self::Heading(level, text) => {
                let level = NonZeroU16::new(*level)
                    .filter(|l| l.get() >= 2)
                    .ok_or_else(|| ReportError::invalid_input("niveau de titre invalide (>= 2)"))?;
                Tag::Hn(level, Some(text.clone())).into()
            }
            Self::Paragraph(_) | Self::ListItem(_) => Tag::P.into(),
        })
    }
}

/// A laid-out line of one block.
#[derive(Debug, Clone, PartialEq)]
struct Line {
    block: usize,
    text: String,
    size: f32,
    /// Baseline, measured from the top of the page.
    baseline: f32,
}

type Page = Vec<Line>;

fn text_width(face: &rustybuzz::Face<'_>, text: &str, size: f32) -> f32 {
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    let shaped = rustybuzz::shape(face, &[], buffer);
    let advance: i32 = shaped.glyph_positions().iter().map(|p| p.x_advance).sum();
    advance as f32 * size / f32::from(face.units_per_em() as u16)
}

/// Greedy wrap. A word wider than the line is split by characters rather than
/// left to run off the page: declarations list long URLs.
fn wrap(face: &rustybuzz::Face<'_>, text: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if text_width(face, &candidate, size) <= max_width {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        let mut piece = String::new();
        for ch in word.chars() {
            let mut next = piece.clone();
            next.push(ch);
            if !piece.is_empty() && text_width(face, &next, size) > max_width {
                lines.push(std::mem::take(&mut piece));
            }
            piece.push(ch);
        }
        current = piece;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn layout(face: &rustybuzz::Face<'_>, blocks: &[Block]) -> Vec<Page> {
    let max_width = PAGE_WIDTH - 2.0 * MARGIN;
    let bottom = PAGE_HEIGHT - MARGIN;
    let mut pages: Vec<Page> = vec![Vec::new()];
    let mut y = MARGIN;
    for (index, block) in blocks.iter().enumerate() {
        let size = block.font_size();
        let lines = wrap(face, &block.text_to_draw(), size, max_width);
        if lines.is_empty() {
            continue;
        }
        let leading = size * LINE_SPACING;
        let at_top = pages.last().is_some_and(Vec::is_empty);
        y += if at_top { 0.0 } else { block.space_before() };
        for text in lines {
            if y + leading > bottom && !pages.last().is_some_and(Vec::is_empty) {
                pages.push(Vec::new());
                y = MARGIN;
            }
            y += leading;
            if let Some(page) = pages.last_mut() {
                page.push(Line {
                    block: index,
                    text,
                    size,
                    baseline: y,
                });
            }
        }
        y += block.space_after();
    }
    pages
}

/// Renders `blocks` as a tagged PDF/UA-1 document.
///
/// `lang` is the document language (`"fr"`), written to the catalog. `font` is
/// the bytes of a TrueType/OpenType font that contains every glyph of the text.
///
/// # Errors
/// Fails when the font cannot be parsed, a heading level is invalid, or krilla's
/// PDF/UA-1 validation rejects the document.
pub fn render_pdf(
    title: &str,
    lang: &str,
    blocks: &[Block],
    font: &[u8],
) -> Result<Vec<u8>, ReportError> {
    let face = rustybuzz::Face::from_slice(font, 0)
        .ok_or_else(|| ReportError::execution("police illisible pour le PDF natif"))?;
    let krilla_font = Font::new(Arc::new(font.to_vec()).into(), 0)
        .ok_or_else(|| ReportError::execution("police refusée par krilla"))?;

    let configuration = ConfigurationBuilder::new()
        .with_accessibility_validator(Accessibility::UA1)
        .finish()
        .map_err(|e| ReportError::execution(format!("configuration PDF/UA : {e:?}")))?;
    let mut document = Document::new_with(SerializeSettings {
        configuration,
        ..SerializeSettings::default()
    });
    document.set_metadata(
        Metadata::new()
            .title(title.to_string())
            .language(lang.to_string()),
    );

    let mut groups: Vec<Option<TagGroup>> = Vec::with_capacity(blocks.len());
    for block in blocks {
        groups.push(Some(TagGroup::new(block.tag()?)));
    }

    // PDF/UA wants a document outline; one entry per heading, at the heading's
    // first line.
    let mut headings: Vec<(u16, String, usize, f32)> = Vec::new();
    let mut seen_heading_blocks = std::collections::HashSet::new();

    for (page_index, page_lines) in layout(&face, blocks).into_iter().enumerate() {
        let settings = PageSettings::from_wh(PAGE_WIDTH, PAGE_HEIGHT)
            .ok_or_else(|| ReportError::execution("dimensions de page invalides"))?;
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        for line in &page_lines {
            if let Some(level) = blocks.get(line.block).and_then(Block::heading_level) {
                if seen_heading_blocks.insert(line.block) {
                    headings.push((
                        level,
                        blocks[line.block].text().to_string(),
                        page_index,
                        line.baseline - line.size,
                    ));
                }
            }
            let id = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
            surface.draw_text(
                Point::from_xy(MARGIN, line.baseline),
                krilla_font.clone(),
                line.size,
                &line.text,
                false,
                TextDirection::Auto,
            );
            surface.end_tagged();
            if let Some(Some(group)) = groups.get_mut(line.block) {
                group.push(id);
            }
        }
        surface.finish();
        page.finish();
    }

    let mut tree = TagTree::new();
    for group in groups.into_iter().flatten() {
        tree.push(group);
    }
    document.set_tag_tree(tree);
    document.set_outline(build_outline(&headings));

    document
        .finish()
        .map_err(|e| ReportError::execution(format!("PDF/UA invalide : {e:?}")))
}

/// Nests the headings by level: H1 holds the H2s, which hold the H3s.
fn build_outline(headings: &[(u16, String, usize, f32)]) -> Outline {
    let mut outline = Outline::new();
    let mut stack: Vec<(u16, OutlineNode)> = Vec::new();

    fn close(stack: &mut Vec<(u16, OutlineNode)>, outline: &mut Outline) {
        if let Some((_, node)) = stack.pop() {
            match stack.last_mut() {
                Some((_, parent)) => parent.push_child(node),
                None => outline.push_child(node),
            }
        }
    }

    for (level, text, page, top) in headings {
        while stack.last().is_some_and(|(open, _)| open >= level) {
            close(&mut stack, &mut outline);
        }
        let destination = XyzDestination::new(*page, Point::from_xy(MARGIN, top.max(0.0)));
        stack.push((*level, OutlineNode::new(text.clone(), destination)));
    }
    while !stack.is_empty() {
        close(&mut stack, &mut outline);
    }
    outline
}

/// The French declaration as semantic blocks, mirroring the HTML sections.
///
/// # Errors
/// Fails, like [`crate::render_declaration_fr`], when no feedback destination
/// exists.
pub fn declaration_fr_blocks(input: &DeclarationFrInput) -> Result<Vec<Block>, ReportError> {
    let destination = crate::guard::destination_contact(input.contact).ok_or_else(|| {
        ReportError::invalid_input(
            "destination du contact de retour d'information manquante".to_string(),
        )
    })?;
    let m = input.metrics;
    let mut out = vec![
        Block::Title("Déclaration d'accessibilité".into()),
        Block::Heading(2, "1. Engagement".into()),
        Block::Paragraph(format!(
            "{} s'engage à rendre son service {} accessible conformément à l'article 47 de la loi n° 2005-102 du 11 février 2005.",
            input.organisme, input.service
        )),
        Block::Heading(2, "2. État de conformité".into()),
        Block::Paragraph(format!(
            "{} est {} avec le Référentiel Général d'Amélioration de l'Accessibilité (RGAA), version 4.1.2. Le taux de conformité s'élève à {:.1} %, établi sur {:.1} % de la surface testable ({} critères conformes, {} non conformes, {} non applicables, {} non testés).{}",
            input.service,
            etat_fr(&m.etat_conformite),
            m.taux_global,
            m.coverage_percent,
            m.conformes,
            m.non_conformes,
            m.non_applicables,
            m.non_testes,
            if m.rate_is_a_conformance_claim() {
                ""
            } else {
                " Audit incomplet : ce taux est une mesure, pas une déclaration de conformité."
            }
        )),
        Block::Heading(2, "3. Résultats des tests et contenus non accessibles".into()),
        Block::Heading(3, "Non-conformités".into()),
    ];

    if input.non_conformites.is_empty() {
        out.push(Block::Paragraph("Aucune non-conformité relevée.".into()));
    }
    for nc in input.non_conformites {
        out.push(Block::ListItem(format!(
            "{} — {} (preuve : {} ; recommandation : {}).",
            nc.intitule, nc.url, nc.preuve, nc.recommandation
        )));
    }

    out.push(Block::Heading(
        3,
        "Dérogations pour charge disproportionnée".into(),
    ));
    if input.derogations.is_empty() {
        out.push(Block::Paragraph("Aucune dérogation.".into()));
    }
    for d in input.derogations {
        out.push(Block::ListItem(format!(
            "{} (motif : {}) : alternative = {} ; réexamen le {}.",
            d.contenu, d.motif, d.alternative, d.date_reexamen
        )));
    }

    out.push(Block::Heading(
        3,
        "Contenus non soumis à l'obligation".into(),
    ));
    if input.non_soumis.is_empty() {
        out.push(Block::Paragraph("Aucun contenu non soumis.".into()));
    }
    for c in input.non_soumis {
        out.push(Block::ListItem(format!(
            "{} : {}.",
            c.categorie, c.justification
        )));
    }

    let pages = input
        .pages
        .iter()
        .map(|p| p.page_type.clone())
        .collect::<Vec<_>>()
        .join(", ");
    out.extend([
        Block::Heading(2, "4. Établissement de cette déclaration".into()),
        Block::Paragraph(format!(
            "Déclaration établie le {}. Technologies utilisées : {}. Environnement de test : {}. Pages auditées : {}.",
            input.date_declaration,
            input.technologies.join(", "),
            input.environnement,
            pages
        )),
        Block::Heading(2, "5. Retour d'information et contact".into()),
        Block::Paragraph(format!("Contact : {destination}.")),
        Block::Heading(2, "6. Voies de recours".into()),
        Block::Paragraph(RECOURS_FR.to_string()),
        Block::Heading(2, "7. Schéma pluriannuel et plan d'action".into()),
    ]);
    out.push(Block::Paragraph(
        match (input.schema_pluriannuel_url, input.plan_action_url) {
            (Some(schema), Some(plan)) => format!(
                "Voir le schéma pluriannuel ({schema}) et le plan d'action de l'année en cours ({plan})."
            ),
            _ => "Schéma pluriannuel et plan d'action en cours de publication.".into(),
        },
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::{Contact, ContenuNonSoumis, Derogation, PageEchantillon};
    use crate::{compute_metrics, NcEntry, SiteMetrics, RGAA_41};
    use rgaa_core::{Classification, CriterionResult, CriterionStatus};

    /// Any font that carries the French accents will do; the crate ships none.
    /// A missing font fails loudly: skipping would let these tests pass on a
    /// machine that exercises nothing.
    fn font_bytes() -> Vec<u8> {
        let candidates = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/Library/Fonts/Arial Unicode.ttf",
        ];
        if let Ok(path) = std::env::var("RGAA_TEST_FONT") {
            return std::fs::read(&path).unwrap_or_else(|e| panic!("RGAA_TEST_FONT {path}: {e}"));
        }
        for path in candidates {
            if let Ok(bytes) = std::fs::read(path) {
                return bytes;
            }
        }
        panic!(
            "no test font found: set RGAA_TEST_FONT to a TrueType font with Latin accents (tried {candidates:?})"
        );
    }

    fn metrics() -> SiteMetrics {
        let result = |id: &str, status| CriterionResult {
            criterion_id: id.into(),
            title: "t".into(),
            classification: Classification::Deterministe,
            status,
            violations: Vec::new(),
            confidence: None,
            justification: None,
            source: "t".into(),
            citations: Vec::new(),
            considered_sources: vec![],
            tests: vec![],
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        };
        compute_metrics(
            &[
                result("1.1", CriterionStatus::Pass),
                result("1.2", CriterionStatus::Fail),
            ],
            &RGAA_41,
        )
    }

    fn contact() -> Contact {
        Contact {
            canal: "email".into(),
            email: Some("aide@example.test".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        }
    }

    fn blocks_for(ncs: &[NcEntry]) -> Vec<Block> {
        let metrics = metrics();
        let contact = contact();
        let derogations: Vec<Derogation> = Vec::new();
        let non_soumis: Vec<ContenuNonSoumis> = Vec::new();
        let technologies = vec!["HTML5".to_string()];
        let pages: Vec<PageEchantillon> = Vec::new();
        declaration_fr_blocks(&DeclarationFrInput {
            service: "Service Client",
            organisme: "Ministère Exemple",
            metrics: &metrics,
            non_conformites: ncs,
            derogations: &derogations,
            non_soumis: &non_soumis,
            contact: &contact,
            date_declaration: "2026-09-19",
            technologies: &technologies,
            pages: &pages,
            environnement: "Firefox + NVDA",
            schema_pluriannuel_url: Some("https://example.test/schema"),
            plan_action_url: Some("https://example.test/plan"),
        })
        .expect("blocs")
    }

    fn nc(url: &str) -> NcEntry {
        NcEntry {
            intitule: "image sans alternative".into(),
            url: url.into(),
            preuve: "dom:abc12345".into(),
            recommandation: "ajouter un attribut alt".into(),
        }
    }

    #[test]
    fn exports_a_tagged_pdf_ua_document_with_a_language() {
        let blocks = blocks_for(&[nc("https://example.test/contact")]);
        let pdf = render_pdf(
            "Déclaration d'accessibilité — Service Client",
            "fr",
            &blocks,
            &font_bytes(),
        )
        .expect("krilla's PDF/UA-1 validation accepts the document");

        // RGAA_PDF_DUMP=/tmp/x.pdf keeps the file for inspection with an external checker.
        if let Ok(path) = std::env::var("RGAA_PDF_DUMP") {
            std::fs::write(&path, &pdf).expect("write RGAA_PDF_DUMP");
        }
        assert!(pdf.starts_with(b"%PDF-"));
        let has = |needle: &[u8]| pdf.windows(needle.len()).any(|w| w == needle);
        assert!(has(b"/StructTreeRoot"), "no structure tree");
        assert!(has(b"/MarkInfo"), "not marked as tagged");
        assert!(has(b"/Lang"), "no document language");
    }

    #[test]
    fn reading_order_follows_the_block_order_across_pages() {
        let many: Vec<NcEntry> = (0..60)
            .map(|i| nc(&format!("https://example.test/{i}")))
            .collect();
        let blocks = blocks_for(&many);
        let font = font_bytes();
        let face = rustybuzz::Face::from_slice(&font, 0).expect("face");
        let pages = layout(&face, &blocks);

        assert!(pages.len() > 1, "60 non-conformities must paginate");
        let mut last_block = 0usize;
        for page in &pages {
            let mut last_baseline = 0.0f32;
            for line in page {
                assert!(line.block >= last_block, "a line precedes an earlier block");
                assert!(
                    line.baseline >= last_baseline,
                    "baselines go back up a page"
                );
                last_block = line.block;
                last_baseline = line.baseline;
            }
        }
    }

    #[test]
    fn long_urls_never_run_past_the_right_margin() {
        let long = format!("https://example.test/{}", "a".repeat(400));
        let blocks = blocks_for(&[nc(&long)]);
        let font = font_bytes();
        let face = rustybuzz::Face::from_slice(&font, 0).expect("face");
        for page in layout(&face, &blocks) {
            for line in page {
                let width = text_width(&face, &line.text, line.size);
                assert!(
                    width <= PAGE_WIDTH - 2.0 * MARGIN + 0.5,
                    "line is {width}pt wide: {}",
                    line.text
                );
            }
        }
    }

    #[test]
    fn a_declaration_without_a_feedback_destination_is_refused() {
        let metrics = metrics();
        let contact = Contact {
            canal: "email".into(),
            email: None,
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        };
        let empty_d: Vec<Derogation> = Vec::new();
        let empty_n: Vec<ContenuNonSoumis> = Vec::new();
        let tech: Vec<String> = Vec::new();
        let pages: Vec<PageEchantillon> = Vec::new();
        let error = declaration_fr_blocks(&DeclarationFrInput {
            service: "s",
            organisme: "o",
            metrics: &metrics,
            non_conformites: &[],
            derogations: &empty_d,
            non_soumis: &empty_n,
            contact: &contact,
            date_declaration: "2026-09-19",
            technologies: &tech,
            pages: &pages,
            environnement: "e",
            schema_pluriannuel_url: None,
            plan_action_url: None,
        })
        .expect_err("no destination");
        assert!(error.to_string().contains("destination"));
    }

    #[test]
    fn a_heading_below_level_two_is_refused() {
        let error = render_pdf("t", "fr", &[Block::Heading(1, "x".into())], &font_bytes())
            .expect_err("H1 is reserved for the title");
        assert!(error.to_string().contains("niveau"));
    }
}
