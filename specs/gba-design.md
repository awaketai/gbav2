# GBA（Geektime Bootcamp Agent）设计文档

## 1. 概述

GBA 是一个基于 Claude Agent SDK 的命令行工具，帮助开发者围绕一个代码仓库，通过 Agent 驱动的交互式工作流来规划、设计和实现新功能。它提供三个核心命令：`gba init`、`gba plan`、`gba run`。

## 2. 核心架构

### 2.1 系统分层

```
┌─────────────────────────────────────────────────────────────────┐
│                       GBA CLI (gba-cli)                         │
│                      clap + ratatui TUI                         │
│  ┌───────────┐  ┌─────────────────┐  ┌───────────────────────┐  │
│  │  gba init  │  │  gba plan       │  │  gba run              │  │
│  │            │  │  <feature-slug> │  │  <feature-slug>       │  │
│  └─────┬──────┘  └───────┬─────────┘  └──────────┬────────────┘  │
│        │                 │                       │               │
├────────┴─────────────────┴───────────────────────┴───────────────┤
│                     GBA Core (gba-core)                          │
│               核心执行引擎（tokio async runtime）                 │
│  ┌───────────┐  ┌──────────────┐  ┌────────────────────────────┐ │
│  │ Initializer│  │   Planner    │  │  Runner                    │ │
│  │ (仓库初始化)│  │ (交互式规划) │  │  (分阶段执行+review+验证)  │ │
│  └─────┬──────┘  └──────┬───────┘  └──────────┬─────────────────┘ │
│        │               │                     │                   │
│        └───────────────┼─────────────────────┘                   │
│                        │                                         │
│              ┌─────────▼──────────┐                              │
│              │  AgentSession      │                              │
│              │  (SDK 会话封装)    │                              │
│              └─────────┬──────────┘                              │
├────────────────────────┼─────────────────────────────────────────┤
│            GBA Prompt Manager (gba-pm)                           │
│                  minijinja 模板引擎                               │
│  ┌───────────────┐  ┌──────────────┐  ┌───────────────────────┐ │
│  │ TemplateStore  │  │ PromptEngine │  │ ContextBuilder        │ │
│  │ (加载/管理模板)│  │ (渲染提示词) │  │ (feature-slug, pwd..) │ │
│  └───────────────┘  └──────────────┘  └───────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                  claude-agent-sdk-rs 0.6 (Tyr)                   │
│             (通过子进程与 Claude Code CLI 通信)                   │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Coding Agent 整体工作流

截图中展示的 Coding Agent 工作流是 GBA 的核心理念：

```
                    ┌──────────────┐
     feedback       │              │
     loop ────────► │ Coding Agent │
     │              │              │
     │              └──────┬───────┘
     │                     │
     │                     ▼ 产出三份文档
     │         ┌───────────────────────────┐
     │         │ 1. 开发计划                │
     │         │ 2. Design Spec            │
     │         │    - 接口设计              │
     │         │    - 数据建模              │
     │         │    - 核心数据结构          │
     │         │ 3. 验证计划                │
     │         └───────────┬───────────────┘
     │                     │
     │                     ▼
     │         ┌───────────────────────────┐      precommit hook:
     │         │ Coding Agent              │      - build
     │         │ 高覆盖率的阶段开发        │──────- fmt
     │         │ (阶段 by 阶段)            │      - lint
     │         └───────────┬───────────────┘      - security check
     │                     │
     │                     ▼ 处理问题
     │         ┌───────────────────────────┐
     │         │ Agent Review              │◄────┐
     │         │ 代码审查                  │     │
     │         └───────────┬───────────────┘     │ 修复
基础的        │                     │ 验证问题
功能需求 ◄────┘         Valid Issues │             │
                        │                     │
                        ▼                     │
               ┌─────────────────┐            │
               │ 验证             │────────────┘
               │ (运行验证计划)   │
               └────────┬────────┘
                        │
                        ▼
               ┌─────────────────┐
               │ PR              │
               └─────────────────┘
```

## 3. 项目目录规范

### 3.1 `.gba/` 目录结构

`gba init` 执行后在仓库中创建如下结构：

```
<repo>/
├── .gba/
│   ├── config.yaml                # GBA 全局配置（model, budget 等）
│   ├── templates/                 # 用户自定义提示词模板（可选覆盖默认）
│   ├── 0001_<feature-slug>/       # 功能工作区（四位编号自增）
│   │   ├── specs/
│   │   │   ├── design.md          # 设计文档（接口设计/数据建模/核心数据结构）
│   │   │   └── verification.md    # 验证计划
│   │   └── .trees/                # 仓库结构快照（被 gitignore）
│   └── 0002_<feature-slug>/
│       └── ...
├── CLAUDE.md                      # 更新：添加 GBA 交互信息
└── .gitignore                     # 更新：添加 .gba/**/.trees/
```

### 3.2 仓库级 gba.md 文件

`gba init` 会分析仓库结构，**为每个重要目录生成 `gba.md` 文档**，描述该目录的职责、关键文件和架构信息，供后续 Agent 交互时作为上下文使用。

```
<repo>/
├── src/
│   ├── gba.md          # 描述 src/ 目录的架构和职责
│   └── components/
│       └── gba.md      # 描述 components/ 的职责
├── tests/
│   └── gba.md          # 描述测试目录的结构
└── ...
```

## 4. 命令工作流

### 4.1 `gba init` — 初始化仓库

```
用户执行: $ gba init
               │
               ▼
      ┌─────────────────────┐
      │ 检查 .gba/ 目录      │
      │ 是否已存在？          │
      └────┬──────────┬──────┘
          是           否
          │            │
          ▼            ▼
       退出         ┌───────────────────────────────────┐
     (已初始化)     │ 1. 创建 .gba/ 目录结构             │
                    │ 2. 创建 .gba/trees/ 目录            │
                    │ 3. 分析仓库架构                     │
                    │ 4. 为每个重要目录生成 gba.md 文档    │
                    │ 5. 在 CLAUDE.md 中添加 GBA 交互信息  │
                    └───────────────────────────────────┘

