//! Static registry of native plugins, merged with discovered sidecars (P-00 §4).

use crate::sidecar;
use crate::types::PluginInfo;
use crate::AnalysisPlugin;
use indexmap::IndexMap;
use std::sync::OnceLock;

static REGISTRY: OnceLock<Vec<&'static dyn AnalysisPlugin>> = OnceLock::new();

/// Native plugins compiled into this binary. Each entry is feature-gated;
/// P-01..P-08 add their crate here when they land.
fn native_plugins() -> Vec<&'static dyn AnalysisPlugin> {
    // Adding a native plugin is one line here; the array is empty in a
    // `--no-default-features` build (acceptance criterion 1).
    const PLUGINS: &[&dyn AnalysisPlugin] = &[
        #[cfg(feature = "plugin-echo")]
        &crate::plugin::EchoPlugin,
    ];
    PLUGINS.to_vec()
}

/// All available plugins, in stable identifier order. Sidecars are
/// indistinguishable from native plugins to callers.
pub fn registry() -> &'static [&'static dyn AnalysisPlugin] {
    REGISTRY.get_or_init(|| {
        let mut plugins = native_plugins();
        for sidecar in sidecar::discover_default() {
            plugins.push(Box::leak(Box::new(sidecar)));
        }
        plugins.sort_by_key(|plugin| plugin.id());
        plugins
    })
}

pub fn plugin_by_id(id: &str) -> Option<&'static dyn AnalysisPlugin> {
    registry().iter().copied().find(|plugin| plugin.id() == id)
}

/// RGAA tests served by more than one plugin — `rgaa plugins list` flags them
/// (acceptance criterion 2). Insertion order follows the registry.
pub fn duplicate_tests(plugins: &[&dyn AnalysisPlugin]) -> IndexMap<String, Vec<String>> {
    let mut providers: IndexMap<String, Vec<String>> = IndexMap::new();
    for plugin in plugins {
        for test in plugin.tests() {
            providers
                .entry((*test).to_owned())
                .or_default()
                .push(plugin.id().to_owned());
        }
    }
    providers.retain(|_, ids| ids.len() > 1);
    providers
}

/// Duplicate tests across the whole registry.
pub fn registry_duplicate_tests() -> IndexMap<String, Vec<String>> {
    duplicate_tests(registry())
}

pub fn list() -> Vec<PluginInfo> {
    registry().iter().map(|plugin| plugin.info()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PageCapture;
    use crate::error::Result;
    use crate::types::{PluginOptions, Preuve};
    use async_trait::async_trait;

    struct SecondEcho;

    #[async_trait]
    impl AnalysisPlugin for SecondEcho {
        fn id(&self) -> &'static str {
            "second-echo"
        }
        fn version(&self) -> &'static str {
            "0.1.0"
        }
        fn tests(&self) -> &'static [&'static str] {
            &["1.1.1"]
        }
        async fn analyze(
            &self,
            _page: &PageCapture<'_>,
            _options: &PluginOptions,
        ) -> Result<Vec<Preuve>> {
            Ok(Vec::new())
        }
    }

    #[test]
    #[cfg(feature = "plugin-echo")]
    fn a_test_served_by_two_plugins_is_flagged() {
        let plugins: Vec<&dyn AnalysisPlugin> = vec![&crate::plugin::EchoPlugin, &SecondEcho];
        let duplicates = duplicate_tests(&plugins);
        assert_eq!(duplicates["1.1.1"], vec!["echo", "second-echo"]);
    }
}
