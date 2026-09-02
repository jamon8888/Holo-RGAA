use serde::Deserialize;

const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/your-org/rgaa-cli/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[allow(dead_code)]
    body: String,
}

pub async fn check_for_updates(current_version: &str) -> anyhow::Result<Option<String>> {
    let client = reqwest::Client::builder()
        .user_agent("rgaa-cli/1.0")
        .build()?;

    let resp = client.get(GITHUB_RELEASES_API).send().await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let release: GithubRelease = resp.json().await?;

    let latest = release.tag_name.trim_start_matches('v');
    if latest > current_version {
        Ok(Some(format!(
            "v{} is available (you have v{}).\nDownload: {}",
            latest, current_version, release.html_url
        )))
    } else {
        Ok(None)
    }
}

pub fn prompt_update(msg: &str) {
    eprintln!("\n�更新 Available: {msg}\n");
}
