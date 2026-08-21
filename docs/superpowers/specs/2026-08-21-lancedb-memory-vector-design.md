# Design: LanceDB Memory & Vector Store for RGAA Agent

- **Date:** 2026-08-21
- **Status:** Approved design, pending implementation plan
- **Owner:** rgaa-agent
- **Approach:** Monolithic rig-agent with LanceDB backend
- **Supersedes:** Custom RgaaAgent implementation
- **Complements:** `2026-08-21-rig-agentic-loop-design.md` (rig-core integration)
- **Complements:** `2026-08-21-rig-core-capabilities-analysis.md` (capability gaps)

## Context & Goal

The current `rgaa-agent` crate uses rig-core's building blocks but doesn't leverage rig-agent's full agentic loop, memory, or vector store capabilities. This design adds LanceDB as the backend for both conversation memory and semantic search, enabling the agent to remember past audits and find similar accessibility patterns.

**Goal:** Use rig-agent's official agentic loop with LanceDB-backed memory and vector store, supporting hybrid embeddings (local fastembed + external APIs).

## Requirements Summary

- **Memory + Vector**: LanceDB for both conversation memory and semantic search
- **Embeddings**: Local (fastembed) + external APIs (OpenAI, etc.)
- **Memory lifecycle**: Hybrid - short-term per-audit + long-term aggregate learning
- **Vector scope**: Criteria + findings + remediation patterns
- **Agentic loop**: Use rig-agent crate's full agentic loop

## Section 1: Architecture Overview

### High-Level Architecture

```
+-------------------------------------------------------------+
|                    rgaa-agent (rig-agent)                   |
+-------------------------------------------------------------+
|  AgentBuilder -> Agent -> prompt() -> tool calls -> results |
+-------------------------------------------------------------+
| Tools: NavigateTool, ScreenshotTool, A11yTreeTool,         |
|        ClickTool, PressKeyTool, TabOrderTool, TypeTool,    |
|        EvalJsTool, RemediateTool, ThinkTool                |
+-------------------------------------------------------------+
| Memory: LanceDbMemory (ConversationMemory trait)           |
+-------------------------------------------------------------+
| Vector: LanceDbVectorIndex (VectorStoreIndex trait)        |
+-------------------------------------------------------------+
| Embeddings: HybridProvider (fastembed + OpenAI)            |
+-------------------------------------------------------------+
| Storage: LanceDB (local file system)                       |
+-------------------------------------------------------------+
```

### Data Flow

1. Audit starts, create conversation_id
2. Agent loads history from LanceDbMemory
3. Agent evaluates criteria:
   - Loads similar past findings from vector store
   - Builds prompt with context + history
   - Calls Holo3 via rig-agent's OpenAI provider
   - Agent calls tools (browser, remediate, think)
   - Results appended to memory
4. Audit completes, findings stored in vector store
5. Memory persists for future audits

### Crate Structure

```
rgaa-agent/
  src/
    lib.rs           # Public API
    agent.rs         # RgaaAgent struct, AgentBuilder
    tools/
      mod.rs         # Tool re-exports
      browser/       # 9 browser tools (from rgaa-browser-tools)
      remediate.rs   # RemediateTool
      think.rs       # ThinkTool (rig-core builtin)
    memory/
      mod.rs         # LanceDbMemory implementation
      schema.rs      # LanceDB table schemas
    embeddings/
      mod.rs         # HybridProvider
      fastembed.rs   # FastEmbedModel
      openai.rs      # OpenAI embedding wrapper
    vector/
      mod.rs         # LanceDbVectorIndex wrapper
      schema.rs      # Vector store schemas
    config.rs        # Configuration types
```

## Section 2: LanceDB Memory Implementation

### ConversationMemory Trait

rig-core defines `ConversationMemory` with three methods:
- `load(conversation_id)` -> `Vec<Message>`
- `append(conversation_id, messages)` -> `Result<(), MemoryError>`
- `clear(conversation_id)` -> `Result<(), MemoryError>`

### LanceDbMemory Implementation

