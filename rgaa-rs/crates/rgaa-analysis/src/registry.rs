//! Explicit registry of analysis plugins (P-00 §4).
//!
//! The registry is *built*, not discovered from Cargo features: the contract
//! crate cannot depend on the plugin crates that depend on it. Callers assemble
//! the plugins they linked (see the `rgaa-plugins` aggregator) and pass them to
//! [`Registry::builder`]. The build sorts by stable identifier and refuses
//! duplicate identifiers.

use crate::error::{analysis_error, Result};
use crate::types::PluginInfo;
use crate::AnalysisPlugin;
use indexmap::IndexMap;
use std::collections::HashSet;

/// An immutable, ordered catalog of the plugins linked into this binary.
pub struct Registry {
    plugins: Vec<Box<dyn AnalysisPlugin>>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Registry")
            .field(
                "ids",
                &self.iter().map(AnalysisPlugin::id).collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Accumulates plugins before validation. Use [`Registry::builder`].
pub struct RegistryBuilder {
    plugins: Vec<Box<dyn AnalysisPlugin>>,
}

impl Registry {
    /// Starts an empty registry.
    #[must_use]
    pub fn builder() -> RegistryBuilder {
        RegistryBuilder {
            plugins: Vec::new(),
        }
    }

    /// Plugins in stable identifier order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &dyn AnalysisPlugin> {
        self.plugins.iter().map(AsRef::as_ref)
    }

    /// Looks up one plugin by its stable identifier.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&dyn AnalysisPlugin> {
        self.iter().find(|plugin| plugin.id() == id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Displayable descriptions, in registry order.
    #[must_use]
    pub fn list(&self) -> Vec<PluginInfo> {
        self.iter().map(|plugin| plugin.info()).collect()
    }

    /// RGAA tests served by more than one plugin, in registry order.
    #[must_use]
    pub fn duplicate_tests(&self) -> IndexMap<String, Vec<String>> {
        let mut providers: IndexMap<String, Vec<String>> = IndexMap::new();
        for plugin in self.iter() {
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
}

impl RegistryBuilder {
    /// Adds one plugin. Duplicate identifiers are rejected by [`Self::build`].
    #[must_use]
    pub fn with(mut self, plugin: Box<dyn AnalysisPlugin>) -> Self {
        self.plugins.push(plugin);
        self
    }

    /// Adds a plugin only when the feature that links it is enabled.
    #[must_use]
    pub fn maybe_with(self, plugin: Option<Box<dyn AnalysisPlugin>>) -> Self {
        match plugin {
            Some(plugin) => self.with(plugin),
            None => self,
        }
    }

    /// Validates and sorts the plugins.
    ///
    /// # Errors
    ///
    /// Returns [`crate::RgaaError::Analysis`] when two plugins declare the same
    /// identifier: a duplicate would make `run_analysis_plugin` ambiguous.
    pub fn build(mut self) -> Result<Registry> {
        let mut seen = HashSet::with_capacity(self.plugins.len());
        for plugin in &self.plugins {
            if !seen.insert(plugin.id()) {
                return Err(analysis_error(
                    "registry",
                    format!("identifiant de plugin en double : {}", plugin.id()),
                ));
            }
        }
        self.plugins
            .sort_by(|left, right| left.id().cmp(right.id()));
        Ok(Registry {
            plugins: self.plugins,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PageCapture;
    use crate::types::{PluginOptions, Preuve};
    use async_trait::async_trait;

    struct PluginA;
    struct PluginB;
    struct SecondEcho;

    #[async_trait]
    impl AnalysisPlugin for PluginA {
        fn id(&self) -> &'static str {
            "zebra"
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

    #[async_trait]
    impl AnalysisPlugin for PluginB {
        fn id(&self) -> &'static str {
            "alpha"
        }
        fn version(&self) -> &'static str {
            "0.1.0"
        }
        fn tests(&self) -> &'static [&'static str] {
            &["1.1.1", "2.2.2"]
        }
        async fn analyze(
            &self,
            _page: &PageCapture<'_>,
            _options: &PluginOptions,
        ) -> Result<Vec<Preuve>> {
            Ok(Vec::new())
        }
    }

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
    fn registry_is_built_in_stable_identifier_order() {
        let registry = Registry::builder()
            .with(Box::new(PluginA))
            .with(Box::new(PluginB))
            .build()
            .expect("registry");
        let ids: Vec<&str> = registry.iter().map(|plugin| plugin.id()).collect();
        assert_eq!(ids, vec!["alpha", "zebra"]);
        assert_eq!(registry.get("zebra").map(AnalysisPlugin::id), Some("zebra"));
        assert!(registry.get("absent").is_none());
    }

    #[test]
    fn duplicate_identifiers_are_refused() {
        let error = Registry::builder()
            .with(Box::new(PluginA))
            .with(Box::new(PluginA))
            .build()
            .expect_err("duplicates must be refused");
        assert!(error.to_string().contains("zebra"), "{error}");
    }

    #[test]
    fn a_test_served_by_two_plugins_is_flagged() {
        let registry = Registry::builder()
            .with(Box::new(PluginA))
            .with(Box::new(SecondEcho))
            .build()
            .expect("registry");
        let duplicates = registry.duplicate_tests();
        assert_eq!(duplicates["1.1.1"], vec!["second-echo", "zebra"]);
        assert!(!duplicates.contains_key("2.2.2"));
    }

    #[cfg(feature = "plugin-echo")]
    #[test]
    fn an_empty_registry_has_no_plugins() {
        let registry = Registry::builder().build().expect("empty registry");
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.list().is_empty());
    }
}
