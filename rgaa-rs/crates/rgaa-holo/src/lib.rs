pub mod client;
pub mod prompts;

pub use client::{HoloClient, HoloResponse};
pub use prompts::{PromptBuilder, PageContext, format_page_context};