```rust
pub struct LanceDbMemory {
    db: lancedb::Connection,
    embedding_model: Arc<dyn EmbeddingModel>,
}

impl ConversationMemory for LanceDbMemory {
    async fn load(&self, conversation_id: &str) -> Result<Vec<Message>, MemoryError> {
        // Query LanceDB for messages by conversation_id
        // Order by timestamp ASC
        // Deserialize to Message types
    }

    async fn append(&self, conversation_id: &str, messages: Vec<Message>) -> Result<(), MemoryError> {
        // Serialize messages
        // Generate embeddings for each message
        // Insert into LanceDB with conversation_id + timestamp
    }

    async fn clear(&self, conversation_id: &str) -> Result<(), MemoryError> {
        // Delete all messages for conversation_id
    }
}
```

### LanceDB Table Schema

Table: conversation_messages
- id: UTF8 (primary key)
- conversation_id: UTF8 (indexed)
- role: UTF8 (user/assistant/tool)
- content: UTF8
- timestamp: TIMESTAMP
- embedding: VECTOR<FLOAT>(384) for semantic search
- metadata: JSON (tool calls, etc.)

### Memory Lifecycle

Per-audit (short-term):
- conversation_id = "audit-{url_hash}-{timestamp}"
- Stores messages for current audit session
- Cleared after audit completes (optional)

Long-term (aggregate):
- conversation_id = "findings-{criterion_id}"
- Stores successful remediation patterns
- Persists across audits
- Used for vector search

## Section 3: Vector Store and Embeddings

### Vector Store Schema

Table: rgaa_criteria
- id: UTF8 (primary key, e.g., "1.3")
- title: UTF8
- description: UTF8
- classification: UTF8
- wcag_refs: UTF8
- embedding: VECTOR<FLOAT>(384)

Table: rgaa_findings
- id: UTF8 (primary key)
- criterion_id: UTF8 (indexed)
- rule: UTF8
- element_html: UTF8
- page_url: UTF8
- remediation: UTF8
- embedding: VECTOR<FLOAT>(384)
- created_at: TIMESTAMP

Table: rgaa_remediation_patterns
- id: UTF8 (primary key)
- rule: UTF8 (indexed)
- framework: UTF8
- before_html: UTF8
- after_html: UTF8
- description: UTF8
- success_count: INT32
- embedding: VECTOR<FLOAT>(384)

### Hybrid Embedding Provider

```rust
pub enum EmbeddingBackend {
    FastEmbed(FastEmbedModel),      // Local, no API key
    OpenAi(OpenAiEmbeddingModel),   // External, requires API key
}

pub struct HybridEmbeddingProvider {
    primary: EmbeddingBackend,
    fallback: Option<EmbeddingBackend>,
    config: EmbeddingConfig,
}

impl EmbeddingModel for HybridEmbeddingProvider {
    fn embed_text(&self, text: &str) -> Result<Embedding> {
        // Try primary backend
        // If rate limited or unavailable, try fallback
        // Both backends must produce same dimension (384)
    }
}
```

### FastEmbed Integration

```rust
use fastembed::{EmbeddingModel, TextEmbedding};

pub struct FastEmbedModel {
    model: TextEmbedding,
    dimensions: usize,  // 384 for all-MiniLM-L6-v2
}

impl EmbeddingModel for FastEmbedModel {
    fn embed_text(&self, text: &str) -> Result<Embedding> {
        let embeddings = self.model.embed(vec![text], None)?;
        Ok(Embedding::new(embeddings[0].clone()))
    }
}
```

### Vector Search Use Cases

1. Find similar criteria:
   Query: "image alternative text"
   Returns: 1.1, 1.3, 1.4 (by semantic similarity)

2. Find similar past findings:
   Query: "missing alt attribute on img tag"
   Returns: Past findings with similar issues

3. Find successful remediation patterns:
   Query: "React component missing aria-label"
   Returns: Past fixes for similar patterns

## Section 4: Agentic Loop and Tool Integration

### rig-agent Integration

```rust
use rig::agent::Agent;
use rig::providers::openai;

pub struct RgaaAgent {
    agent: Agent<openai::CompletionModel>,
    memory: LanceDbMemory,
    vector_index: LanceDbVectorIndex,
}
```

### Agent Construction

