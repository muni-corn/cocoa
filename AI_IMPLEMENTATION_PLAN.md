# AI Commit Generation Implementation Plan

## Overview

This document outlines the implementation plan for AI-powered commit message generation in cocoa, following the requirements specified in SPEC.md.

## Library Selection

### **Recommended: genai (v0.3.5)**
- **Pros**: Unified API for multiple providers (Anthropic, OpenAI, OpenRouter, Ollama, Groq)
- **Pros**: Single dependency, ergonomic API, actively maintained (May 2025)
- **Pros**: Built-in async/await support with tokio
- **Use**: Primary choice for multi-provider support

### **Alternative: Individual SDKs**
- **async-openai**: Mature OpenAI client (updated 6 days ago)
- **anthropic-ai-sdk**: Dedicated Anthropic SDK (July 2025)
- **Cons**: Multiple dependencies, different APIs per provider

## Implementation Phases

### **Phase 1: Core AI Module ✅**
```rust
src/
├── ai/
│   ├── mod.rs          // Public API exports
│   ├── provider.rs     // Provider enum & error handling
│   ├── config.rs       // AI configuration & secret management
│   └── client.rs       // Generic client wrapper
```

**Features Implemented:**
- Provider enum supporting OpenAI, Anthropic, Ollama, OpenRouter
- Secure API key management (env vars, files)
- Configuration parsing with serde
- Comprehensive error handling
- Full unit test coverage

### **Phase 2: Provider Integration ✅**
**Features Implemented:**
- Integration with genai crate for real AI calls
- Async client initialization with proper error handling
- Temperature and max_tokens configuration support
- Context-aware prompt building
- Multi-provider abstraction layer

### **Phase 3: Commit Generation Logic** 🔄
```rust
src/generate.rs
```

**Features to Implement:**
- Git staged changes analysis (`git diff --cached`)
- Context extraction (branch name, recent commits)
- Conventional commit prompt generation
- Message validation against lint rules
- Response caching to minimize API calls
- Dry-run mode support

**Key Functions:**
```rust
pub async fn generate_commit_message(config: &Config) -> Result<String, GenerateError>
pub fn extract_git_context() -> Result<CommitContext, GitError>
pub fn analyze_staged_changes() -> Result<String, GitError>
pub fn build_generation_prompt(changes: &str, context: &CommitContext) -> String
```

### **Phase 4: CLI Integration**
**Features to Implement:**
- `cocoa generate` command implementation
- Interactive message editing with user confirmation
- Real-time validation feedback during editing
- Integration with existing lint module
- Proper error handling and user messaging

**CLI Flow:**
1. Check for staged changes
2. Extract git context
3. Generate commit message via AI
4. Present to user for editing/confirmation
5. Validate against lint rules
6. Commit or save message

## Key Design Decisions

### **1. Use genai crate**
- Unified interface for multiple AI providers
- Single dependency reduces complexity
- Active maintenance and feature updates

### **2. Async/await architecture**
- Non-blocking AI API calls
- Better performance for network operations
- Tokio runtime integration

### **3. Builder pattern for configuration**
- Flexible message generation options
- Easy testing and customization
- Clean separation of concerns

### **4. Comprehensive error handling**
- Result types for all fallible operations
- Specific error variants for different failure modes
- User-friendly error messages with actionable suggestions

### **5. Template-based prompts**
- Customizable prompt generation
- Easy maintenance and updates
- Support for different commit styles

## Security Considerations

### **API Key Management**
- Never store keys in configuration files
- Support environment variables and secure file storage
- No logging of sensitive information
- Per-provider key isolation

### **Content Filtering**
- Scan diffs for potentially sensitive data
- Warn users about sensitive content inclusion
- Option to exclude certain file patterns
- Respect .gitignore and security patterns

## Testing Strategy

### **Unit Tests**
- All public functions and methods
- Error condition handling
- Configuration parsing and validation
- Provider abstraction correctness

### **Integration Tests**
- End-to-end commit generation flow
- CLI command integration
- Git repository operations
- Mock AI responses for consistent testing

### **Security Tests**
- API key handling verification
- Sensitive data detection
- Configuration validation
- Error message sanitization

## Performance Optimization

### **Caching Strategy**
- Cache AI responses based on diff hashes
- Configurable cache duration
- Memory-based cache for session reuse
- Optional persistent cache for repeated patterns

### **Request Optimization**
- Minimize API calls through intelligent caching
- Batch operations where possible
- Configurable timeouts and retry logic
- Rate limiting awareness

## Future Enhancements

### **Advanced Context Analysis**
- File type detection for better prompts
- Code analysis for semantic understanding
- Integration with issue trackers
- Custom prompt templates per project

### **Multi-language Support**
- Internationalized commit messages
- Locale-aware formatting
- Cultural conventions support

### **Learning Features**
- User feedback incorporation
- Pattern recognition for project-specific styles
- Adaptive prompt improvement
- Usage analytics and optimization

---

## Implementation Status

- ✅ **Phase 1**: Core AI Module (Complete)
- ✅ **Phase 2**: Provider Integration (Complete)  
- 🔄 **Phase 3**: Commit Generation Logic (In Progress)
- ⏳ **Phase 4**: CLI Integration (Pending)

**Next Steps**: Begin Phase 3 implementation with git context extraction and staged changes analysis.