use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn};

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

pub struct PlaywrightBridge;

impl PlaywrightBridge {
    pub fn new() -> Self {
        Self
    }

    pub async fn run_axe(&self, url: &str) -> Result<String, String> {
        let url = escape_js_string(url);
        let script = format!(
            r#"
const {{ chromium }} = require('playwright');
(async () => {{
  const browser = await chromium.launch({{ headless: true }});
  const page = await browser.newPage();
  await page.goto('{url}', {{ waitUntil: 'networkidle', timeout: 30000 }});
  await page.addScriptTag({{ path: require.resolve('axe-core') }});
  const results = await page.evaluate(async () => {{
    await axe.run();
    return axe.getRules().map(r => r.ruleId);
  }});
  const violations = await page.evaluate(async () => {{
    const axeResults = await axe.run();
    return axeResults.violations;
  }});
  console.log(JSON.stringify(violations));
  await browser.close();
}})();
"#
        );
        self.run_node_script(&script).await
    }

    pub async fn run_gap_fix(
        &self,
        url: &str,
        snippets: &HashMap<String, &str>,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let url = escape_js_string(url);
        let mut results = HashMap::new();
        for (criterion_id, snippet) in snippets {
            let script = format!(
                r#"
const {{ chromium }} = require('playwright');
(async () => {{
  const browser = await chromium.launch({{ headless: true }});
  const page = await browser.newPage();
  await page.goto('{url}', {{ waitUntil: 'networkidle', timeout: 30000 }});
  try {{
    const result = await page.evaluate(() => {{
      {snippet}
      return {{ success: true }};
    }});
    console.log(JSON.stringify(result));
  }} catch (e) {{
    console.log(JSON.stringify({{ success: false, error: e.message }}));
  }}
  await browser.close();
}})();
"#
            );
            let output = self.run_node_script(&script).await?;
            let value: serde_json::Value =
                serde_json::from_str(&output).map_err(|e| e.to_string())?;
            results.insert(criterion_id.clone(), value);
        }
        Ok(results)
    }

    pub async fn run_interaction(
        &self,
        url: &str,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let url = escape_js_string(url);
        let js_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("js")
            .join("interaction.js");
        let js_code = tokio::fs::read_to_string(&js_path)
            .await
            .map_err(|e| format!("Failed to read interaction.js: {e}"))?;

        let script = format!(
            r#"
const {{ chromium }} = require('playwright');
{js_code}
(async () => {{
  const browser = await chromium.launch({{ headless: true }});
  const page = await browser.newPage();
  const results = await runInteractionTests(page, '{url}');
  console.log(JSON.stringify(results));
  await browser.close();
}})();
"#
        );
        let output = self.run_node_script(&script).await?;
        let value: serde_json::Value =
            serde_json::from_str(&output).map_err(|e| e.to_string())?;
        let map: HashMap<String, serde_json::Value> =
            serde_json::from_value(value).map_err(|e| e.to_string())?;
        Ok(map)
    }

    pub async fn extract_page_context(&self, url: &str) -> Result<serde_json::Value, String> {
        let url = escape_js_string(url);
        let script = format!(
            r#"
const {{ chromium }} = require('playwright');
(async () => {{
  const browser = await chromium.launch({{ headless: true }});
  const page = await browser.newPage();
  await page.goto('{url}', {{ waitUntil: 'networkidle', timeout: 30000 }});
  const context = await page.evaluate(() => {{
    const title = document.title;
    const lang = document.documentElement.lang;
    const headings = Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6'))
      .map(h => ({{ level: parseInt(h.tagName[1]), text: h.textContent.trim() }}));
    const landmarks = Array.from(document.querySelectorAll('header, nav, main, aside, footer, [role="banner"], [role="navigation"], [role="main"], [role="complementary"], [role="contentinfo"]'))
      .map(el => ({{ tag: el.tagName.toLowerCase(), role: el.getAttribute('role'), label: el.getAttribute('aria-label') }}));
    const images = Array.from(document.querySelectorAll('img'))
      .map(img => ({{ src: img.src, alt: img.alt, hasAlt: img.hasAttribute('alt') }}));
    const forms = Array.from(document.querySelectorAll('form'))
      .map(form => ({{ action: form.action, inputs: Array.from(form.querySelectorAll('input, select, textarea')).length }}));
    return {{ title, lang, headings, landmarks, images, forms }};
  }});
  console.log(JSON.stringify(context));
  await browser.close();
}})();
"#
        );
        let output = self.run_node_script(&script).await?;
        serde_json::from_str(&output).map_err(|e| e.to_string())
    }

    async fn run_node_script(&self, script: &str) -> Result<String, String> {
        info!("Running Node.js script");
        let output = timeout(Duration::from_secs(60), async {
            Command::new("node")
                .arg("-e")
                .arg(script)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to spawn node: {e}"))
        })
        .await
        .map_err(|_| "Node.js script timed out after 60s".to_string())??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "Node.js script failed");
            return Err(format!("Node script failed: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }
}
