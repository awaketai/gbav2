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

#### 公开接口

```rust
/// 提示词标识符，对应 GBA 所有操作场景。
#[non_exhaustive]
pub enum PromptId {
    /// 初始化：分析仓库结构
    InitAnalyze,
    /// 初始化：生成目录级 gba.md 文档
    InitGbaMd,
    /// 初始化：生成/更新 CLAUDE.md 内容
    InitClaudeMd,
    /// 规划：系统提示词（多轮对话）
    PlanSystem,
    /// 规划：生成设计文档（接口设计/数据建模/核心数据结构）
    PlanDesignSpec,
    /// 规划：生成验证计划
    PlanVerification,
    /// 执行：实现某个开发阶段
    RunPhase,
    /// 执行：Codex Review 审查代码
    RunReview,
    /// 执行：运行验证计划
    RunVerify,
}

/// 传递给模板的上下文数据。
#[non_exhaustive]
#[derive(Debug, Serialize)]
pub struct PromptContext {
    /// 功能标识符（如 "add-auth"）
    pub feature_slug: String,
    /// 当前工作目录
    pub working_dir: PathBuf,
    /// 仓库目录树快照
    pub repo_tree: String,
    /// 设计文档内容
    pub design_spec: String,
    /// 验证计划内容
    pub verification_plan: String,
    /// 当前阶段名称
    pub phase_name: String,
    /// 当前阶段描述
    pub phase_description: String,
    /// Review 发现的问题列表
    pub review_issues: Vec<String>,
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
├── context.rs          # PromptContext 定义
├── prompt_id.rs        # PromptId 枚举
├── manager.rs          # PromptManager 实现
└── templates/          # 默认模板（编译期嵌入）
    ├── init_analyze.jinja
    ├── init_gba_md.jinja
    ├── init_claude_md.jinja
    ├── plan_system.jinja
    ├── plan_design_spec.jinja
    ├── plan_verification.jinja
    ├── run_phase.jinja
    ├── run_review.jinja
    └── run_verify.jinja
```

---

### 5.2 `gba-core` — 核心执行引擎

**职责**：编排所有 GBA 操作，组合提示词渲染（gba-pm）与 Claude Agent SDK 调用。管理 Agent 会话生命周期。

**设计决策**：
- 每个命令（init/plan/run）对应独立的 async 函数
- `AgentSession` 封装 `ClaudeClient`，管理多轮对话
- Runner 使用状态机模式驱动分阶段执行
- 所有操作支持通过 `CancellationToken` 取消

#### 公开接口

```rust
/// GBA 引擎配置。
#[derive(Debug, Deserialize)]
pub struct GbaConfig {
    /// 工作目录
    pub working_dir: PathBuf,
    /// Claude 模型名称
    pub model: String,
    /// 最大对话轮次
    pub max_turns: usize,
    /// 最大预算（美元）
    pub max_budget_usd: f64,
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

#### 内部结构

```
gba-core/src/
├── lib.rs          # 公开 re-exports
├── config.rs       # GbaConfig 配置
├── engine.rs       # GbaEngine 实现
├── event.rs        # GbaEvent 事件枚举
├── session.rs      # AgentSession（ClaudeClient 封装）
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

### 6.1 `gba plan` — 端到端数据流

```
 用户                    CLI (ratatui)           Core               PM              SDK
  │                         │                     │                  │                │
  │─ gba plan slug ────────►│                     │                  │                │
  │                         │── plan("slug") ────►│                  │                │
  │                         │                     │── render(Plan    │                │
  │                         │                     │   System) ──────►│                │
  │                         │                     │◄── system_prompt─│                │
  │                         │                     │── connect() ────────────────────► │
  │                         │◄─ WaitingForInput ──│                  │                │
  │◄─ 显示提示 ─────────────│                     │                  │                │
  │                         │                     │                  │                │
  │── "构建 web 前端" ─────►│                     │                  │                │
  │                         │── send(msg) ────────►│                 │                │
  │                         │                     │── query(msg) ───────────────────► │
  │                         │                     │◄── stream ─────────────────────── │
  │                         │◄─ AssistantMessage ──│                 │                │
  │◄─ 显示响应 ─────────────│                     │                  │                │
  │                         │                     │                  │                │
  │── "需要修改" ──────────►│   ... 多轮对话 ...   │                  │                │
  │                         │                     │                  │                │
  │── "同意" ──────────────►│                     │                  │                │
  │                         │── finalize() ───────►│                 │                │
  │                         │                     │── render(Design  │                │
  │                         │                     │   Spec) ────────►│                │
  │                         │                     │── 写入 specs ────│                │
  │                         │◄─ "请调用 gba run"───│                 │                │
  │◄─ 显示完成 ─────────────│                     │                  │                │
```

### 6.2 `gba run` — 分阶段执行数据流

