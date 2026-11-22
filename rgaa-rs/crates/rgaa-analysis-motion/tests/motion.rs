//! P-05 — cas des specimens générés par le code (spec §7).
//!
//! Les valeurs attendues viennent de la spec (wcAG 2.3.1 : 341×256 px,
//! 25 % de surface, plus de 3 flashs/s, R/(R+G+B) ≥ 0,8) et du texte RGAA,
//! jamais du calcul de l'implémentation.

use image::codecs::gif::GifEncoder;
use image::{Delay, Frame as ImageFrame, RgbaImage};
use rgaa_analysis::AnalysisPlugin;
use rgaa_analysis::{AxNode, AxTree, DomSnapshot, NoopResourceFetcher, PageCapture, PluginOptions};
use rgaa_analysis_motion::flash::{detect_flashs, FlashType};
use rgaa_analysis_motion::frames::{decode_gif, decode_video_avec, Frame};
use rgaa_analysis_motion::iris::{IrisOracle, OracleFlash};
use rgaa_analysis_motion::limites::detecte_limites;
use rgaa_analysis_motion::luminance::{red_metric, relative_luminance};
use rgaa_analysis_motion::mouvement::{a_controle_pause, analyse_mouvement};
use rgaa_analysis_motion::{MotionPlugin, AVERTISSEMENT, FENETTE_HAUTEUR, FENETTE_LARGEUR};
use std::path::Path;

