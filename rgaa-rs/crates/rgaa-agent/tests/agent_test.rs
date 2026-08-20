use rgaa_agent::prompts::PromptBuilder;
use rgaa_holo::PageContext;

fn sample_context() -> PageContext {
    PageContext {
        title: Some("Test Page".to_string()),
        lang: Some("fr".to_string()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    }
}

#[test]
fn prompt_includes_criterion_definition() {
    let prompt = PromptBuilder::build("1.3", &sample_context());
    assert!(prompt.contains("Alternative textuelle pertinente"));
    assert!(prompt.contains("1.1.1"));
}

#[test]
fn prompt_includes_page_title() {
    let prompt = PromptBuilder::build("3.1", &sample_context());
    assert!(prompt.contains("Test Page"));
}

#[test]
fn prompt_includes_instructions() {
    let prompt = PromptBuilder::build("12.8", &sample_context());
    assert!(prompt.contains("verdict"));
    assert!(prompt.contains("confidence"));
    assert!(prompt.contains("justification"));
}
