pub mod client;
pub mod prompts;

pub use client::{HoloClient, HoloResponse};
pub use prompts::{format_page_context, PageContext, PromptBuilder};
