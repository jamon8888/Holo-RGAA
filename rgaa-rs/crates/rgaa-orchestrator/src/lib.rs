pub mod merge;
pub mod pipeline;
pub(crate) mod site_comparison;
pub mod transverse;
pub(crate) mod sitemap_links;

pub use merge::{merge_candidates, merge_results};
pub use pipeline::{
    aggregate_site_compliance, assemble_page_audit, assemble_site_audit, AuditEvent, AuditPhase,
    BatchObserver, Orchestrator,
};
pub use transverse::{
    apply_transverse_pages, detect_transverse_landmarks, normalized_subtree_hash,
    transverse_page_from, LandmarkNode, LandmarkSubtree, PageLandmarks, PageTransverseContext,
    TransverseSignature, TRANSVERSE_PAGE_URL, TRANSVERSE_REFERENCE_SOURCE, TRANSVERSE_THRESHOLD,
};