输出: "Initialize current project for GBA"
```

### 4.2 `gba plan <feature-slug>` — 交互式规划

```
用户执行: $ gba plan <feature-slug>
               │
               ▼
      ┌──────────────────────────────┐
      │ 启动 ratatui TUI 交互界面     │
      │ 多轮对话                      │
      └───────────┬──────────────────┘
                  │
                  ▼
      ┌──────────────────────────────────────────────────────┐
      │ Asst: "Can you let me know feature details?"         │
      │ User: "我想构建一个 web 前端，把 gba 的功能放在 web 上" │
      │ Asst: "我打算用这种技术栈来..."                       │
      │ User: "需要修改"                                      │
      │ Asst: "好的，这是修改后的方案...要不要生成 spec？"     │
      │ User: "同意"                                          │
      │ Asst: "开始生成 spec...spec 已生成，请 review。"      │
      │ User: "没意见"                                        │
      └───────────┬──────────────────────────────────────────┘
                  │
                  ▼
      ┌─────────────────────────────────┐
      │ 写入 .gba/<id>_<slug>/specs/    │
      │  - design.md（开发计划 +         │
      │    接口设计/数据建模/核心数据结构）│
      │  - verification.md（验证计划）    │
      └───────────┬─────────────────────┘
                  │
                  ▼
      输出: "Plan finished, 请调用 'gba run' 来执行"
```

### 4.3 `gba run <feature-slug>` — 分阶段执行

截图中明确列出了 9 个执行步骤：

```
用户执行: $ gba run <feature-slug>
               │
               ▼
  ┌────────────────────────────────────────────────────────┐
  │ 步骤 1: 生成目录                                       │
  │    创建功能所需的目录结构                                │
  │                                                        │
  │ 步骤 2: Phase 1 — 构建核心功能（如 observer）           │
  │    Coding Agent 根据 design spec 实现核心代码           │
  │                                                        │
  │ 步骤 3: 提交 Phase 1                                   │
  │    git commit（触发 precommit hook: build/fmt/lint）    │
  │                                                        │
  │ 步骤 4: Phase 2 — 构建测试                              │
  │    Coding Agent 根据验证计划编写测试用例                 │
  │                                                        │
  │ 步骤 5: 提交 Phase 2                                   │
  │    git commit（触发 precommit hook）                    │
  │                                                        │
  │ 步骤 6: Codex Review                                   │
  │    Agent 审查所有生成的代码，识别 valid issues           │
  │                                                        │
  │ 步骤 7: 处理 Review 修复                                │
  │    Agent 修复审查中发现的问题（循环直到通过）            │
  │                                                        │
  │ 步骤 8: 验证系统                                       │
  │    运行验证计划，确保所有检查通过                        │
  │                                                        │
  │ 步骤 9: 提交 PR                                        │
  │    创建 Pull Request                                   │
  └────────────────────────────────────────────────────────┘
```

流程图：

```
  生成目录
     │
     ▼
  Phase 1: 构建核心功能 ──► git commit ──► precommit hook
     │                                      (build/fmt/lint/
     ▼                                       security check)
  Phase 2: 构建测试 ──────► git commit ──► precommit hook
     │
     ▼
  ┌──────────────┐
  │ Codex Review  │◄─────────┐
  │ (Agent 审查)  │          │
  └──────┬───────┘          │
         │                  │
    Valid Issues?           │
     是   │   否             │
     │    │    │             │
     ▼    │    │             │
  处理修复 ─────┘             │
         │                   │
         └───► 无问题 ───────┘
                    │
                    ▼
            ┌──────────────┐
            │ 验证系统      │
            │ (运行验证计划)│
            └──────┬───────┘
                   │
                   ▼
            ┌──────────────┐
            │ 提交 PR       │
            └──────────────┘
```

### 4.4 Pre-commit Hook 集成

每次 `gba run` 中的 git commit 都会触发 pre-commit hook：

```
  ┌─────────┐
  │ commit  │──► pre-commit hook
  └─────────┘       │
                    ├── cargo build
                    ├── cargo +nightly fmt
                    ├── cargo clippy -- -D warnings
                    └── security check（cargo audit）