```rust
impl RgaaAgent {
    pub fn new(config: &AgentConfig) -> Result<Self> {
        // 1. Create OpenAI client pointing at Holo3
        let client = openai::Client::builder()
            .base_url(&config.holo3_base_url)
            .api_key(&config.api_key)
            .build()?;

        // 2. Create embedding provider
        let embeddings = HybridEmbeddingProvider::new(config)?;

        // 3. Create LanceDB memory
        let memory = LanceDbMemory::new(&config.lancedb_path, embeddings.clone())?;

        // 4. Create vector index
        let vector_index = LanceDbVectorIndex::new(...)?;

        // 5. Build agent with tools
        let agent = client.agent(&config.model)
            .preamble(EXPERT_SYSTEM_PROMPT)
            .tool(NavigateTool::new(tool_ctx.clone()))
            .tool(ScreenshotTool::new(tool_ctx.clone()))
            .tool(A11yTreeTool::new(tool_ctx.clone()))
            .tool(ClickTool::new(tool_ctx.clone()))
            .tool(PressKeyTool::new(tool_ctx.clone()))
            .tool(TabOrderTool::new(tool_ctx.clone()))
            .tool(TypeTool::new(tool_ctx.clone()))
            .tool(EvalJsTool::new(tool_ctx.clone()))
            .tool(RemediateTool::new(policy))
            .tool(ThinkTool)
            .memory(memory.clone())
            .build();

        Ok(Self { agent, memory, vector_index })
    }
}
```

### Evaluation Flow with rig-agent

```rust
impl RgaaAgent {
    pub async fn evaluate_criterion(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
    ) -> CriterionResult {
        // 1. Search vector store for similar past findings
        let similar_findings = self.vector_index
            .top_n::<Finding>(&criterion.description, 5)
            .await?;

        // 2. Build prompt with context
        let prompt = PromptBuilder::build_with_context(
            criterion,
            page_context,
            &similar_findings,
        );

        // 3. rig-agent handles the agentic loop:
        //    - Sends prompt to model
        //    - Model calls tools (navigate, screenshot, etc.)
        //    - Tools execute, return results
        //    - Model reasons with tool results
        //    - Repeats until model produces final answer
        let response = self.agent.prompt(&prompt).await?;

        // 4. Parse response
        let verdict = parse_verdict(&response);

        // 5. Store finding in vector store
        if let Some(finding) = &verdict.finding {
            self.vector_index.insert(finding).await?;
        }

        // 6. Append to memory
        self.memory.append(&conversation_id, messages).await?;

        verdict
    }
}
```

### Tool Execution Context

```rust
// rig-agent passes ToolContext to each tool
pub struct ToolContext {
    pub session: Arc<Mutex<BrowserSession>>,
    pub memory: LanceDbMemory,
    pub vector_index: LanceDbVectorIndex,
}

// Each tool receives ToolContext
impl PortableTool for NavigateTool {
    type Args = NavigateArgs;
    type Output = NavigateOutput;
    type Error = NavigateError;

    async fn call(&self, args: Self::Args, ctx: &ToolContext) -> Result<Self::Output> {
        // Navigate browser
        let session = ctx.session.lock().await;
        session.navigate(&args.url).await?;
        Ok(NavigateOutput { success: true })
    }
}
```

### ThinkTool Integration

```rust
// rig-core's built-in ThinkTool for structured reasoning
use rig::tool::builtin::ThinkTool;

// Added to agent tools:
.tool(ThinkTool)

// Agent can now pause and reason:
// "I need to check if this image has proper alt text.
//  Let me think about what to look for..."
```

## Section 5: Configuration and Error Handling

