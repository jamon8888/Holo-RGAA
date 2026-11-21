//! Published CVD simulation matrices, in linear sRGB (P-02 §3).
//!
//! Machado, Oliveira & Fernandes (2009), "A Physiologically-based Model for
//! Simulation of Color Vision Deficiency", IEEE TVCG 15(6), Table 1 — the
//! eleven matrices at severity steps of 0.1, transcribed verbatim from
//! <https://inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html>.
//!
//! Tritanopia uses the two projection planes of Brettel, Viénot & Mollon
//! (1997) as published (with the Viénot linear-RGB variant) by DaltonLens:
//! <https://daltonlens.org/cvd-simulation-svg-filters/>. The separation plane
//! normal selects one of the two matrices per pixel.
//!
//! The same coefficients are exported to `types/colorvision-matrices.json`
//! for the P-09 review page; the export is generated from these constants.

use serde::{Deserialize, Serialize};

use crate::color::{encode_channel, linearize_channel, Rgb8};

/// The simulated deficiency. `Achromatopsia` is the Rec. 709 greyscale
/// reference used by RGAA 3.1 ("does the information survive in greyscale?").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CvdKind {
    Protanopia,
    Deuteranopia,
    Tritanopia,
    Achromatopsia,
}

impl CvdKind {
    /// Kebab-case identifier used in `mesures.simulations[].type`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Protanopia => "protanopia",
            Self::Deuteranopia => "deuteranopia",
            Self::Tritanopia => "tritanopia",
            Self::Achromatopsia => "achromatopsia",
        }
    }

    /// French label used in the review page selector (P-02 §6).
    pub fn label(self) -> &'static str {
        match self {
            Self::Protanopia => "protanope",
            Self::Deuteranopia => "deutéranope",
            Self::Tritanopia => "tritanope",
            Self::Achromatopsia => "niveaux de gris",
        }
    }
}

/// All four simulations, in a stable order.
pub const ALL_KINDS: [CvdKind; 4] = [
    CvdKind::Protanopia,
    CvdKind::Deuteranopia,
    CvdKind::Tritanopia,
    CvdKind::Achromatopsia,
];

/// A row-major 3×3 matrix applied to linear sRGB.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Matrix3 {
    pub rows: [[f64; 3]; 3],
}

impl Matrix3 {
    /// Builds a matrix from its three rows.
    pub const fn new(rows: [[f64; 3]; 3]) -> Self {
        Self { rows }
    }

    /// Multiplies this matrix by a linear RGB vector.
    pub fn apply(self, vector: [f64; 3]) -> [f64; 3] {
        let mut out = [0.0; 3];
        for (target, row) in out.iter_mut().zip(self.rows.iter()) {
            *target = row[0] * vector[0] + row[1] * vector[1] + row[2] * vector[2];
        }
        out
    }
}

impl Default for Matrix3 {
    fn default() -> Self {
        Self::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    }
}

