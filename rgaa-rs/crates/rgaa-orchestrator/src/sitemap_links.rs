//! RGAA 12.3 — links of the site-map page (#271).
//!
//! `HEAD` requests only, same host only, `robots.txt` respected, capped (default 50) and
//! counted apart from the LLM budget. The verdict is conservative: a broken link is a
//! `fail` with the list (URL, HTTP status); everything else is `review`, never `pass`,
//! because whether the map is representative (test 1) stays a human judgement.

use std::collections::BTreeSet;
use std::time::Duration;

use rgaa_core::CriterionStatus;

use crate::site_comparison::SiteCriterionObservation;

pub(crate) const DEFAULT_LINK_CAP: usize = 50;

/// Link cap from `RGAA_SITEMAP_LINK_CAP`, falling back to [`DEFAULT_LINK_CAP`].
pub(crate) fn link_cap_from_env() -> usize {
    std::env::var("RGAA_SITEMAP_LINK_CAP")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|cap| *cap > 0)
        .unwrap_or(DEFAULT_LINK_CAP)
}

/// What the HEAD pass found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LinkCheck {
    pub sitemap_url: String,
    /// Same-host links found on the map page, before the cap.
    pub candidates: usize,
    pub cap: usize,
    pub checked: usize,
    pub skipped_by_robots: usize,
    /// `(url, status)`; a transport error is reported as status 0.
    pub broken: Vec<(String, u16)>,
}

/// `true` when `robots_txt` forbids `path` for every user agent (`User-agent: *`).
/// Longest matching rule wins; `Allow` beats `Disallow` on a tie, as in the RFC 9309.
pub(crate) fn robots_disallows(robots_txt: &str, path: &str) -> bool {
    let mut in_star_group = false;
    let mut group_has_rules = false;
    let mut best: Option<(usize, bool)> = None; // (length, allowed)
    for raw in robots_txt.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
        match key.as_str() {
            "user-agent" => {
                if group_has_rules {
                    in_star_group = false;
                    group_has_rules = false;
                }
                if value == "*" {
                    in_star_group = true;
                }
            }
            "allow" | "disallow" if in_star_group => {
                group_has_rules = true;
                if value.is_empty() || !path.starts_with(value) {
                    continue;
                }
                let allowed = key == "allow";
                let better = match best {
                    None => true,
                    Some((len, was_allowed)) => {
                        value.len() > len || (value.len() == len && allowed && !was_allowed)
                    }
                };
                if better {
                    best = Some((value.len(), allowed));
                }
            }
            _ => {}
        }
    }
    matches!(best, Some((_, false)))
}

/// Same-host http(s) links, deduplicated in first-seen order, fragment removed.
pub(crate) fn same_host_links(sitemap_url: &str, links: &[String]) -> Vec<String> {
    let Ok(base) = reqwest::Url::parse(sitemap_url) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for link in links {
        let Ok(mut url) = reqwest::Url::parse(link) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https") || url.host_str() != base.host_str() {
            continue;
        }
        url.set_fragment(None);
        let key = url.to_string();
        if key != base.as_str() && seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out
}

/// Check at most `cap` links with `HEAD`. `robots_txt` is the site's robots file when it
/// could be read; a link it forbids is skipped, not requested.
pub(crate) async fn check_links(
    client: &reqwest::Client,
    sitemap_url: &str,
    links: &[String],
    cap: usize,
    robots_txt: Option<&str>,
) -> LinkCheck {
    let candidates = same_host_links(sitemap_url, links);
    let mut check = LinkCheck {
        sitemap_url: sitemap_url.to_string(),
        candidates: candidates.len(),
        cap,
        ..LinkCheck::default()
    };
    for link in candidates.iter().take(cap) {
        if let (Some(robots), Ok(url)) = (robots_txt, reqwest::Url::parse(link)) {
            if robots_disallows(robots, url.path()) {
                check.skipped_by_robots += 1;
                continue;
            }
        }
        check.checked += 1;
        match client.head(link).send().await {
            Ok(response)
                if response.status().is_client_error() || response.status().is_server_error() =>
            {
                check
                    .broken
                    .push((link.clone(), response.status().as_u16()));
            }
            Ok(_) => {}
            Err(_) => check.broken.push((link.clone(), 0)),
        }
    }
    check
}

/// A client suited to link checking: short timeouts, no hidden redirect loops.
pub(crate) fn link_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("rgaa-rs-linkcheck")
        .build()
}

