//! 1.2 — « Chaque image **de décoration** est-elle correctement ignorée ? »
//!
//! Les six tests de 1.2 ne s'appliquent qu'aux images **de décoration**. Or rien, dans le
//! balisage, ne dit qu'une image est décorative : c'est un jugement éditorial. Une sonde qui
//! rend `pass` ou `fail` sur le seul DOM affirme donc ce qu'elle ne peut pas savoir.
//!
//! Ce que la sonde doit faire, pour un audit opposable :
//!   * `fail` sur la **contradiction décidable** — une image déclarée décorative (`alt=""`,
//!     `role="presentation"`, `aria-hidden="true"`) qui porte malgré tout une alternative
//!     (`title`, `aria-label`, `aria-labelledby`) : 1.2.1 l'interdit explicitement ;
//!   * `review` partout ailleurs : la nature de l'image n'est pas établie.
//!
//! Elle ne doit **jamais** rendre `pass` sur le balisage seul, et 1.2 ne peut pas être
//! déclaré en couverture complète : la sonde ne regarde que `<img>`, un porteur sur six.
//!
//! Lancer à la main avec `RUN_E2E=1`.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use rgaa_core::CriterionStatus;
use rgaa_obscura::ObscuraBridge;
use rgaa_rules::GapFixRules;

fn serve(dir: PathBuf) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let name = request
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .trim_start_matches('/')
                .to_string();
            let body = if name.contains("..") || name.contains('/') {
                None
            } else {
                std::fs::read(dir.join(&name)).ok()
            };
            let _ = match body {
                Some(b) => {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n",
                        b.len()
                    );
                    stream
                        .write_all(head.as_bytes())
                        .and_then(|()| stream.write_all(&b))
                }
                None => stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                ),
            };
        }
    });
    Ok(port)
}

/// Le statut que la sonde `gapfix-1-2` pose pour 1.2, tel que le pipeline le matérialise.
/// `None` = aucun verdict, le critère retombe sur le repli déclaré.
async fn probe_1_2(url: &str) -> Option<CriterionStatus> {
    probe_1_2_detailed(url).await.map(|(status, _)| status)
}

/// Le statut et le nombre d'éléments cités comme preuve.
async fn probe_1_2_detailed(url: &str) -> Option<(CriterionStatus, usize)> {
    let binary = ObscuraBridge::new().binary_path().to_string();
    let snippets: HashMap<String, String> = GapFixRules::snippets()
        .iter()
        .filter(|(id, _)| id.as_str() == "1.2")
        .map(|(id, s)| (id.clone(), (*s).to_string()))
        .collect();
    let mut probes = ObscuraBridge::run_gap_fix_batch(binary, vec![url.to_string()], snippets, 1)
        .await
        .expect("run_gap_fix_batch");
    let raw = probes.remove(url).unwrap_or_default();
    GapFixRules::parse_results(&raw).get("1.2").map(|result| {
        let nodes = result
            .violations
            .first()
            .map_or(0, |violation| violation.nodes_affected);
        (result.status.clone(), nodes)
    })
}

#[tokio::test]
async fn criterion_1_2_judges_only_what_the_markup_settles() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_1_2_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    // Témoin : la contradiction est décidable depuis le balisage, la sonde doit la voir.
    // Sans ce témoin, « pas de verdict » ailleurs ne prouverait rien.
    let contradiction = format!("http://127.0.0.1:{port}/coverage-1-2-contradiction.html");
    let status = probe_1_2(&contradiction).await;
    eprintln!("alt=\"\" + title : sonde -> {status:?}");
    assert_eq!(
        status,
        Some(CriterionStatus::Fail),
        "une image déclarée décorative qui porte un title fournit une alternative : \
         1.2.1 l'interdit, et c'est lisible dans le balisage"
    );

    // Nature indécidable : la sonde ne doit rien affirmer.
    let undecidable = format!("http://127.0.0.1:{port}/coverage-1-2-undecidable.html");
    let status = probe_1_2(&undecidable).await;
    eprintln!("nature indécidable : sonde -> {status:?}");
    assert_ne!(
        status,
        Some(CriterionStatus::Pass),
        "la sonde conclut à la conformité de 1.2 alors que rien ne dit que ces images \
         sont décoratives"
    );
    assert_ne!(
        status,
        Some(CriterionStatus::Fail),
        "la sonde conclut à la non-conformité de 1.2 alors qu'une image sans alt est \
         d'abord un manquement à 1.1, et qu'une image à alt non vide peut être informative"
    );
}

/// Les six porteurs de 1.2, chacun déclaré décoratif et portant malgré tout une alternative.
/// Un seul `fail` ne suffirait pas : le compte de preuves doit valoir six, sinon un porteur
/// passe au travers — le défaut même qui a fait ouvrir #366.
#[tokio::test]
async fn every_carrier_of_criterion_1_2_is_examined() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_1_2_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    let all = format!("http://127.0.0.1:{port}/coverage-1-2-contradictions-all.html");
    let detailed = probe_1_2_detailed(&all).await;
    eprintln!("six contradictions : sonde -> {detailed:?}");
    let (status, nodes) = detailed.expect("la sonde doit rendre un résultat pour 1.2");
    assert_eq!(
        status,
        CriterionStatus::Fail,
        "six éléments déclarés décoratifs portent une alternative"
    );
    assert_eq!(
        nodes, 6,
        "chaque porteur de 1.2 doit être examiné : <img>, <area>, <object>, <svg>, \
         <canvas>, <embed> — obtenu {nodes} au lieu de 6"
    );

    // Les mêmes porteurs, correctement masqués : aucune contradiction, mais rien ne dit
    // qu'ils sont décoratifs. Ni `fail`, ni `pass`.
    let hidden = format!("http://127.0.0.1:{port}/coverage-1-2-correctly-hidden.html");
    let status = probe_1_2(&hidden).await;
    eprintln!("six porteurs masqués : sonde -> {status:?}");
    assert_eq!(
        status,
        Some(CriterionStatus::NeedsReview),
        "sans contradiction, le caractère décoratif reste à établir par un humain"
    );
}