/// GIF dont toute l'image (ou une zone) alterne deux couleurs à la fréquence
/// demandée. Les délais sont des multiples de 10 ms, unité du GIF.
fn gif_alternance(
    hz: u32,
    largeur: u32,
    hauteur: u32,
    zone: Option<(u32, u32, u32, u32)>,
    clair: [u8; 4],
    sombre: [u8; 4],
) -> Vec<u8> {
    let delay_ms = 1000 / (2 * hz);
    let mut octets = Vec::new();
    {
        let mut encodeur = GifEncoder::new_with_speed(&mut octets, 30);
        for index in 0..(4 * hz) {
            let couleur = if index % 2 == 0 { clair } else { sombre };
            let mut image = RgbaImage::from_pixel(largeur, hauteur, image::Rgba(sombre));
            if let Some((x, y, w, h)) = zone {
                for py in y..(y + h) {
                    for px in x..(x + w) {
                        image.put_pixel(px, py, image::Rgba(couleur));
                    }
                }
            } else {
                image = RgbaImage::from_pixel(largeur, hauteur, image::Rgba(couleur));
            }
            let image_frame =
                ImageFrame::from_parts(image, 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
            encodeur.encode_frame(image_frame).expect("frame GIF");
        }
    }
    octets
}

fn noir() -> [u8; 4] {
    [0, 0, 0, 255]
}

fn blanc() -> [u8; 4] {
    [255, 255, 255, 255]
}

fn rouge() -> [u8; 4] {
    [255, 0, 0, 255]
}

#[test]
fn la_luminance_relative_suit_la_formule_wcag() {
    assert_eq!(relative_luminance(0, 0, 0), 0.0);
    assert!((relative_luminance(255, 255, 255) - 1.0).abs() < 1e-6);
    // Valeur de référence WCAG : gris 128 → ~0,2158 (0,03928 et 12,92 inclus).
    assert!((relative_luminance(128, 128, 128) - 0.2158).abs() < 0.002);
}

#[test]
fn le_rouge_sature_est_ordonne_par_le_rapport_rouge_sur_total() {
    assert!(red_metric(255, 0, 0) >= 300.0); // (1-0-0)*320 = 320
    assert_eq!(red_metric(0, 0, 0), 0.0);
    assert_eq!(red_metric(100, 100, 100), 0.0); // R/(R+G+B) = 1/3
}

#[test]
fn un_gif_noir_blanc_plein_a_5_hz_donne_5_flashs_et_une_surface_de_1() {
    let octets = gif_alternance(5, 256, 192, None, blanc(), noir());
    let frames = decode_gif(&octets).expect("décodage GIF");
    let (general, _rouge) = detect_flashs(&frames);
    assert_eq!(general.max_par_s, 5, "{general:?}");
    assert!(general.surface > 0.99, "{general:?}");
    assert_eq!(general.fenetre[2], FENETTE_LARGEUR);
    assert_eq!(general.fenetre[3], FENETTE_HAUTEUR);
}

#[test]
fn le_meme_gif_a_2_hz_reste_sous_le_seuil_de_trois_flashs() {
    let octets = gif_alternance(2, 256, 192, None, blanc(), noir());
    let frames = decode_gif(&octets).expect("décodage GIF");
    let (general, _rouge) = detect_flashs(&frames);
    assert_eq!(general.max_par_s, 2, "{general:?}");
}

#[test]
fn une_zone_de_100_sur_100_a_5_hz_reste_sous_le_seuil_de_surface() {
    let octets = gif_alternance(5, 1024, 768, Some((100, 100, 100, 100)), blanc(), noir());
    let frames = decode_gif(&octets).expect("décodage GIF");
    let (general, _rouge) = detect_flashs(&frames);
    assert_eq!(general.max_par_s, 5, "{general:?}");
    // 10 000 px / (341*256) ≈ 0,11 < 0,25 : pas d'échec probable.
    assert!(general.surface < 0.25, "{general:?}");
}

#[test]
fn une_alternance_rouge_noir_a_4_hz_donne_un_red_flash() {
    let octets = gif_alternance(4, 256, 192, None, rouge(), noir());
    let frames = decode_gif(&octets).expect("décodage GIF");
    let (_general, rouge) = detect_flashs(&frames);
    assert_eq!(rouge.max_par_s, 4, "{rouge:?}");
    assert!(rouge.surface > 0.9, "{rouge:?}");
    assert_eq!(detect_flashs(&frames).1.type_, FlashType::Rouge);
}

#[test]
fn la_borne_est_strictement_superieure_a_trois_flashs_par_seconde() {
    // Prototype ffmpeg (RESULTS.md) : 3 vs 4 flashs/s est le cas aux bornes
    // qui distingue une rampe d'un vrai défaut.
    fn alternance(hz: u32) -> Vec<Frame> {
        let total = 2 * hz * 2;
        (0..total)
            .map(|index| {
                let couleur = if index % 2 == 0 { blanc() } else { noir() };
                Frame::de_couleur(64, 48, couleur, index as f64 / (2 * hz) as f64)
            })
            .collect()
    }
    let (trois, _) = detect_flashs(&alternance(3));
    let (quatre, _) = detect_flashs(&alternance(4));
    assert_eq!(trois.max_par_s, 3, "{trois:?}");
    assert_eq!(quatre.max_par_s, 4, "{quatre:?}");
    assert!(trois.surface <= 0.25, "{trois:?}");
    assert!(quatre.surface > 0.25, "{quatre:?}");
}

#[test]
fn une_rampe_lente_n_est_pas_un_flash() {
    // 1 cycle sinusoïdal par seconde : un seul « flash » par seconde au plus.
    let frames: Vec<Frame> = (0..60)
        .map(|index| {
            let t = index as f64 / 30.0;
            let value = (((1.0 - (2.0 * std::f64::consts::PI * t).cos()) / 2.0) * 255.0) as u8;
            Frame::de_couleur(64, 48, [value, value, value, 255], t)
        })
        .collect();
    let (general, _rouge) = detect_flashs(&frames);
    assert!(general.max_par_s <= 1, "{general:?}");
}

#[test]
fn un_mouvement_de_plus_de_cinq_sans_controle_est_suspect() {
    let frames: Vec<Frame> = (0..180)
        .map(|index| {
            let cle = if index % 2 == 0 {
                [10, 20, 30, 255]
            } else {
                [200, 210, 220, 255]
            };
            Frame::de_couleur(64, 48, cle, index as f64 / 30.0)
        })
        .collect();
    let mouvements = analyse_mouvement(&frames, None, ".carousel");
    assert_eq!(mouvements.len(), 1, "{mouvements:?}");
    assert!(mouvements[0].duree_s > 5.0, "{mouvements:?}");
    assert!(!mouvements[0].controle_pause);
    assert!(!mouvements[0].reduced_motion_respecte);
}

#[test]
fn un_bouton_pause_de_l_arbre_ax_est_reconnu() {
    let ax = AxTree {
        nodes: vec![AxNode {
            role: "button".to_owned(),
            name: "Mettre en pause le carrousel".to_owned(),
        }],
    };
    assert!(a_controle_pause(Some(&ax)));
    assert!(!a_controle_pause(None));
}

#[test]
fn un_meta_refresh_de_30_s_est_une_preuve_mesuree() {
    let dom =
        r#"{"nodes":[{"tag":"meta","attrs":{"http-equiv":"refresh","content":"30"},"text":""}]}"#;
    let limites = detecte_limites(dom).expect("limites");
    assert_eq!(limites.len(), 1, "{limites:?}");
    assert_eq!(limites[0].type_, "meta-refresh");
    assert_eq!(limites[0].delai_s, 30.0);
    assert!(!limites[0].mecanisme_prolongation);
}

#[test]
fn un_timer_de_redirection_est_releve_en_secondes() {
    let dom = r#"{"nodes":[{"tag":"script","attrs":{},"text":"setTimeout(function(){location.assign('/next')}, 5000);"}]}"#;
    let limites = detecte_limites(dom).expect("limites");
    assert_eq!(limites.len(), 1, "{limites:?}");
    assert_eq!(limites[0].type_, "timer-redirection");
    assert_eq!(limites[0].delai_s, 5.0);
}

#[tokio::test]
async fn le_rapport_porte_l_avertissement_et_aucun_statut() {
    let dom =
        r#"{"nodes":[{"tag":"meta","attrs":{"http-equiv":"refresh","content":"30"},"text":""}]}"#;
    let snapshot = DomSnapshot::new(dom);
    let resources = NoopResourceFetcher;
    let page = PageCapture {
        url: "https://example.test/page",
        dom_snapshot: &snapshot,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources: &resources,
        lang_declared: Some("fr"),
    };
    let preuves = MotionPlugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("analyse");
    assert!(!preuves.is_empty());
    for preuve in &preuves {
        assert_eq!(preuve.mesures["avertissement"], AVERTISSEMENT);
    }
    let json = serde_json::to_string(&preuves).expect("json");
    assert!(!json.contains("\"status\""), "R-P1");
    assert!(!json.contains("\"verdict\""), "R-P1");
}

#[test]
fn ffmpeg_absent_est_une_preuve_manquante() {
    let erreur = decode_video_avec(
        Path::new("/aucun/chemin/vers/ffmpeg"),
        "https://example.test/video.mp4",
    )
    .expect_err("ffmpeg absent");
    let message = erreur.to_string();
    assert!(message.contains("preuve manquante"), "{message}");
    assert!(message.contains("ffmpeg"), "{message}");
}

#[test]
fn iris_absent_laisse_un_point_d_extension_sans_echouer() {
    let oracle = IrisOracle::avec_commande(None);
    assert!(!oracle.disponible());
    assert!(oracle.compte_flashs(&[]).expect("pas d'oracle").is_none());
    assert!(IrisOracle::avec_commande(Some("iris-inexistant".to_owned())).disponible());
}

struct FakeFetcher {
    gif: Vec<u8>,
}

#[async_trait::async_trait]
impl rgaa_analysis::ResourceFetcher for FakeFetcher {
    async fn fetch(&self, url: &str) -> rgaa_analysis::Result<Vec<u8>> {
        if url == GIF_URL {
            Ok(self.gif.clone())
        } else {
            Err(rgaa_analysis::analysis_error(
                "motion",
                format!("ressource absente : {url}"),
            ))
        }
    }
}

const GIF_URL: &str = "https://example.test/flash.gif";

#[tokio::test]
async fn le_plugin_rend_une_preuve_estimee_pour_un_gif_sans_controle() {
    let octets = gif_alternance(5, 128, 96, None, blanc(), noir());
    let dom = format!(r#"{{"nodes":[{{"tag":"img","attrs":{{"src":"{GIF_URL}"}},"text":""}}]}}"#);
    let snapshot = DomSnapshot::new(dom);
    let fetcher = FakeFetcher { gif: octets };
    let page = PageCapture {
        url: "https://example.test/page",
        dom_snapshot: &snapshot,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources: &fetcher,
        lang_declared: None,
    };
    let preuves = MotionPlugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("analyse GIF");
    assert_eq!(preuves.len(), 1, "{preuves:?}");
    let preuve = &preuves[0];
    assert_eq!(preuve.tests, vec!["13.7".to_owned(), "13.8".to_owned()]);
    assert_eq!(preuve.portee, rgaa_analysis::Portee::Estimee);
    assert_eq!(
        preuve.sujet,
        rgaa_analysis::Subject::Media {
            url: GIF_URL.into()
        }
    );
    assert!(matches!(
        preuve.indice,
        Some(rgaa_analysis::Indice::Suspect { .. })
    ));
    assert_eq!(preuve.mesures["source"], format!("gif:{GIF_URL}"));
    assert_eq!(preuve.mesures["flash_general"]["max_par_s"], 5);
    assert_eq!(preuve.mesures["avertissement"], AVERTISSEMENT);
}

#[test]
fn le_schema_des_mesures_ne_contient_aucun_statut() {
    let schema = rgaa_analysis_motion::mesures::schema().expect("schema");
    let texte = schema.to_string();
    assert!(!texte.contains("\"status\""), "R-P1");
    assert!(!texte.contains("\"verdict\""), "R-P1");
}