```

## 5. Crate 设计

### 5.1 `gba-pm` — 提示词管理器

**职责**：加载、管理、渲染提示词模板。不包含任何执行逻辑或 SDK 调用。

**设计决策**：
- 默认模板通过 `include_str!` 在编译期嵌入
- 用户可通过 `.gba/templates/` 覆盖默认模板
- 使用 minijinja 进行模板渲染，接收结构化上下文
- 模板分为 **System Prompt**（设定 Agent 角色与规则）和 **User Prompt**（发出具体任务指令）两类

#### 提示词角色分类

每个 Agent 会话由一个 System Prompt 设定身份，配合一个或多个 User Prompt 驱动具体工作：

| 模板 | 角色 | 所属命令 | 说明 |
|------|------|----------|------|
| `init_system` | **System** | `gba init` | Repo analyst — 精确、事实导向、不投机 |
| `init_analyze` | User | `gba init` | 分析仓库 → 输出 JSON 架构报告 |
| `init_gba_md` | User | `gba init` | 为某个目录生成 `gba.md` 文档 |
| `init_claude_md` | User | `gba init` | 生成 `## GBA Context` 追加到 CLAUDE.md |
| `plan_system` | **System** | `gba plan` | Architect — 协作式多轮规划，不写代码 |
| `plan_design_spec` | User | `gba plan` | 用户确认方案后，生成 design.md |
| `plan_verification` | User | `gba plan` | 紧接 design spec，生成 verification.md |
| `run_system` | **System** | `gba run` | Coding Agent — 代码质量规则、设计文档上下文 |
| `run_phase` | User | `gba run` | 实现一个开发阶段 |
| `run_review` | User | `gba run` | 审查所有生成代码 → 输出 JSON issue 列表 |
| `run_verify` | User | `gba run` | 执行验证计划 → 输出 JSON 结果 |

调用流：

```
gba init:   [init_system] + init_analyze → init_gba_md (×N dirs) → init_claude_md

gba plan:   [plan_system] + 用户真实输入 (×N 轮) → plan_design_spec → plan_verification

gba run:    [run_system]  + run_phase (×N phases, 每个后跟 git commit)
                          → run_review → 修复循环 → run_verify → PR
```

#### 公开接口

```rust
/// 提示词角色，区分 System Prompt 和 User Prompt。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptRole {
    /// System Prompt：设定 Agent 身份、规则、约束。
    /// 在会话创建时传入，整个会话生命周期内不变。
    System,
    /// User Prompt：具体任务指令。
    /// 每次 Agent 调用时作为用户消息发送。
    User,
}

/// 提示词标识符，对应 GBA 所有操作场景。
#[non_exhaustive]
pub enum PromptId {
    // ── gba init ──
    /// System: Repo analyst 角色设定
    InitSystem,
    /// User: 分析仓库结构，输出 JSON
    InitAnalyze,
    /// User: 为某个目录生成 gba.md 文档
    InitGbaMd,
    /// User: 生成/更新 CLAUDE.md 的 GBA 段落
    InitClaudeMd,

    // ── gba plan ──
    /// System: 规划对话的角色设定（多轮对话）
    PlanSystem,
    /// User: 生成设计文档
    PlanDesignSpec,
    /// User: 生成验证计划
    PlanVerification,

    // ── gba run ──
    /// System: Coding Agent 角色设定 + 代码质量规则
    RunSystem,
    /// User: 实现某个开发阶段
    RunPhase,
    /// User: 审查代码，输出 JSON issue 列表
    RunReview,
    /// User: 执行验证计划，输出 JSON 结果
    RunVerify,
}

impl PromptId {
    /// 返回此提示词的角色。
    pub fn role(&self) -> PromptRole {
        match self {
            Self::InitSystem | Self::PlanSystem | Self::RunSystem => PromptRole::System,
            _ => PromptRole::User,
        }
    }
}

/// 传递给模板的上下文数据。
/// 并非所有字段在每个模板中都会使用，未使用的字段保持默认值。
#[non_exhaustive]
#[derive(Debug, Default, Serialize)]
pub struct PromptContext {
    /// 功能标识符（如 "add-auth"）
    pub feature_slug: String,
    /// 功能编号（如 "0001"），用于 spec 文件路径
    pub feature_id: String,
    /// 当前工作目录
    pub working_dir: PathBuf,
    /// 仓库目录树快照
    pub repo_tree: String,
    /// 设计文档内容（run 阶段使用）
    pub design_spec: String,
    /// 验证计划内容（run 阶段使用）
    pub verification_plan: String,
    /// 当前阶段名称（run_phase 使用）
    pub phase_name: String,
    /// 当前阶段描述（run_phase 使用）
    pub phase_description: String,
    /// 当前阶段序号，从 1 开始（run_phase 使用）
    pub phase_index: usize,
    /// 阶段总数（run_phase 使用）
    pub phase_total: usize,
    /// Review 发现的问题列表（run_verify 使用）
    pub review_issues: Vec<String>,
    /// 目录路径，用于 init_gba_md
    pub directory_path: String,
    /// 目录分析上下文，用于 init_gba_md
    pub directory_analysis: String,
    /// 已生成的 gba.md 文件列表，用于 init_claude_md
    pub gba_md_files: Vec<GbaMdEntry>,
}

/// gba.md 文件条目，用于 init_claude_md 模板。
#[derive(Debug, Default, Serialize)]
pub struct GbaMdEntry {
    /// 文件路径（相对于仓库根目录）
    pub path: String,
    /// 该目录的一行摘要
    pub summary: String,
}

/// 提示词管理器：加载与渲染模板。
pub struct PromptManager { .. }

impl PromptManager {
    /// 创建提示词管理器，加载默认模板 + 可选的用户覆盖目录。
    pub fn new(override_dir: Option<&Path>) -> Result<Self>;

    /// 根据 PromptId 和上下文渲染提示词。
    pub fn render(&self, id: PromptId, ctx: &PromptContext) -> Result<String>;
}
```

