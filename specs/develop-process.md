# Development Process

Multi-model collaborative development workflow.

## Flow Diagram

```mermaid
flowchart TD
    A["1. Gemini Deep Research\n(Deep research & analysis)"] --> B["2. CC Opus 4.6\n(Design implementation plan)"]
    B --> C["3. GLM5\n(Optimize plan)"]
    C --> D["4. CC Opus 4.6 / Sonnet 4.6\n(Refine & improve plan)"]
    D --> E["5. GLM5\n(Implement code)"]
    E --> F["6. Codex GPT 5.4\n(Code review)"]
    F --> G["7. GLM5 / CC Opus 4.6\n(Optimize based on review)"]
    G --> H["8. Codex GPT 5.4\n(Code review round 2)"]
    H --> I["9. Codex GPT 5.4\n(Deep discussion & suggestions)"]
    I -->|"Need further iteration"| G

    style A fill:#4285F4,color:#fff
    style B fill:#7B61FF,color:#fff
    style C fill:#FF6B35,color:#fff
    style D fill:#7B61FF,color:#fff
    style E fill:#FF6B35,color:#fff
    style F fill:#10A37F,color:#fff
    style G fill:#FF6B35,color:#fff
    style H fill:#10A37F,color:#fff
    style I fill:#10A37F,color:#fff
```

## Sequence Diagram

```mermaid
sequenceDiagram
    participant Gemini as Gemini
    participant CC as Claude Code<br/>(Opus 4.6 / Sonnet 4.6)
    participant GLM as GLM5
    participant Codex as Codex<br/>(GPT 5.4)

    Note over Gemini: Phase 1: Research
    Gemini->>CC: Research results

    Note over CC: Phase 2: Design
    CC->>GLM: Implementation plan

    Note over GLM: Phase 3: Optimize Plan
    GLM->>CC: Optimized plan

    Note over CC: Phase 4: Refine Plan
    CC->>GLM: Final plan

    Note over GLM: Phase 5: Implement
    GLM->>Codex: Code

    Note over Codex: Phase 6: Code Review (Round 1)
    Codex->>GLM: Review feedback
    Codex->>CC: Review feedback

    Note over GLM,CC: Phase 7: Optimize Code
    GLM->>Codex: Updated code
    CC->>Codex: Updated code

    Note over Codex: Phase 8: Code Review (Round 2)
    Codex->>Codex: Phase 9: Deep discussion & suggestions

    opt Need further iteration
        Codex->>GLM: Optimization suggestions
        Codex->>CC: Optimization suggestions
        GLM->>Codex: Updated code
    end
```

## Role Summary

| Model | Role |
|-------|------|
| **Gemini** | Deep research & background analysis |
| **Claude Code (Opus 4.6 / Sonnet 4.6)** | Architecture design, plan refinement, code optimization |
| **GLM5** | Plan optimization, primary code implementation, code optimization |
| **Codex (GPT 5.4)** | Code review, deep discussion, quality assurance |