const MACHADO_PROTAN: [Matrix3; 11] = [
    Matrix3::new([
        [1.000000, 0.000000, 0.000000],
        [0.000000, 1.000000, 0.000000],
        [0.000000, 0.000000, 1.000000],
    ]),
    Matrix3::new([
        [0.856167, 0.182038, -0.038205],
        [0.029342, 0.955115, 0.015544],
        [-0.002880, -0.001563, 1.004443],
    ]),
    Matrix3::new([
        [0.734766, 0.334872, -0.069637],
        [0.051840, 0.919198, 0.028963],
        [-0.004928, -0.004209, 1.009137],
    ]),
    Matrix3::new([
        [0.630323, 0.465641, -0.095964],
        [0.069181, 0.890046, 0.040773],
        [-0.006308, -0.007724, 1.014032],
    ]),
    Matrix3::new([
        [0.539009, 0.579343, -0.118352],
        [0.082546, 0.866121, 0.051332],
        [-0.007136, -0.011959, 1.019095],
    ]),
    Matrix3::new([
        [0.458064, 0.679578, -0.137642],
        [0.092785, 0.846313, 0.060902],
        [-0.007494, -0.016807, 1.024301],
    ]),
    Matrix3::new([
        [0.385450, 0.769005, -0.154455],
        [0.100526, 0.829802, 0.069673],
        [-0.007442, -0.022190, 1.029632],
    ]),
    Matrix3::new([
        [0.319627, 0.849633, -0.169261],
        [0.106241, 0.815969, 0.077790],
        [-0.007025, -0.028051, 1.035076],
    ]),
    Matrix3::new([
        [0.259411, 0.923008, -0.182420],
        [0.110296, 0.804340, 0.085364],
        [-0.006276, -0.034346, 1.040622],
    ]),
    Matrix3::new([
        [0.203876, 0.990338, -0.194214],
        [0.112975, 0.794542, 0.092483],
        [-0.005222, -0.041043, 1.046265],
    ]),
    Matrix3::new([
        [0.152286, 1.052583, -0.204868],
        [0.114503, 0.786281, 0.099216],
        [-0.003882, -0.048116, 1.051998],
    ]),
];

const MACHADO_DEUTAN: [Matrix3; 11] = [
    Matrix3::new([
        [1.000000, 0.000000, 0.000000],
        [0.000000, 1.000000, 0.000000],
        [0.000000, 0.000000, 1.000000],
    ]),
    Matrix3::new([
        [0.866435, 0.177704, -0.044139],
        [0.049567, 0.939063, 0.011370],
        [-0.003453, 0.007233, 0.996220],
    ]),
    Matrix3::new([
        [0.760729, 0.319078, -0.079807],
        [0.090568, 0.889315, 0.020117],
        [-0.006027, 0.013325, 0.992702],
    ]),
    Matrix3::new([
        [0.675425, 0.433850, -0.109275],
        [0.125303, 0.847755, 0.026942],
        [-0.007950, 0.018572, 0.989378],
    ]),
    Matrix3::new([
        [0.605511, 0.528560, -0.134071],
        [0.155318, 0.812366, 0.032316],
        [-0.009376, 0.023176, 0.986200],
    ]),
    Matrix3::new([
        [0.547494, 0.607765, -0.155259],
        [0.181692, 0.781742, 0.036566],
        [-0.010410, 0.027275, 0.983136],
    ]),
    Matrix3::new([
        [0.498864, 0.674741, -0.173604],
        [0.205199, 0.754872, 0.039929],
        [-0.011131, 0.030969, 0.980162],
    ]),
    Matrix3::new([
        [0.457771, 0.731899, -0.189670],
        [0.226409, 0.731012, 0.042579],
        [-0.011595, 0.034333, 0.977261],
    ]),
    Matrix3::new([
        [0.422823, 0.781057, -0.203881],
        [0.245752, 0.709602, 0.044646],
        [-0.011843, 0.037423, 0.974421],
    ]),
    Matrix3::new([
        [0.392952, 0.823610, -0.216562],
        [0.263559, 0.690210, 0.046232],
        [-0.011910, 0.040281, 0.971630],
    ]),
    Matrix3::new([
        [0.367322, 0.860646, -0.227968],
        [0.280085, 0.672501, 0.047413],
        [-0.011820, 0.042940, 0.968881],
    ]),
];