#### 内部结构

```
gba-pm/src/
├── lib.rs              # 公开 re-exports
├── context.rs          # PromptContext, GbaMdEntry 定义
├── prompt_id.rs        # PromptId, PromptRole 枚举
├── manager.rs          # PromptManager 实现
└── templates/          # 默认模板（编译期嵌入）
    ├── init_system.jinja        # [System] repo analyst 角色
    ├── init_analyze.jinja       # [User]   分析仓库 → JSON
    ├── init_gba_md.jinja        # [User]   生成目录 gba.md
    ├── init_claude_md.jinja     # [User]   生成 CLAUDE.md 段落
    ├── plan_system.jinja        # [System] planning architect 角色
    ├── plan_design_spec.jinja   # [User]   生成 design.md
    ├── plan_verification.jinja  # [User]   生成 verification.md
    ├── run_system.jinja         # [System] coding agent 角色 + 代码规则
    ├── run_phase.jinja          # [User]   实现开发阶段
    ├── run_review.jinja         # [User]   代码审查 → JSON issues
    └── run_verify.jinja         # [User]   验证 → JSON result
```

---

### 5.2 `gba-core` — 核心执行引擎

**职责**：编排所有 GBA 操作，组合提示词渲染（gba-pm）与 Claude Agent SDK 调用。管理 Agent 会话生命周期。**拥有工具权限的安全边界**。

**设计决策**：
- 每个命令（init/plan/run）对应独立的 async 函数
- `AgentSession` 封装 `ClaudeClient`，管理多轮对话
- Runner 使用状态机模式驱动分阶段执行
- 所有操作支持通过 `CancellationToken` 取消
- **工具权限（AgentPreset）硬编码在引擎中**，不对用户开放配置，这是安全边界
- **会话参数（model/max_turns/budget）通过配置文件调优**，允许用户按场景覆盖

#### Agent 工具预设（安全边界）

Agent 在不同阶段需要的工具权限差异很大。**这是安全边界，硬编码在引擎中，不允许用户配置。**

```rust
/// Agent 工具权限预设。
/// 硬编码在引擎中，是安全边界——不通过配置文件暴露。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPreset {
    /// 只读：Read, Glob, Grep。
    /// 用于分析和审查，零副作用。
    ReadOnly,
    /// 写 Spec：Read, Glob, Grep, Write。
    /// 仅限生成 .gba/ 下的文档文件。
    WriteSpec,
    /// 完整编码：全部工具（Read, Write, Edit, Bash, Glob, Grep）。
    /// 用于实现阶段，可以创建文件、运行 build/test。
    FullCoding,
    /// 验证：Read, Glob, Grep, Bash（只读命令）。
    /// 可运行 cargo build/test/clippy，但不可修改文件。
    Verify,
}
```

各步骤到预设的映射：

| 步骤 | AgentPreset | System Prompt | User Prompt | Read/Glob/Grep | Write/Edit | Bash |
|------|-------------|---------------|-------------|:-:|:-:|:-:|
| `init` 分析仓库 | `ReadOnly` | `init_system` | `init_analyze` | yes | **no** | **no** |
| `init` 生成 gba.md | `WriteSpec` | `init_system` | `init_gba_md` | yes | write | **no** |
| `init` 更新 CLAUDE.md | `WriteSpec` | `init_system` | `init_claude_md` | yes | edit | **no** |
| `plan` 多轮对话 | `ReadOnly` | `plan_system` | 用户真实输入 | yes | **no** | **no** |
| `plan` 生成 design | `WriteSpec` | `plan_system` | `plan_design_spec` | yes | write | **no** |
| `plan` 生成 verification | `WriteSpec` | `plan_system` | `plan_verification` | yes | write | **no** |
| `run` 实现阶段 | `FullCoding` | `run_system` | `run_phase` | yes | yes | **yes** |
| `run` 代码审查 | `ReadOnly` | `run_system` | `run_review` | yes | **no** | **no** |
| `run` 修复问题 | `FullCoding` | `run_system` | `run_phase`(fix) | yes | yes | **yes** |
| `run` 验证 | `Verify` | `run_system` | `run_verify` | yes | **no** | **yes** |

**设计原则**：
- **`run_review` 必须是 ReadOnly** — 审查者不能同时修改代码，否则 review-fix 循环的职责分离被破坏
- **`run_verify` 只能运行命令不能改文件** — 如果验证失败，应回到 `run_phase` 修复，而非在验证阶段偷偷改代码
- **`plan` 对话阶段是 ReadOnly** — 架构师只探索代码理解上下文，不做任何修改

