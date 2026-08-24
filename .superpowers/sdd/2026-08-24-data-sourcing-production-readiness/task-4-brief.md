# Task 4: Parse automatability from criteres.json

**Files:**
- Create: `rgaa-rs/crates/rgaa-data/src/automatability.rs`
- Modify: `rgaa-rs/crates/rgaa-data/src/main.rs`

**Interfaces:**
- Consumes: `criteres.json` (vendored at `rgaa-core/data/rgaa-4.1.2/criteres.json`)
- Produces: `automatable_criteres.json` with 106 criteria + automatability status

## Steps

- [ ] **Step 1: Write automatability.rs**

Create `/home/jamin/Documents/Holo-RGAA/rgaa-rs/crates/rgaa-data/src/automatability.rs`:

The file should:
1. Load and parse `criteres.json` (JSON structure: `{"topics": [{"criteria": [{"criterium": {"number": "1.1", "title": "...", "tests": {"principle_1": ["test 1", "test 2"]}}}]}]}`)
2. Extract test keys for each criterion
3. Classify each criterion based on its test keys:
   - **FullyAutomatable**: All tests use keys like `principle_X` (no human verification needed)
   - **PartiallyAutomatable**: Some tests are automatable, some require human verification
   - **NotAutomatable**: Tests require human verification (screen reader, cognitive load, etc.)

Create these types:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatableCriterion {
    pub criterion_id: String,
    pub title: String,
    pub classification: String, // "FullyAutomatable", "PartiallyAutomatable", "NotAutomatable"
    pub automatable_test_count: usize,
    pub total_test_count: usize,
    pub test_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatabilityReport {
    pub total_criteria: usize,
    pub fully_automatable: usize,
    pub partially_automatable: usize,
    pub not_automatable: usize,
    pub criteria: Vec<AutomatableCriterion>,
}
```

For the initial classification, use a simple heuristic:
- Criteria with only `principle_1` through `principle_5` test keys are "FullyAutomatable"
- Criteria with test keys mentioning "human", "screen reader", "cognitive", "subjective" are "NotAutomatable"
- All others are "PartiallyAutomatable"

- [ ] **Step 2: Wire into main.rs**

Add to main.rs after the mapping validation:
```rust
tracing::info!("Analyzing automatability from criteres.json...");
let automatability = automatability::analyze_automatability()?;
let auto_json = serde_json::to_string_pretty(&automatability)?;
std::fs::write(out_dir.join("automatable_criteres.json"), &auto_json)?;
tracing::info!(
    "Automatability: {} fully, {} partial, {} not automatable",
    automatability.fully_automatable,
    automatability.partially_automatable,
    automatability.not_automatable
);
```

- [ ] **Step 3: Run and verify**

Run: `cargo run -p rgaa-data`
Expected: `automatable_criteres.json` with 106 criteria

- [ ] **Step 4: Verify output**

Run: `python3 -c "import json; d=json.load(open('crates/rgaa-core/data/rgaa-4.1.2/automatable_criteres.json')); print(f'Total: {d[\"total_criteria\"]}'); print(f'Fully: {d[\"fully_automatable\"]}'); print(f'Partial: {d[\"partially_automatable\"]}'); print(f'Not: {d[\"not_automatable\"]}')"`
Expected: Total: 106, and counts that sum to 106

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-data/
git commit -m "Add automatability analysis from criteres.json"
```