```
 CLI                        Core                     PM                 SDK
  │                           │                       │                   │
  │── run("slug") ───────────►│                       │                   │
  │                           │── 加载 specs ─────────│                   │
  │                           │                       │                   │
  │  ╔═══════ 步骤 1: 生成目录 ═══════╗               │                   │
  │  ║                               ║               │                   │
  │◄─║── PhaseStarted("生成目录") ────║               │                   │
  │  ╚═══════════════════════════════╝               │                   │
  │                           │                       │                   │
  │  ╔═══════ 步骤 2-3: Phase 1 构建+提交 ══════╗    │                   │
  │  ║                                          ║    │                   │
  │  ║ render(RunPhase) ────────────────────────────►│                   │
  │  ║◄── phase_prompt ────────────────────────────── │                  │
  │  ║ query(prompt) ──────────────────────────────────────────────────► │
  │  ║◄── stream (代码生成) ─────────────────────────────────────────── │
  │◄─║── AssistantMessage ──║                         │                  │
  │  ║ git commit ──────────║ → precommit hook        │                  │
  │◄─║── PhaseCommitted ────║                         │                  │
  │  ╚══════════════════════╝                         │                  │
  │                           │                       │                   │
  │  ╔═══════ 步骤 4-5: Phase 2 测试+提交 ══════╗    │                   │
  │  ║  （同上流程）                              ║   │                   │
  │  ╚══════════════════════════════════════════╝    │                   │
  │                           │                       │                   │
  │  ╔═══════ 步骤 6-7: Codex Review + 修复 ════╗    │                  │
  │  ║                                          ║    │                   │
  │  ║ render(RunReview) ──────────────────────────► │                   │
  │  ║ query(review) ─────────────────────────────────────────────────► │
  │  ║◄── issues ────────────────────────────────────────────────────── │
  │◄─║── IssuesFound ───────║                        │                   │
  │◄─║── FixingIssues ──────║                        │                   │
  │  ║ （循环修复直到无问题）║                        │                   │
  │  ╚══════════════════════╝                        │                   │
  │                           │                       │                   │
  │  ╔═══════ 步骤 8: 验证系统 ═════════════════╗    │                   │
  │  ║                                          ║    │                   │
  │  ║ render(RunVerify) ─────────────────────────► │                    │
  │  ║ query(verify) ──────────────────────────────────────────────────►│
  │◄─║── VerificationResult ║                        │                   │
  │  ╚══════════════════════╝                        │                   │
  │                           │                       │                   │
  │  ╔═══════ 步骤 9: 提交 PR ═════════════════╗     │                   │
  │  ║                                         ║     │                   │
  │  ║ 创建 PR ───────────────────────────────────────────────────────►│
  │◄─║── PrCreated ─────────║                        │                   │
  │  ╚═════════════════════╝                         │                   │
```

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
| 1 | 实现 `PromptId`、`PromptContext` | gba-pm | 定义所有提示词标识和上下文结构 |
| 2 | 实现 `PromptManager` | gba-pm | 模板加载（嵌入 + 覆盖）、minijinja 渲染 |
| 3 | 编写默认模板 | gba-pm | 创建所有 `.jinja` 模板，含正确的变量占位 |
| 4 | 实现 `GbaConfig` | gba-core | 配置结构体，yaml 反序列化 |
| 5 | 实现 `.gba/` 工作区管理 | gba-core | 创建/读写 .gba/ 目录、specs、trees |
| 6 | 实现 `GbaEvent` | gba-core | 面向 UI 的事件枚举 |
| 7 | 实现 `AgentSession` | gba-core | 封装 `ClaudeClient`，支持流式多轮对话 |
| 8 | 单元测试 | 两者 | 模板渲染、配置解析、工作区操作 |

### 第二阶段：核心命令（gba-core）

| # | 任务 | Crate | 描述 |
|---|------|-------|------|
| 1 | 实现 `GbaEngine::init()` | gba-core | 仓库分析、.gba 创建、gba.md 生成、CLAUDE.md 更新 |
| 2 | 实现 `PlanSession` | gba-core | 多轮对话 + spec 生成（design.md / verification.md） |
| 3 | 实现 `GbaEngine::plan()` | gba-core | 串联 PlanSession 与提示词渲染 |
| 4 | 实现分阶段 Runner | gba-core | 9 步流程：目录 → 阶段实现 → 提交 → review → 修复 → 验证 → PR |
| 5 | 实现 Codex Reviewer | gba-core | 代码审查 + 问题修复循环 |
| 6 | 实现 `GbaEngine::run()` | gba-core | 串联 Runner + Reviewer + 验证 + PR 创建 |
| 7 | 集成测试 | gba-core | 使用 mock SDK 测试完整 init/plan/run 流程 |

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
| 3 | 配置文件支持 | gba-core | `.gba/config.yaml` 支持 model/budget 等配置项 |
| 4 | 错误体验优化 | gba-cli | 用户友好的错误消息与建议 |
| 5 | 文档完善 | 全部 | doc comments、README、使用示例 |