const MACHADO_TRITAN: [Matrix3; 11] = [
    Matrix3::new([
        [1.000000, 0.000000, 0.000000],
        [0.000000, 1.000000, 0.000000],
        [0.000000, 0.000000, 1.000000],
    ]),
    Matrix3::new([
        [0.926670, 0.092514, -0.019184],
        [0.021191, 0.964503, 0.014306],
        [0.008437, 0.054813, 0.936750],
    ]),
    Matrix3::new([
        [0.895720, 0.133330, -0.029050],
        [0.029997, 0.945400, 0.024603],
        [0.013027, 0.104707, 0.882266],
    ]),
    Matrix3::new([
        [0.905871, 0.127791, -0.033662],
        [0.026856, 0.941251, 0.031893],
        [0.013410, 0.148296, 0.838294],
    ]),
    Matrix3::new([
        [0.948035, 0.089490, -0.037526],
        [0.014364, 0.946792, 0.038844],
        [0.010853, 0.193991, 0.795156],
    ]),
    Matrix3::new([
        [1.017277, 0.027029, -0.044306],
        [-0.006113, 0.958479, 0.047634],
        [0.006379, 0.248708, 0.744913],
    ]),
    Matrix3::new([
        [1.104996, -0.046633, -0.058363],
        [-0.032137, 0.971635, 0.060503],
        [0.001336, 0.317922, 0.680742],
    ]),
    Matrix3::new([
        [1.193214, -0.109812, -0.083402],
        [-0.058496, 0.979410, 0.079086],
        [-0.002346, 0.403492, 0.598854],
    ]),
    Matrix3::new([
        [1.257728, -0.139648, -0.118081],
        [-0.078003, 0.975409, 0.102594],
        [-0.003316, 0.501214, 0.502102],
    ]),
    Matrix3::new([
        [1.278864, -0.125333, -0.153531],
        [-0.084748, 0.957674, 0.127074],
        [-0.000989, 0.601151, 0.399838],
    ]),
    Matrix3::new([
        [1.255528, -0.076749, -0.178779],
        [-0.078411, 0.930809, 0.147602],
        [0.004733, 0.691367, 0.303900],
    ]),
];
/// The Rec. 709 luminance matrix, replicated on all three output channels.
pub fn achromatopsia() -> Matrix3 {
    Matrix3::new([
        [0.2126, 0.7152, 0.0722],
        [0.2126, 0.7152, 0.0722],
        [0.2126, 0.7152, 0.0722],
    ])
}

/// The two Brettel 1997 tritan projection planes and the normal of their
/// separation plane, after conversion to linear RGB (Viénot variant).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BrettelTritan {
    pub h1: Matrix3,
    pub h2: Matrix3,
    pub separation: [f64; 3],
}

/// Published Brettel/Viénot tritan coefficients.
pub fn brettel_tritan() -> BrettelTritan {
    BrettelTritan {
        h1: Matrix3::new([
            [1.01354, 0.14268, -0.15622],
            [-0.01181, 0.87561, 0.13619],
            [0.07707, 0.81208, 0.11085],
        ]),
        h2: Matrix3::new([
            [0.93337, 0.19999, -0.13336],
            [0.05809, 0.82565, 0.11626],
            [-0.37923, 1.13825, 0.24098],
        ]),
        separation: [7.92482, -5.66475, -2.26007],
    }
}

fn machado_table(kind: CvdKind) -> &'static [Matrix3; 11] {
    match kind {
        CvdKind::Protanopia => &MACHADO_PROTAN,
        CvdKind::Deuteranopia => &MACHADO_DEUTAN,
        CvdKind::Tritanopia => &MACHADO_TRITAN,
        CvdKind::Achromatopsia => &MACHADO_PROTAN, // never called
    }
}

/// Machado's matrix at `severity`, linearly interpolated between the two
/// neighbouring published 0.1 steps (the approximation recommended by the
/// authors). Severity is clamped to `[0.0, 1.0]`.
pub fn machado(kind: CvdKind, severity: f64) -> Matrix3 {
    if kind == CvdKind::Achromatopsia {
        return achromatopsia();
    }
    let severity = severity.clamp(0.0, 1.0) * 10.0;
    let low = severity.floor() as usize;
    let high = (severity.ceil() as usize).min(10);
    let weight = severity - low as f64;
    let table = machado_table(kind);
    let mut rows = [[0.0; 3]; 3];
    for (target_row, (low_row, high_row)) in rows
        .iter_mut()
        .zip(table[low].rows.iter().zip(table[high].rows.iter()))
    {
        for (target, (low_value, high_value)) in target_row
            .iter_mut()
            .zip(low_row.iter().zip(high_row.iter()))
        {
            *target = low_value * (1.0 - weight) + high_value * weight;
        }
    }
    Matrix3::new(rows)
}

