//! Comportement du plugin P-03 sans modèle, avec CAPTCHA et avec des
//! backend de test (point d'extension pour tesseract / oar-ocr).

use async_trait::async_trait;
use image::{DynamicImage, Rgba, RgbaImage};
use rgaa_analysis::{
    run_plugin, AnalysisPlugin, NoopResourceFetcher, PageCapture, PluginOptions, Portee,
    ResourceFetcher, Subject,
};
use rgaa_analysis_images::{
    cache::DecisionCache,
    clip::{ClipBackend, ClipMesures},
    inventory::{ImageNode, Snapshot},
    ocr::{OcrBackend, OcrMesures},
    ImagesPlugin,
};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn png() -> Vec<u8> {
    let mut image = RgbaImage::new(64, 64);
    for y in 0..64 {
        for x in 0..64 {
            image.put_pixel(x, y, Rgba([x as u8, y as u8, 128, 255]));
        }
    }
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("png");
    bytes.into_inner()
}

struct PngFetcher {
    bytes: Vec<u8>,
}

#[async_trait]
impl ResourceFetcher for PngFetcher {
    async fn fetch(&self, _url: &str) -> rgaa_analysis::Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }
}

struct FakeOcr {
    mesures: OcrMesures,
    appels: Arc<AtomicUsize>,
}

impl OcrBackend for FakeOcr {
    fn id(&self) -> &str {
        "ocr-faux"
    }

    fn analyze(&self, _image: &DynamicImage) -> rgaa_analysis::Result<OcrMesures> {
        self.appels.fetch_add(1, Ordering::SeqCst);
        Ok(self.mesures.clone())
    }
}

struct ExplodingOcr;

impl OcrBackend for ExplodingOcr {
    fn id(&self) -> &str {
        "ocr-interdit"
    }

    fn analyze(&self, _image: &DynamicImage) -> rgaa_analysis::Result<OcrMesures> {
        panic!("un CAPTCHA ne doit jamais être OCRisé")
    }
}

struct FakeClip {
    mesures: ClipMesures,
}

impl ClipBackend for FakeClip {
    fn id(&self) -> &str {
        "clip-faux"
    }

    fn analyze(
        &self,
        _image: &DynamicImage,
        _alt: &str,
        _alts: &[String],
    ) -> rgaa_analysis::Result<ClipMesures> {
        Ok(self.mesures.clone())
    }
}

fn snapshot_json(node: ImageNode) -> String {
    serde_json::to_string(&Snapshot {
        page: "https://exemple.test".to_owned(),
        images: vec![node],
    })
    .expect("snapshot")
}

fn image_node() -> ImageNode {
    ImageNode {
        type_noeud: "img".to_owned(),
        src: Some("https://exemple.test/chat.png".to_owned()),
        alt: Some("chat roux sur un canapé".to_owned()),
        ..Default::default()
    }
}