#### 会话配置（用户可调优）

工具权限之外的参数通过 `.gba/config.yaml` 配置，允许用户按场景调优：

```yaml
# .gba/config.yaml
sessions:
  init:
    model: claude-sonnet-4-20250514
    max_turns: 3
  plan:
    model: claude-sonnet-4-20250514
    max_turns: 30          # 多轮对话需要更多轮次
  run_phase:
    model: claude-sonnet-4-20250514
    max_turns: 20
  run_review:
    model: claude-sonnet-4-20250514    # 可用更便宜的模型
    max_turns: 5
  run_verify:
    model: claude-sonnet-4-20250514
    max_turns: 10
```

#### 公开接口

```rust
/// GBA 引擎全局配置。
#[derive(Debug, Deserialize)]
pub struct GbaConfig {
    /// 工作目录
    pub working_dir: PathBuf,
    /// 各场景的会话配置
    pub sessions: SessionsConfig,
}

/// 各场景的会话参数配置（用户可调优）。
#[derive(Debug, Deserialize)]
pub struct SessionsConfig {
    pub init: SessionConfig,
    pub plan: SessionConfig,
    pub run_phase: SessionConfig,
    pub run_review: SessionConfig,
    pub run_verify: SessionConfig,
}

/// 单个场景的会话参数。
#[derive(Debug, Deserialize)]
pub struct SessionConfig {
    /// Claude 模型名称
    pub model: String,
    /// 最大对话轮次
    pub max_turns: usize,
}

/// 执行过程中产生的事件，供 UI 层消费。
#[non_exhaustive]
pub enum GbaEvent {
    /// Agent 发送了文本消息
    AssistantMessage(String),
    /// 等待用户输入
    WaitingForInput,
    /// 阶段开始
    PhaseStarted { name: String, index: usize, total: usize },
    /// 阶段完成并已提交
    PhaseCommitted { name: String },
    /// Codex Review 开始
    ReviewStarted,
    /// Review 发现问题
    IssuesFound(Vec<String>),
    /// 正在修复 Review 问题
    FixingIssues,
    /// 验证结果
    VerificationResult { passed: bool, details: String },
    /// PR 已创建
    PrCreated { url: String },
    /// 执行出错
    Error(String),
}

/// GBA 核心引擎。
pub struct GbaEngine { .. }

impl GbaEngine {
    /// 使用配置和提示词管理器创建引擎。
    pub fn new(config: GbaConfig, prompt_manager: PromptManager) -> Result<Self>;

    /// 初始化当前仓库：
    /// 创建 .gba/ 目录、分析仓库、生成 gba.md、更新 CLAUDE.md。
    pub async fn init(&self) -> Result<impl Stream<Item = GbaEvent>>;

    /// 启动交互式规划会话，返回 PlanSession 用于多轮对话。
    pub async fn plan(&self, feature_slug: &str) -> Result<PlanSession>;

    /// 根据 specs 分阶段执行功能开发：
    /// 生成目录 → 各阶段实现+提交 → review → 修复 → 验证 → PR。
    pub async fn run(&self, feature_slug: &str) -> Result<impl Stream<Item = GbaEvent>>;
}

/// 多轮规划会话，用于 gba plan 的交互式对话。
pub struct PlanSession { .. }

impl PlanSession {
    /// 发送用户消息，接收 Agent 响应。
    pub async fn send(&mut self, message: &str) -> Result<GbaEvent>;

    /// 确认规划完成，生成 design.md 和 verification.md。
    pub async fn finalize(&mut self) -> Result<()>;
}
```

#### AgentSession 内部设计

`AgentSession` 封装 `ClaudeClient`，根据 `AgentPreset` 配置工具权限：

```rust
/// AgentSession 管理与 Claude Agent SDK 的单次会话。
struct AgentSession {
    client: ClaudeClient,
    preset: AgentPreset,
    config: SessionConfig,
    system_prompt: String,
}

impl AgentSession {
    /// 创建新会话。根据 preset 自动配置 allowed_tools。
    fn new(preset: AgentPreset, config: &SessionConfig, system_prompt: String) -> Result<Self>;

    /// 发送用户消息，返回 Agent 响应流。
    async fn send(&mut self, user_prompt: &str) -> Result<impl Stream<Item = AgentMessage>>;

    /// 返回此预设对应的 Claude Code allowed_tools 列表。
    fn allowed_tools(preset: AgentPreset) -> Vec<String> {
        match preset {
            AgentPreset::ReadOnly   => vec!["Read", "Glob", "Grep"],
            AgentPreset::WriteSpec  => vec!["Read", "Glob", "Grep", "Write"],
            AgentPreset::FullCoding => vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
            AgentPreset::Verify    => vec!["Read", "Glob", "Grep", "Bash"],
        }
    }
}
```

#### 内部结构