/// 12.3 verdict. `None` means no site-map page could be located or read.
pub(crate) fn verdict(check: Option<&LinkCheck>) -> SiteCriterionObservation {
    let (status, details, sampled) = match check {
        None => (
            CriterionStatus::NeedsReview,
            "page plan du site introuvable: à vérifier manuellement".to_string(),
            0,
        ),
        Some(check) => {
            let budget = format!(
                "{} lien(s) vérifié(s) par HEAD sur {} (plafond {}), {} ignoré(s) par robots.txt",
                check.checked, check.candidates, check.cap, check.skipped_by_robots
            );
            if check.broken.is_empty() {
                (
                    CriterionStatus::NeedsReview,
                    format!(
                        "{}: aucun lien cassé; représentativité du plan et intitulés des liens à vérifier ({budget})",
                        check.sitemap_url
                    ),
                    check.checked,
                )
            } else {
                let list: Vec<String> = check
                    .broken
                    .iter()
                    .map(|(url, status)| format!("{url} ({status})"))
                    .collect();
                (
                    CriterionStatus::Fail,
                    format!(
                        "{}: lien(s) cassé(s): {} ({budget})",
                        check.sitemap_url,
                        list.join(", ")
                    ),
                    check.checked,
                )
            }
        }
    };
    SiteCriterionObservation {
        criterion_id: "12.3",
        status,
        details,
        sampled_pages: sampled,
        failed_pages: 0,
        sample_complete: check.is_some_and(|c| c.candidates <= c.cap),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn robots_longest_match_and_allow_tie() {
        let robots = "User-agent: *\nDisallow: /private\nAllow: /private/open\n";
        assert!(robots_disallows(robots, "/private/x"));
        assert!(!robots_disallows(robots, "/private/open/page"));
        assert!(!robots_disallows(robots, "/public"));
        assert!(!robots_disallows("User-agent: bot\nDisallow: /\n", "/x"));
        assert!(!robots_disallows("User-agent: *\nDisallow:\n", "/x"));
    }

    #[test]
    fn links_are_same_host_deduplicated_and_exclude_the_map_itself() {
        let links = vec![
            "https://a.test/plan".to_string(),
            "https://a.test/x#top".to_string(),
            "https://a.test/x".to_string(),
            "https://other.test/y".to_string(),
            "mailto:me@a.test".to_string(),
        ];
        assert_eq!(
            same_host_links("https://a.test/plan", &links),
            vec!["https://a.test/x".to_string()]
        );
    }

    #[test]
    fn verdict_never_passes_and_fails_only_on_a_broken_link() {
        let none = verdict(None);
        assert_eq!(none.status, CriterionStatus::NeedsReview);
        let clean = verdict(Some(&LinkCheck {
            sitemap_url: "https://a.test/plan".into(),
            candidates: 3,
            cap: 50,
            checked: 3,
            ..LinkCheck::default()
        }));
        assert_eq!(clean.status, CriterionStatus::NeedsReview);
        let broken = verdict(Some(&LinkCheck {
            sitemap_url: "https://a.test/plan".into(),
            candidates: 3,
            cap: 50,
            checked: 3,
            broken: vec![("https://a.test/gone".into(), 404)],
            ..LinkCheck::default()
        }));
        assert_eq!(broken.status, CriterionStatus::Fail);
        assert!(broken.details.contains("https://a.test/gone (404)"));
        assert!(broken.details.contains("plafond 50"));
    }

    /// Minimal HTTP server: 404 for `/gone`, 200 otherwise, HEAD only is expected.
    async fn serve() -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let handle = tokio::spawn(async move {
            let mut requests = Vec::new();
            for _ in 0..4 {
                let Ok(Ok((mut socket, _))) =
                    tokio::time::timeout(Duration::from_secs(3), listener.accept()).await
                else {
                    break;
                };
                let mut buffer = [0u8; 1024];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let first = request.lines().next().unwrap_or("").to_string();
                let status = if first.contains("/gone") {
                    "404 Not Found"
                } else {
                    "200 OK"
                };
                let _ = socket
                    .write_all(
                        format!(
                            "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .await;
                requests.push(first);
            }
            requests
        });
        (base, handle)
    }

    #[tokio::test]
    async fn head_requests_find_the_broken_link_and_respect_the_cap() {
        let (base, server) = serve().await;
        let client = link_client().expect("client");
        let links: Vec<String> = ["ok", "gone", "second", "third"]
            .iter()
            .map(|p| format!("{base}/{p}"))
            .collect();
        let check = check_links(&client, &format!("{base}/plan"), &links, 3, None).await;
        assert_eq!(check.candidates, 4);
        assert_eq!(check.checked, 3, "the cap bounds the requests");
        assert_eq!(check.broken, vec![(format!("{base}/gone"), 404)]);
        let requests = server.await.expect("server");
        assert!(
            requests.iter().all(|r| r.starts_with("HEAD ")),
            "{requests:?}"
        );
        assert_eq!(requests.len(), 3);
    }

    #[tokio::test]
    async fn robots_forbidden_links_are_not_requested() {
        let (base, server) = serve().await;
        let client = link_client().expect("client");
        let links = vec![format!("{base}/private/a"), format!("{base}/ok")];
        let robots = "User-agent: *\nDisallow: /private\n";
        let check = check_links(&client, &format!("{base}/plan"), &links, 50, Some(robots)).await;
        assert_eq!((check.checked, check.skipped_by_robots), (1, 1));
        assert!(check.broken.is_empty());
        let requests = server.await.expect("server");
        assert_eq!(requests.len(), 1);
    }
}
