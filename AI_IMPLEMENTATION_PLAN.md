# AI Commit Generation Implementation Plan

## Library Selection

### Chosen: `genai` crate
- **Pros**: Unified API for multiple providers (Anthropic, OpenAI, OpenRouter, Ollama, Groq)
- **Pros**: Single dependency, ergonomic API, actively maintained (May 2025)
- **Pros**: Matches SPEC.md requirement for multiple AI providers

### Alternative Libraries Considered
- **async-openai**: Mature OpenAI client (6 days ago update)
- **anthropic-ai-sdk**: Dedicated Anthropic SDK (July 2025)  
- **llm-chain**: Advanced features for chaining, agents (Nov 2023)

## Implementation Phases

### Phase 1: Core AI Module Structure
```rust
src/
├── ai/
│   ├── mod.rs          // Public API and re-exports
│   ├── provider.rs     // Provider enum & trait abstraction
│   ├── config.rs       // AI configuration handling
│   └── client.rs       // Generic client wrapper
```

**Tasks:**
1. Create AI module structure
2. Define provider trait and enum
3. Implement configuration parsing for AI section
4. Create generic client wrapper
5. Add unit tests for each component

### Phase 2: Provider Integration
**Tasks:**
1. Add `genai` dependency to Cargo.toml
2. Implement provider-specific client initialization
3. Handle API key management (env vars, files per SPEC.md)
4. Implement retry logic and comprehensive error handling
5. Add integration tests with mock providers

### Phase 3: Commit Generation Logic
```rust
src/generate.rs
```

**Tasks:**
1. Implement git diff analysis for staged changes
2. Extract context (branch name, recent commits)
3. Build prompt templates following conventional commit spec
4. Generate commit messages with validation against lint rules
5. Implement response caching to minimize API calls
6. Add comprehensive unit tests

### Phase 4: CLI Integration
**Tasks:**
1. Add `cocoa generate` command to CLI
2. Implement interactive editing of generated messages
3. Add real-time validation feedback
4. Implement dry-run mode for testing
5. Add end-to-end integration tests

## Key Design Decisions

1. **Use `genai` crate** for unified provider interface
2. **Async/await** with tokio for API calls  
3. **Builder pattern** for configurable generation
4. **Result types** for comprehensive error handling per SPEC.md
5. **Template system** for customizable prompts
6. **Security**: Never log API keys, read from env/files only

## Configuration Schema (from SPEC.md)
```toml
[ai]
provider = "openai" # or "anthropic", "ollama", "openrouter"
model = "gpt-4"
temperature = 0.7
max_tokens = 500

[ai.secret]
env = "OPENAI_API_KEY" # or file path
file = "./path/to/file"
```

## Security Requirements
- API keys MUST NEVER be stored in configuration files
- API keys MUST be read from environment variables or secure storage
- The system MUST NOT log sensitive information
- Generated commits MUST NOT include sensitive data from diffs

## Testing Strategy
- Unit tests for each module
- Integration tests with mock AI providers
- End-to-end tests with actual git repositories
- Security tests to ensure no key leakage