/// Simulates one linear sRGB triplet. The severity applies to the Machado
/// matrices; for Brettel tritan it blends the projection with the identity
/// (documented approximation, severity 1.0 is the published model).
pub fn simulate_linear(linear: [f64; 3], kind: CvdKind, severity: f64) -> [f64; 3] {
    let out = match kind {
        CvdKind::Achromatopsia => achromatopsia().apply(linear),
        CvdKind::Tritanopia => {
            let tritan = brettel_tritan();
            let side = tritan
                .separation
                .iter()
                .zip(linear)
                .map(|(n, v)| n * v)
                .sum::<f64>();
            let plane = if side >= 0.0 { tritan.h1 } else { tritan.h2 };
            plane.apply(linear)
        }
        _ => machado(kind, severity).apply(linear),
    };
    if severity >= 1.0 {
        return out;
    }
    // Anomalous trichromacy: interpolate the simulated colour towards the
    // original one. Only used for severity < 1 (Brettel has no severity table).
    let weight = severity.clamp(0.0, 1.0);
    let mut mixed = [0.0; 3];
    for channel in 0..3 {
        mixed[channel] = linear[channel] * (1.0 - weight) + out[channel] * weight;
    }
    mixed
}

/// Simulates one 8-bit sRGB colour: linearise, apply the matrix, clamp and
/// re-encode. This is the entry point used by the oracle tests.
pub fn simulate_rgb(colour: Rgb8, kind: CvdKind, severity: f64) -> Rgb8 {
    let linear = [
        linearize_channel(colour[0]),
        linearize_channel(colour[1]),
        linearize_channel(colour[2]),
    ];
    let simulated = simulate_linear(linear, kind, severity);
    [
        encode_channel(simulated[0]),
        encode_channel(simulated[1]),
        encode_channel(simulated[2]),
    ]
}

/// Deterministic document exported for the P-09 review page. Kept here so
/// the browser filter and the plugin cannot drift apart.
pub fn matrices_document() -> serde_json::Value {
    let mut severities = serde_json::Map::new();
    for index in 0..11 {
        let label = format!("{:.1}", index as f64 / 10.0);
        severities.insert(
            label,
            serde_json::json!({
                "protanopia": MACHADO_PROTAN[index].rows,
                "deuteranopia": MACHADO_DEUTAN[index].rows,
                "tritanopia_machado": MACHADO_TRITAN[index].rows,
            }),
        );
    }
    serde_json::json!({
        "$comment": "Generated by `cargo run -p rgaa-analysis-colorvision --bin dump-colorvision-matrices`. Do not edit by hand.",
        "source": {
            "machado_2009": "https://inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html",
            "brettel_1997_vienot": "https://daltonlens.org/cvd-simulation-svg-filters/",
            "achromatopsia": "Rec. 709 linear luminance (RGAA 3.1 greyscale reference)"
        },
        "machado": severities,
        "brettel_tritan": {
            "h1": brettel_tritan().h1.rows,
            "h2": brettel_tritan().h2.rows,
            "separation": brettel_tritan().separation,
        },
        "achromatopsia": achromatopsia().rows,
    })
}

/// The exported JSON, also available at `types/colorvision-matrices.json`.
pub const MATRICES_JSON: &str = include_str!("../../../../types/colorvision-matrices.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_at_a_published_step_returns_that_step() {
        let interpolated = machado(CvdKind::Deuteranopia, 0.7);
        for row in 0..3 {
            for col in 0..3 {
                assert!(
                    (interpolated.rows[row][col] - MACHADO_DEUTAN[7].rows[row][col]).abs() < 1e-9
                );
            }
        }
    }
}