```
gba-core/src/
├── lib.rs          # 公开 re-exports
├── config.rs       # GbaConfig, SessionsConfig, SessionConfig
├── preset.rs       # AgentPreset 枚举 + allowed_tools 映射
├── engine.rs       # GbaEngine 实现
├── event.rs        # GbaEvent 事件枚举
├── session.rs      # AgentSession（ClaudeClient 封装 + preset 应用）
├── plan.rs         # PlanSession（多轮规划对话）
├── runner.rs       # 分阶段执行逻辑（9 步流程）
├── reviewer.rs     # Codex Review + 修复循环
├── workspace.rs    # .gba/ 目录管理（specs/trees 读写）
└── error.rs        # GbaCoreError 错误枚举
```

---

### 5.3 `gba-cli` — 命令行界面

**职责**：解析命令行参数，管理 TUI 渲染，桥接用户交互与 `gba-core`。不包含任何业务逻辑。

**设计决策**：
- `clap` 解析子命令
- `ratatui` TUI 用于 `gba plan` 交互式对话
- `gba init` 和 `gba run` 使用流式输出到终端
- 消费 `GbaEvent` 流驱动 UI 更新

#### CLI 用法

```
gba 0.1.0 — Geektime Bootcamp Agent CLI

用法:
    gba [选项] <命令>

选项:
    -v, --verbose    启用详细日志
    -h, --help       显示帮助
    -V, --version    显示版本

命令:
    init             初始化当前仓库为 GBA 项目
    plan <slug>      为功能启动交互式规划
    run <slug>       执行已规划的功能
```

#### 内部结构

```
gba-cli/src/
├── main.rs         # 入口，clap CLI 定义
├── commands/
│   ├── mod.rs
│   ├── init.rs     # 处理 gba init，消费事件流输出
│   ├── plan.rs     # 处理 gba plan，启动 TUI
│   └── run.rs      # 处理 gba run，消费事件流输出
└── tui/
    ├── mod.rs
    ├── app.rs      # TUI 应用状态
    ├── ui.rs       # ratatui 布局与渲染
    └── event.rs    # 终端事件处理（键盘输入、窗口调整）
```

## 6. 数据流

### 6.1 `gba init` — 端到端数据流

```
 CLI                        Core                     PM                 SDK
  │                           │                       │                   │
  │── init() ────────────────►│                       │                   │
  │                           │                       │                   │
  │  ╔═══ Session 1: 分析仓库 (ReadOnly) ═══════════╗ │                   │
  │  ║                                              ║ │                   │
  │  ║ render(InitSystem) ─────────────────────────►║ │                   │
  │  ║◄── system_prompt ────────────────────────────║─│                   │
  │  ║ render(InitAnalyze) ────────────────────────►║ │                   │
  │  ║◄── user_prompt ─────────────────────────────║─│                   │
  │  ║ new_session(ReadOnly, system) ──────────────║─────────────────► │
  │  ║ send(user_prompt) ─────────────────────────║──────────────────► │
  │  ║◄── JSON(架构分析结果) ──────────────────────║──────────────────── │
  │  ╚═════════════════════════════════════════════╝ │                   │
  │                           │                       │                   │
  │  ╔═══ Session 2..N: 生成 gba.md (WriteSpec) ══╗  │                   │
  │  ║ for each important directory:               ║  │                   │
  │  ║   render(InitGbaMd, {dir_path, analysis}) ──║─►│                   │
  │  ║   send(user_prompt) ────────────────────────║───────────────────► │
  │  ║   Agent writes gba.md ──────────────────────║───────────────────► │
  │  ╚═════════════════════════════════════════════╝  │                   │
  │                           │                       │                   │
  │  ╔═══ Session N+1: 更新 CLAUDE.md (WriteSpec) ═╗  │                   │
  │  ║ render(InitClaudeMd, {gba_md_files}) ───────║─►│                   │
  │  ║ send(user_prompt) ─────────────────────────║────────────────────► │
  │  ║ Agent edits CLAUDE.md ──────────────────────║───────────────────► │
  │  ╚═════════════════════════════════════════════╝  │                   │
  │                           │                       │                   │
  │◄── "Initialized" ─────────│                       │                   │
```

### 6.2 `gba plan` — 端到端数据流

