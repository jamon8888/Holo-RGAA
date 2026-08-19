#[derive(Debug, Clone)]
pub struct Config {
    pub min_compliance: f64,
    pub required_criteria: Vec<String>,
    pub auto_remediate: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_compliance: 80.0,
            required_criteria: vec![],
            auto_remediate: false,
        }
    }
}