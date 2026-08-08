#!/usr/bin/env rust
//! Servo headless POC : capture DOM + CSSOM + screenshot
//! 
//! Run: cargo run --example servo_poc
//! Requires: servo = "0.4.0" in Cargo.toml

use servo::embedder_traits::{EmbedderScriptMsg, EmbedderScriptMsgPort};
use servo::compositor::CompositorProxy;
use servo::script::ScriptMsg;
use servo::servo_config::prefs::PREFS_MAP;
use servo::servo_config::opts::Options;
use servo::servo_geometry::DeviceIndependentPixel;
use servo::servo_url::ServoUrl;
use servo::style::media_queries::Device;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Servo Headless POC ===\n");
    
    // Config headless
    let mut opts = Options::default();
    opts.headless = true;
    opts.no_default_window = true;
    opts.window_size = Some((1280, 720).into());
    opts.output_screenshot = Some("screenshot.png".into());
    
    // Prefs pour batch
    PREFS_MAP.write().unwrap().set("dom.disable_beforeunload", true);
    PREFS_MAP.write().unwrap().set("network.http.connection-timeout", 30);
    
    println!("1. Initialisation Servo...");
    let (servo, event_loop) = servo::init(opts).await?;
    println!("   ✓ Servo initialisé");
    
    // Test sur 3 URLs réelles
    let test_urls = [
        "https://www.service-public.fr",
        "https://www.gouvernement.fr", 
        "https://www.legifrance.gouv.fr",
    ];
    
    for (i, url) in test_urls.iter().enumerate() {
        println!("\n2.{}: Test {} - {}", i+1, i+1, url);
        
        let servo_url = ServoUrl::parse(url).expect("URL valide");
        
        // Navigation
        let start = std::time::Instant::now();
        servo.load_url(servo_url, None).await?;
        let nav_time = start.elapsed();
        println!("   Navigation: {:.2}s", nav_time.as_secs_f32());
        
        // Attendre chargement (timeout 30s)
        let script_port = servo.get_script_port();
        let result = timeout(Duration::from_secs(30), async {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if servo.is_document_ready().await {
                    break;
                }
            }
        }).await;
        
        match result {
            Ok(_) => println!("   ✓ Document prêt"),
            Err(_) => println!("   ⚠ Timeout chargement"),
        }
        
        // Capture DOM (outerHTML)
        let dom_start = std::time::Instant::now();
        let dom_html = servo.get_document_outer_html().await?;
        println!("   DOM capturé: {} chars ({:.2}s)", dom_html.len(), dom_start.elapsed().as_secs_f32());
        
        // Capture CSSOM (computed styles)
        let css_start = std::time::Instant::now();
        let cssom = servo.get_computed_styles().await?;
        println!("   CSSOM capturé: {} règles ({:.2}s)", cssom.len(), css_start.elapsed().as_secs_f32());
        
        // Capture screenshot (si supporté)
        let shot_start = std::time::Instant::now();
        if let Some(path) = servo.take_screenshot().await {
            println!("   Screenshot: {} ({:.2}s)", path.display(), shot_start.elapsed().as_secs_f32());
        } else {
            println!("   Screenshot: non supporté dans cette version");
        }
        
        // Extraction infos clés pour RGAA
        println!("   --- Analyse RGAA rapide ---");
        analyse_rgaa_basics(&dom_html, &cssom);
    }
    
    println!("\n=== POC terminé ===");
    Ok(())
}

fn analyse_rgaa_basics(dom: &str, cssom: &serde_json::Value) {
    use std::collections::HashMap;
    
    // 1.1 Alternative textuelle images
    let img_count = dom.matches("<img").count();
    let img_alt_count = dom.matches("alt=\"").count() + dom.matches("alt='").count();
    println!("   Images: {} | Avec alt: {} | Sans alt: {}", img_count, img_alt_count, img_count.saturating_sub(img_alt_count));
    
    // 1.2 Images décoratives (alt vide ou role=presentation)
    let decorative = dom.matches("alt=\"\"").count() + dom.matches("alt=''").count() + dom.matches("role=\"presentation\"").count();
    println!("   Images décoratives détectées: {}", decorative);
    
    // 2.1 Cadres avec titre
    let iframe_count = dom.matches("<iframe").count();
    let iframe_title = dom.matches("title=\"").count(); // approximatif
    println!("   Iframes: {} | Avec title: {}", iframe_count, iframe_title);
    
    // 3.2 Contraste (nécessite CSSOM réel - placeholder)
    println!("   Contraste: analyse CSSOM requise");
    
    // 8.3 Langue page
    let has_lang = dom.contains("lang=\"") || dom.contains("lang='");
    println!("   Langue déclarée: {}", if has_lang { "OUI" } else { "NON" });
    
    // 8.5 Titre page
    let has_title = dom.contains("<title>") && dom.contains("</title>");
    println!("   Titre page: {}", if has_title { "OUI" } else { "NON" });
    
    // 9.1 Hiérarchie titres
    let h_counts: HashMap<&str, usize> = [
        ("h1", dom.matches("<h1").count()),
        ("h2", dom.matches("<h2").count()),
        ("h3", dom.matches("<h3").count()),
    ].into_iter().collect();
    println!("   Titres: h1={}, h2={}, h3={}", h_counts["h1"], h_counts["h2"], h_counts["h3"]);
    
    // 11.1 Étiquettes formulaires
    let label_count = dom.matches("<label").count();
    let input_count = dom.matches("<input").count();
    let aria_labelled = dom.matches("aria-labelledby=").count() + dom.matches("aria-label=").count();
    println!("   Inputs: {} | Labels: {} | ARIA labels: {}", input_count, label_count, aria_labelled);
    
    // 12.6 Landmarks
    let landmarks = ["main", "nav", "header", "footer", "aside", "section"].iter()
        .map(|tag| (tag, dom.matches(&format!("<{}", tag)).count()))
        .filter(|(_, c)| *c > 0)
        .collect::<Vec<_>>();
    println!("   Landmarks: {}", landmarks.iter().map(|(t,c)| format!("{}={}", t, c)).collect::<Vec<_>>().join(", "));
    
    // 12.7 Skip link
    let skip_link = dom.contains("skip-link") || dom.contains("aller au contenu") || dom.contains("skip to");
    println!("   Skip link: {}", if skip_link { "DÉTECTÉ" } else { "ABSENT" });
}