```
 用户                    CLI (ratatui)           Core               PM              SDK
  │                         │                     │                  │                │
  │─ gba plan slug ────────►│                     │                  │                │
  │                         │── plan("slug") ────►│                  │                │
  │                         │                     │                  │                │
  │  ╔═══ 单一会话: plan_system 作为 System Prompt (ReadOnly) ═══════════════════╗  │
  │  ║                                                                           ║  │
  │  ║ render(PlanSystem) → system_prompt                                        ║  │
  │  ║ new_session(ReadOnly, system_prompt) ──────────────────────────────────► ║  │
  │  ║                                                                           ║  │
  │  ║  ┌─── 多轮对话 (用户真实输入) ───────────────────────────────────────────┐ ║  │
  │  ║  │                                                                      │ ║  │
  │  ║  │ 用户: "构建 web 前端"  → send(msg) → query(msg) ──────────────────► │ ║  │
  │  ║  │◄── AssistantMessage ◄── stream ◄──────────────────────────────────── │ ║  │
  │  ║  │                                                                      │ ║  │
  │  ║  │ 用户: "需要修改"       → send(msg) → query(msg) ──────────────────► │ ║  │
  │  ║  │◄── AssistantMessage ◄── stream ◄──────────────────────────────────── │ ║  │
  │  ║  │                                                                      │ ║  │
  │  ║  └──────────────────────────────────────────────────────────────────────┘ ║  │
  │  ║                                                                           ║  │
  │  ║  ┌─── finalize(): 切换到 WriteSpec ─────────────────────────────────────┐ ║  │
  │  ║  │                                                                      │ ║  │
  │  ║  │ render(PlanDesignSpec) → user_prompt                                 │ ║  │
  │  ║  │ send(user_prompt) → Agent writes design.md ──────────────────────► │ ║  │
  │  ║  │                                                                      │ ║  │
  │  ║  │ render(PlanVerification) → user_prompt                               │ ║  │
  │  ║  │ send(user_prompt) → Agent writes verification.md ────────────────► │ ║  │
  │  ║  │                                                                      │ ║  │
  │  ║  └──────────────────────────────────────────────────────────────────────┘ ║  │
  │  ╚═══════════════════════════════════════════════════════════════════════════╝  │
  │                         │                     │                  │                │
  │◄─ "Plan finished" ─────│                     │                  │                │
```

**注意**：`plan` 会话在 `finalize()` 时需要从 `ReadOnly` 升级到 `WriteSpec`，有两种实现策略：
1. **新建子会话**：finalize 时创建新的 `WriteSpec` 会话，把对话历史摘要 + design spec/verification 指令作为 user prompt 发送
2. **单会话预授权**：整个 plan 会话使用 `WriteSpec`，但 system prompt 中约束 Agent 只在收到明确指令后才写文件

推荐策略 1（新建子会话），因为它在权限层面强制保证对话阶段不可能有写操作。

### 6.3 `gba run` — 分阶段执行数据流

```
 CLI                        Core                     PM                 SDK
  │                           │                       │                   │
  │── run("slug") ───────────►│                       │                   │
  │                           │── 加载 specs ─────────│                   │
  │                           │── render(RunSystem) ──►│                  │
  │                           │◄── system_prompt ──────│                  │
  │                           │                       │                   │
  │  ╔═══════ 步骤 1-5: 分阶段实现 (FullCoding) ══════╗│                  │
  │  ║ for each phase in design_spec:                 ║│                  │
  │  ║                                                ║│                  │
  │  ║   render(RunPhase, {phase_N}) ────────────────►║│                  │
  │  ║◄── user_prompt ────────────────────────────────║│                  │
  │  ║   new_session(FullCoding, system) ─────────────║─────────────────►│
  │  ║   send(user_prompt) ──────────────────────────║──────────────────►│
  │  ║◄── stream (代码生成) ──────────────────────────║──────────────────│
  │◄─║── PhaseStarted/AssistantMessage ──║            ║│                  │
  │  ║   git commit → precommit hook     ║            ║│                  │
  │◄─║── PhaseCommitted ─────────────────║            ║│                  │
  │  ╚════════════════════════════════════════════════╝│                  │
  │                           │                       │                   │
  │  ╔═══════ 步骤 6: Codex Review (ReadOnly) ════════╗│                  │
  │  ║                                                ║│                  │
  │  ║ render(RunReview) ────────────────────────────►║│                  │
  │  ║ new_session(ReadOnly, system) ─────────────────║─────────────────►│
  │  ║ send(review_prompt) ──────────────────────────║──────────────────►│
  │  ║◄── JSON(issues) ──────────────────────────────║──────────────────│
  │◄─║── ReviewStarted / IssuesFound ────║            ║│                  │
  │  ╚════════════════════════════════════════════════╝│                  │
  │                           │                       │                   │
  │  ╔═══════ 步骤 7: 修复循环 (FullCoding) ══════════╗│                  │
  │  ║ while issues not empty:                        ║│                  │
  │  ║   new_session(FullCoding, system) → fix ───────║─────────────────►│
  │  ║   new_session(ReadOnly, system) → re-review ───║─────────────────►│
  │◄─║── FixingIssues ───────────────────║            ║│                  │
  │  ╚════════════════════════════════════════════════╝│                  │
  │                           │                       │                   │
  │  ╔═══════ 步骤 8: 验证 (Verify) ═════════════════╗│                  │
  │  ║                                                ║│                  │
  │  ║ render(RunVerify) ────────────────────────────►║│                  │
  │  ║ new_session(Verify, system) ───────────────────║─────────────────►│
  │  ║ send(verify_prompt) ──────────────────────────║──────────────────►│
  │  ║◄── JSON(result) ──────────────────────────────║──────────────────│
  │◄─║── VerificationResult ─────────────║            ║│                  │
  │  ╚════════════════════════════════════════════════╝│                  │
  │                           │                       │                   │
  │  ╔═══════ 步骤 9: 提交 PR (FullCoding) ══════════╗│                  │
  │  ║                                                ║│                  │
  │  ║ new_session(FullCoding, system) → 创建 PR ─────║─────────────────►│
  │◄─║── PrCreated ──────────────────────║            ║│                  │
  │  ╚════════════════════════════════════════════════╝│                  │
```