#[test]
fn sans_modele_les_regles_deterministes_tournent_et_ocr_clip_sont_preuve_manquante() {
    let repertoire = tempfile::tempdir().expect("temp");
    let plugin = ImagesPlugin::new().with_models_dir(repertoire.path());
    let dom = rgaa_analysis::DomSnapshot::new(snapshot_json(ImageNode {
        alt: Some("DSC_0042.JPG".to_owned()),
        src: Some("https://exemple.test/DSC_0042.JPG".to_owned()),
        ..Default::default()
    }));
    let resources = NoopResourceFetcher;
    let page = PageCapture {
        url: "https://exemple.test",
        dom_snapshot: &dom,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources: &resources,
        lang_declared: Some("fr"),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let preuves = runtime
        .block_on(plugin.analyze(&page, &PluginOptions::default()))
        .expect("preuves");

    let deterministe = preuves
        .iter()
        .find(|preuve| preuve.portee == Portee::Mesuree)
        .expect("preuve mesurée");
    assert_eq!(deterministe.sujet, Subject::Page);
    assert!(deterministe.mesures.to_string().contains("alt-nom-fichier"));

    for nom in ["ocr", "clip"] {
        let preuve = preuves
            .iter()
            .find(|preuve| preuve.portee == Portee::Estimee && preuve.mesures.get(nom).is_some())
            .unwrap_or_else(|| panic!("preuve {nom} manquante"));
        assert!(
            preuve.mesures.to_string().contains("preuve manquante"),
            "{}",
            preuve.mesures
        );
    }
}

#[tokio::test]
async fn run_plugin_ne_bloque_pas_sur_les_modeles_absents() {
    let repertoire = tempfile::tempdir().expect("temp");
    let plugin = ImagesPlugin::new().with_models_dir(repertoire.path());
    let preuves = run_plugin(&plugin, "https://exemple.test", &serde_json::json!({}))
        .await
        .expect("les règles déterministes doivent tourner");
    // Le chemin de convenance P-00 n'a pas encore de DOM : inventaire vide,
    // mais surtout aucune erreur « modèle absent » (contrairement au défaut).
    assert!(preuves.is_empty(), "snapshot P-00 vide : {preuves:?}");
}

#[tokio::test]
async fn un_captcha_n_est_jamais_ocrisé() {
    let appels = Arc::new(AtomicUsize::new(0));
    let ocr = FakeOcr {
        mesures: OcrMesures {
            texte: "ignoré".to_owned(),
            couverture: 0.9,
            confiance: None,
            moteur: "ocrs".to_owned(),
        },
        appels: appels.clone(),
    };
    let _ = ExplodingOcr;
    let plugin = ImagesPlugin::with_backends(Some(Arc::new(ocr)), None);
    let dom = rgaa_analysis::DomSnapshot::new(snapshot_json(ImageNode {
        classes: Some("g-recaptcha".to_owned()),
        captcha: true,
        src: Some("https://exemple.test/captcha.png".to_owned()),
        ..Default::default()
    }));
    let resources = PngFetcher { bytes: png() };
    let page = PageCapture {
        url: "https://exemple.test",
        dom_snapshot: &dom,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources: &resources,
        lang_declared: None,
    };
    let preuves = plugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("preuves");
    assert_eq!(appels.load(Ordering::SeqCst), 0);
    assert!(preuves.iter().any(|p| p.tests.iter().any(|t| t == "1.4.1")));
    assert!(!preuves.iter().any(|p| p.mesures.get("ocr").is_some()));
}

#[tokio::test]
async fn ocr_texte_suspect_sur_une_image_texte() {
    let appels = Arc::new(AtomicUsize::new(0));
    let ocr = FakeOcr {
        mesures: OcrMesures {
            texte: "SOLDES -50% DANS TOUS LES RAYONS".to_owned(),
            couverture: 0.62,
            confiance: None,
            moteur: "ocrs".to_owned(),
        },
        appels: appels.clone(),
    };
    let plugin = ImagesPlugin::with_backends(Some(Arc::new(ocr)), None);
    let dom = rgaa_analysis::DomSnapshot::new(snapshot_json(ImageNode {
        src: Some("https://exemple.test/banniere.png".to_owned()),
        alt: Some("".to_owned()),
        ..Default::default()
    }));
    let resources = PngFetcher { bytes: png() };
    let page = PageCapture {
        url: "https://exemple.test",
        dom_snapshot: &dom,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources: &resources,
        lang_declared: None,
    };
    let preuves = plugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("preuves");
    let ocr_preuve = preuves
        .iter()
        .find(|p| p.mesures.get("ocr").is_some())
        .expect("preuve OCR");
    assert_eq!(appels.load(Ordering::SeqCst), 1);
    assert!(ocr_preuve.tests.iter().any(|t| t == "1.8.1"));
    assert!(matches!(
        ocr_preuve.indice,
        Some(rgaa_analysis::Indice::Suspect { .. })
    ));
}

#[tokio::test]
async fn similarite_clip_under_chance_anchor_is_suspect() {
    let clip = FakeClip {
        mesures: ClipMesures {
            classe: "une photo".to_owned(),
            score: 0.71,
            similarite_alt: 0.19,
            ancre_hasard: 0.22,
        },
    };
    let plugin = ImagesPlugin::with_backends(None, Some(Arc::new(clip)));
    let dom = rgaa_analysis::DomSnapshot::new(snapshot_json(image_node()));
    let resources = PngFetcher { bytes: png() };
    let page = PageCapture {
        url: "https://exemple.test",
        dom_snapshot: &dom,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources: &resources,
        lang_declared: None,
    };
    let preuves = plugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("preuves");
    let clip_preuve = preuves
        .iter()
        .find(|p| p.mesures.get("clip").is_some())
        .expect("preuve CLIP");
    assert!(clip_preuve.tests.iter().any(|t| t == "1.3.1"));
    assert!(matches!(
        clip_preuve.indice,
        Some(rgaa_analysis::Indice::Suspect { .. })
    ));
}

#[tokio::test]
async fn logo_repete_sur_41_pages_n_est_analyse_qu_une_fois() {
    let appels = Arc::new(AtomicUsize::new(0));
    let ocr = FakeOcr {
        mesures: OcrMesures {
            texte: "LOGO".to_owned(),
            couverture: 0.1,
            confiance: None,
            moteur: "ocr-faux".to_owned(),
        },
        appels: appels.clone(),
    };
    let plugin = ImagesPlugin::with_backends(Some(Arc::new(ocr)), None);
    let octets = png();
    for page_numero in 0..41 {
        let url = format!("https://exemple.test/page-{page_numero}");
        let dom = rgaa_analysis::DomSnapshot::new(snapshot_json(ImageNode {
            src: Some("https://exemple.test/logo.png".to_owned()),
            alt: Some("Logo de l'association".to_owned()),
            texte_adjacent: Some("Association".to_owned()),
            ..Default::default()
        }));
        let resources = PngFetcher {
            bytes: octets.clone(),
        };
        let page = PageCapture {
            url: &url,
            dom_snapshot: &dom,
            ax_tree: None,
            screenshot: None,
            dpr: 1.0,
            resources: &resources,
            lang_declared: None,
        };
        let preuves = plugin
            .analyze(&page, &PluginOptions::default())
            .await
            .expect("preuves");
        let réutilisé = preuves
            .iter()
            .any(|p| p.mesures.get("reutilisation_cache").is_some());
        if page_numero == 0 {
            assert!(!réutilisé, "la première page doit être analysée");
            assert!(preuves.iter().any(|p| p
                .mesures
                .get("ocr")
                .is_some_and(|ocr| ocr.get("texte").is_some())));
        } else {
            assert!(réutilisé, "page {page_numero} : cache non utilisé");
        }
    }
    assert_eq!(appels.load(Ordering::SeqCst), 1);
    let cache = plugin.cache();
    assert_eq!(
        cache.lock().expect("verrou").occurrences(
            rgaa_analysis_images::phash::calculer(&image::load_from_memory(&octets).expect("png"))
                .expect("pHash")
        ),
        41
    );
}

#[test]
fn decision_cache_compte_les_occurrences_par_phash() {
    let mut cache = DecisionCache::new();
    let base: u64 = 0b1010_1010;
    for i in 0..41 {
        cache.enregistrer(
            base,
            "logo",
            "contexte",
            &format!("https://exemple.test/page-{i}"),
        );
    }
    assert_eq!(cache.occurrences(base), 41);
    assert!(rgaa_analysis_images::phash::sont_des_copies(base, base));
    let cle = DecisionCache::cle(base, "logo", "contexte");
    assert!(cache.decision(&cle).is_none());
    cache.enregistrer_decision(&cle, "conforme");
    assert_eq!(cache.decision(&cle), Some("conforme"));
}

#[test]
fn schema_des_mesures_sans_statut_ni_verdict() {
    let schema = rgaa_analysis_images::schema::mesures_schema().expect("schema");
    let texte = schema.to_string();
    assert!(!texte.contains("\"status\""));
    assert!(!texte.contains("\"verdict\""));
    assert!(texte.contains("alt-nom-fichier") || texte.contains("regles"));
}