### Configuration Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    // Holo3 API
    pub holo3_base_url: String,
    pub api_key: String,
    pub model: String,

    // LanceDB
    pub lancedb_path: String,

    // Embeddings
    pub embedding_backend: EmbeddingBackendConfig,
    pub embedding_dimensions: usize,  // 384

    // Memory
    pub memory_retention: MemoryRetention,

    // Agent behavior
    pub max_turns: usize,              // max tool call iterations
    pub max_tokens: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingBackendConfig {
    FastEmbed {
        model_name: String,  // e.g., "all-MiniLM-L6-v2"
    },
    OpenAi {
        base_url: String,
        api_key: String,
        model: String,       // e.g., "text-embedding-3-small"
    },
    Hybrid {
        primary: Box<EmbeddingBackendConfig>,
        fallback: Box<EmbeddingBackendConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryRetention {
    PerAudit,                    // cleared after each audit
    Persistent,                  // kept indefinitely
    Hybrid {
        short_term_ttl: Duration, // e.g., 7 days
        long_term_pattern: String, // conversation_id pattern
    },
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("rig agent error: {0}")]
    RigAgent(#[from] rig::agent::AgentError),

    #[error("lancedb error: {0}")]
    LanceDb(#[from] lancedb::Error),

    #[error("embedding error: {0}")]
    Embedding(#[from] rig::embeddings::EmbeddingError),

    #[error("memory error: {0}")]
    Memory(#[from] rig::memory::MemoryError),

    #[error("tool execution error: {0}")]
    ToolExecution(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("holog3 api error: {0}")]
    Holo3Api(String),
}
```

### Error Recovery

```rust
impl RgaaAgent {
    pub async fn evaluate_criterion_safe(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
    ) -> CriterionResult {
        match self.evaluate_criterion(criterion, page_context).await {
            Ok(result) => result,
            Err(e) => {
                // Log error with tracing
                tracing::warn!(
                    criterion = criterion.id,
                    error = %e,
                    "evaluation failed, returning NeedsReview"
                );

                // Return safe fallback
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::IaAssiste,
                    status: CriterionStatus::NeedsReview,
                    violations: vec![],
                    confidence: None,
                    justification: Some(format!("Erreur: {e}")),
                    source: "agent-error".to_string(),
                }
            }
        }
    }
}
```

### Configuration Loading

```rust
impl AgentConfig {
    pub fn from_env() -> Result<Self, AgentError> {
        Ok(Self {
            holo3_base_url: std::env::var("HOLO3_BASE_URL")
                .unwrap_or_else(|_| "https://api.hcompany.ai/v1".into()),
            api_key: std::env::var("HOLO3_API_KEY")
                .map_err(|_| AgentError::Config("HOLO3_API_KEY required".into()))?,
            model: std::env::var("HOLO3_MODEL")
                .unwrap_or_else(|_| "holo3-1-35b-a3b".into()),
            lancedb_path: std::env::var("LANCEDB_PATH")
                .unwrap_or_else(|_| "./data/lancedb".into()),
            embedding_backend: EmbeddingBackendConfig::Hybrid {
                primary: Box::new(EmbeddingBackendConfig::FastEmbed {
                    model_name: "all-MiniLM-L6-v2".into(),
                }),
                fallback: Box::new(EmbeddingBackendConfig::OpenAi {
                    base_url: "https://api.openai.com/v1".into(),
                    api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                    model: "text-embedding-3-small".into(),
                }),
            },
            embedding_dimensions: 384,
            memory_retention: MemoryRetention::Hybrid {
                short_term_ttl: Duration::from_secs(7 * 24 * 60 * 60),
                long_term_pattern: "findings-*".into(),
            },
            max_turns: 10,
            max_tokens: 4096,
            temperature: 0.3,
        })
    }
}
```

## Section 6: Testing and Migration

### Testing Strategy

- Unit tests for LanceDbMemory, HybridEmbeddingProvider, vector store operations
- Integration tests for full agent evaluation flow
- Use test databases with temporary directories
- Mock external APIs for deterministic testing

### Migration Path

Phase 1: Add LanceDB dependencies
- Add rig-lancedb, rig-memory, fastembed to Cargo.toml
- No breaking changes to existing code

Phase 2: Implement embedding provider
- Create FastEmbedModel
- Create HybridEmbeddingProvider
- Test independently

Phase 3: Implement LanceDbMemory
- Implement ConversationMemory trait
- Add LanceDB table schemas
- Test with unit tests

Phase 4: Create vector store tables
- Define schemas for criteria, findings, patterns
- Implement insert/query functions
- Seed with RGAA criteria

Phase 5: Integrate rig-agent
- Replace custom RgaaAgent with rig-agent Agent
- Wire tools, memory, vector store
- Update orchestrator

Phase 6: Update orchestrator
- Use new AgentConfig
- Remove old ModelRouter/RateLimiter (rig-agent handles this)
- Update pipeline.rs

Phase 7: Testing and validation
- Run integration tests
- Validate with real audits
- Performance benchmarking

### Breaking Changes

None expected. We're adding new functionality:
- New crate dependencies
- New modules in rgaa-agent
- New configuration options (with defaults)
- Existing API remains backward-compatible

### Performance Considerations

1. Embedding latency:
   - FastEmbed: ~10ms per text (local)
   - OpenAI: ~100ms per text (network)
   - Mitigation: Cache embeddings for repeated text

2. LanceDB query latency:
   - In-memory: ~1ms
   - On-disk: ~10ms
   - Mitigation: Use ANN index for large tables

3. Memory overhead:
   - LanceDB: ~50MB base + data
   - FastEmbed: ~100MB model in memory
   - Mitigation: Lazy loading, optional features
