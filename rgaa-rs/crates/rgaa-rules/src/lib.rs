pub mod axe_mapper;
pub mod gap_fix;
#[cfg(feature = "seo")]
pub mod seo;

pub use axe_mapper::AxeMapper;
pub use gap_fix::GapFixRules;
#[cfg(feature = "seo")]
pub use seo::SeoMapper;