**每个步骤使用独立会话**，确保：
1. 工具权限精确匹配当前步骤需求
2. Review 会话物理隔离于 Coding 会话，无法偷偷修改代码
3. 验证会话无写权限，若失败则回到修复循环而非自行修改

## 7. 错误处理策略

```
gba-cli:  anyhow::Result  ──► 面向用户的错误展示
              │
gba-core: GbaCoreError    ──► 领域错误枚举（thiserror）
              │
              ├── ConfigError       — 配置无效 / .gba 目录缺失
              ├── WorkspaceError    — .gba 目录操作失败
              ├── SessionError      — Claude SDK 通信异常
              ├── PlanError         — 规划会话失败
              ├── RunError          — 分阶段执行失败
              └── ReviewError       — 代码审查失败
              │
gba-pm:   GbaPmError       ──► 模板错误枚举（thiserror）
              │
              ├── TemplateNotFound  — 未知 PromptId
              ├── RenderError       — minijinja 渲染失败
              └── LoadError         — 模板文件 I/O 错误
```

## 8. 开发计划

### 第一阶段：基础设施（gba-pm + gba-core 骨架）

| # | 任务 | Crate | 描述 |
|---|------|-------|------|
| 1 | 实现 `PromptId`、`PromptRole`、`PromptContext` | gba-pm | 定义提示词标识（含 3 个 System + 8 个 User）、角色枚举、上下文结构（含新增字段） |
| 2 | 实现 `PromptManager` | gba-pm | 模板加载（嵌入 + 覆盖）、minijinja 渲染 |
| 3 | 编写默认模板 | gba-pm | 创建 11 个 `.jinja` 模板（3 system + 8 user），含正确的变量占位 |
| 4 | 实现 `GbaConfig`、`SessionsConfig`、`SessionConfig` | gba-core | 配置结构体，yaml 反序列化，含各场景会话参数 |
| 5 | 实现 `AgentPreset` | gba-core | 工具权限预设枚举 + `allowed_tools()` 映射 |
| 6 | 实现 `.gba/` 工作区管理 | gba-core | 创建/读写 .gba/ 目录、specs、trees |
| 7 | 实现 `GbaEvent` | gba-core | 面向 UI 的事件枚举 |
| 8 | 实现 `AgentSession` | gba-core | 封装 `ClaudeClient`，根据 `AgentPreset` 配置工具权限，支持流式多轮对话 |
| 9 | 单元测试 | 两者 | 模板渲染、配置解析、工作区操作、preset 映射 |

### 第二阶段：核心命令（gba-core）

| # | 任务 | Crate | 描述 |
|---|------|-------|------|
| 1 | 实现 `GbaEngine::init()` | gba-core | ReadOnly 会话分析仓库 → WriteSpec 会话生成 gba.md → WriteSpec 会话更新 CLAUDE.md |
| 2 | 实现 `PlanSession` | gba-core | ReadOnly 多轮对话 + finalize 时升级为 WriteSpec 子会话生成 spec |
| 3 | 实现 `GbaEngine::plan()` | gba-core | 串联 PlanSession 与提示词渲染 |
| 4 | 实现分阶段 Runner | gba-core | FullCoding 会话执行各阶段 + git commit |
| 5 | 实现 Codex Reviewer | gba-core | ReadOnly 会话审查 + FullCoding 会话修复循环 |
| 6 | 实现 `GbaEngine::run()` | gba-core | 串联 Runner(FullCoding) + Reviewer(ReadOnly) + Verify(Verify) + PR(FullCoding) |
| 7 | 集成测试 | gba-core | 使用 mock SDK 测试完整 init/plan/run 流程，验证 preset 隔离 |

### 第三阶段：CLI 与 TUI（gba-cli）

| # | 任务 | Crate | 描述 |
|---|------|-------|------|
| 1 | 实现 clap 子命令 | gba-cli | `init`、`plan <slug>`、`run <slug>` |
| 2 | 实现 `gba init` 命令处理 | gba-cli | 消费 GbaEvent 流，输出到终端 |
| 3 | 实现 `gba plan` ratatui TUI | gba-cli | 聊天式交互界面（输入/输出面板） |
| 4 | 实现 `gba run` 命令处理 | gba-cli | 进度展示、阶段追踪 |
| 5 | 端到端测试 | 全部 | 对真实仓库测试完整工作流 |

### 第四阶段：完善与加固

| # | 任务 | Crate | 描述 |
|---|------|-------|------|
| 1 | Pre-commit hook 配置 | gba-core | 自动配置 build/fmt/lint/security check |
| 2 | 优雅取消 | 全部 | 处理 Ctrl+C，清理部分状态 |
| 3 | 配置文件支持 | gba-core | `.gba/config.yaml` 支持 sessions/model/max_turns 等场景级配置项 |
| 4 | 错误体验优化 | gba-cli | 用户友好的错误消息与建议 |
| 5 | 文档完善 | 全部 | doc comments、README、使用示例 |
| 6 | Preset 安全审计 | gba-core | 验证各场景 preset 隔离，确保 review/verify 不可写 |
