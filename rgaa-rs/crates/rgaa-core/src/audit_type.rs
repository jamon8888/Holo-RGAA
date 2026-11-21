//! Audit scope (`rapide` / `complémentaire` / `complet`).
//!
//! The 25 + 25 criteria lists are versioned data next to `criteres.json`,
//! transcribed from the official DesignGouv tools. Nothing here hard-codes a
//! criterion number: adding a criterion to the RGAA catalog automatically adds
//! it to `Complet`, and editing the data file is what changes the two partial
//! scopes.

use std::collections::HashSet;
use std::sync::OnceLock;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::RgaaCatalog;

const AUDIT_TYPES_JSON: &str = include_str!("../data/rgaa-4.1.2/audit_types.json");

/// Scope of an audit: 25, 50 or all 106 RGAA criteria.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditType {
    /// The 25 essential criteria of the official *audit rapide*.
    Rapide,
    /// The rapid 25 plus the 25 additional criteria of the *audit complémentaire*.
    Complementaire,
    /// All 106 criteria of the RGAA 4.1.2 catalog.
    #[default]
    Complet,
}

impl AuditType {
    /// Every scope, in increasing size order.
    pub const ALL: [Self; 3] = [Self::Rapide, Self::Complementaire, Self::Complet];

    /// Stable wire/render name (`rapide`, `complementaire`, `complet`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rapide => "rapide",
            Self::Complementaire => "complementaire",
            Self::Complet => "complet",
        }
    }

    /// French label for reports and API responses.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rapide => "Audit rapide",
            Self::Complementaire => "Audit complémentaire",
            Self::Complet => "Audit complet",
        }
    }

    /// Only a full 106-criterion audit may carry a conformity declaration.
    ///
    /// A rapid audit is explicitly a partial measurement of RGAA uptake, not a
    /// statement about the site's conformance (DesignGouv, *audit rapide*).
    #[must_use]
    pub const fn allows_conformity_declaration(self) -> bool {
        !matches!(self, Self::Rapide)
    }

    /// Whether `criterion_id` belongs to this audit's scope.
    #[must_use]
    pub fn contains(self, criterion_id: &str) -> bool {
        match self {
            Self::Complet => RgaaCatalog::by_id(criterion_id).is_some(),
            Self::Rapide => selections().rapide_set.contains(criterion_id),
            Self::Complementaire => selections().complementaire_set.contains(criterion_id),
        }
    }

    /// Scope criteria in a deterministic order: the order of the versioned data
    /// for the two partial scopes, catalog order for `Complet`.
    #[must_use]
    pub fn criterion_ids(self) -> Vec<String> {
        match self {
            Self::Rapide => selections().rapide.clone(),
            Self::Complementaire => selections().complementaire.clone(),
            Self::Complet => RgaaCatalog::all()
                .iter()
                .flat_map(|theme| {
                    theme
                        .criteria
                        .iter()
                        .map(move |wrapper| wrapper.criterium.id_for_theme(theme.number))
                })
                .collect(),
        }
    }

    /// Number of criteria in scope: 25, 50 or 106.
    #[must_use]
    pub fn count(self) -> usize {
        match self {
            Self::Rapide => selections().rapide.len(),
            Self::Complementaire => selections().complementaire.len(),
            Self::Complet => RgaaCatalog::count(),
        }
    }
}

struct Selections {
    rapide: Vec<String>,
    complementaire: Vec<String>,
    rapide_set: HashSet<String>,
    complementaire_set: HashSet<String>,
}

#[derive(Deserialize)]
struct RawAuditTypes {
    types: RawTypeLists,
}

#[derive(Deserialize)]
struct RawTypeLists {
    rapide: Vec<String>,
    complementaire: Vec<String>,
}

fn selections() -> &'static Selections {
    static SELECTIONS: OnceLock<Selections> = OnceLock::new();
    SELECTIONS.get_or_init(|| {
        // Invariant: this JSON is compiled into the binary by include_str! and
        // validated by tests/audit_types.rs. A parse failure is a build defect,
        // not a recoverable runtime error.
        let raw: RawAuditTypes =
            serde_json::from_str(AUDIT_TYPES_JSON).expect("embedded audit_types.json must parse");
        let mut complementaire = raw.types.rapide.clone();
        complementaire.extend(raw.types.complementaire.iter().cloned());
        let rapide_set = raw.types.rapide.iter().cloned().collect();
        let complementaire_set = complementaire.iter().cloned().collect();
        Selections {
            rapide: raw.types.rapide,
            complementaire,
            rapide_set,
            complementaire_set,
        }
    })
}
