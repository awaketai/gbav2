# GBA - Git-Based Agent CLI

GBA 是一个基于 AI 的开发代理工具，通过结构化工作流自动完成功能开发：**init**（分析仓库）-> **plan**（设计功能）-> **run**（实现、审查、验证并创建 PR）。底层使用 Claude 作为大语言模型，并通过细粒度的工具权限预设在每个阶段实施安全边界控制。

## 架构

```
gba-cli (命令行应用)
  ├── gba-core (核心引擎)
  │     ├── gba-pm (Prompt 模板管理器)
  │     └── claude-agent-sdk-rs (Claude SDK)
  └── ratatui (交互式规划 TUI)
```

| Crate | 说明 |
|-------|------|
| `gba-cli` | 命令行应用，包含 TUI 界面、事件展示和命令路由 |
| `gba-core` | 核心引擎：会话管理、执行流水线、Git 操作 |
| `gba-pm` | 基于 MiniJinja 的 Prompt 模板管理器，模板在编译时内嵌 |

## 前置条件

- Rust stable（2024 edition，版本锁定在 `rust-toolchain.toml`）
- 安装 [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code)（`claude` 命令需在 `$PATH` 中可用）
- 安装 `gh` CLI（用于 `gba run` 中创建 PR）

### Claude API 配置

GBA 底层通过 `claude-agent-sdk-rs` 以子进程方式调用本机安装的 Claude Code CLI（`claude`），**不直接调用 HTTP API**。因此 API URL 和 API Key 的配置完全依赖于 Claude Code CLI 自身的配置：

- **API Key**：设置环境变量 `ANTHROPIC_API_KEY`，或在 Claude Code 的 `~/.claude/settings.json` 中配置
- **API URL**（自定义端点）：设置环境变量 `ANTHROPIC_BASE_URL`，或在 `~/.claude/settings.json` 中配置

```bash
# 环境变量方式
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_BASE_URL="https://api.anthropic.com"  # 可选，默认即为官方端点

# 或在 Claude Code settings 中配置
# ~/.claude/settings.json
{
  "env": {
    "ANTHROPIC_API_KEY": "sk-ant-...",
    "ANTHROPIC_BASE_URL": "https://your-custom-endpoint.com"
  }
}
```

> **注意**：请确保运行 `claude --version` 能正常输出版本号，以验证 CLI 安装正确。

## 安装

```bash
cargo install --path apps/gba-cli
```

或从源码构建：

```bash
make build
```

## 使用说明

### `gba init`

分析当前仓库并生成 GBA 上下文文件。

```bash
gba init
```

该命令会：
1. 创建 `.gba/` 工作区目录结构
2. 通过只读代理会话分析仓库结构
3. 为关键目录生成 `gba.md` 文档
4. 更新 `CLAUDE.md`，补充项目上下文信息

### `gba plan <feature_slug>`

为新功能启动交互式规划会话。

```bash
gba plan add-user-auth
```

该命令会启动 ratatui TUI 界面，你可以与规划代理进行多轮对话。当你对设计方案满意后，结束会话将自动生成：
- `.gba/specs/NNNN_<feature_slug>/design.md` — 包含实现阶段的设计规格文档
- `.gba/specs/NNNN_<feature_slug>/verification.md` — 验证计划

### `gba run <feature_id>`

执行已规划功能的实现方案。

```bash
gba run 0001
```

该命令运行完整的实现流水线：
1. **阶段执行** — 使用完全编码代理，逐阶段实现 `design.md` 中定义的功能
2. **代码审查** — 使用只读代理审查生成的代码；发现问题则进入修复循环
3. **验证** — 执行验证计划，校验实现的正确性
4. **创建 PR** — 提交变更并通过 `gh` 创建 GitHub Pull Request

### 命令选项

```
gba [OPTIONS] <COMMAND>

Options:
  -v, --verbose    启用详细输出（显示完整的代理消息）
  -h, --help       显示帮助信息
  -V, --version    显示版本号
```

## 配置

GBA 使用 `.gba/config.yaml` 进行项目级配置。若文件不存在则使用默认值。

```yaml
sessions:
  init:
    model: claude-sonnet-4-20250514
    max_turns: 3
  plan:
    model: claude-sonnet-4-20250514
    max_turns: 30
  run_phase:
    model: claude-sonnet-4-20250514
    max_turns: 20
  run_review:
    model: claude-sonnet-4-20250514
    max_turns: 5
  run_verify:
    model: claude-sonnet-4-20250514
    max_turns: 10
```

| 字段 | 说明 |
|------|------|
| `model` | 使用的 Claude 模型 |
| `max_turns` | 该阶段代理的最大交互轮次 |

## 工作区结构

初始化后，GBA 创建如下目录结构：

```
.gba/
├── config.yaml              # 项目配置（可选）
├── specs/
│   ├── 0001_feature-slug/
│   │   ├── design.md        # 设计规格文档
│   │   ├── verification.md  # 验证计划
│   │   └── .slug            # 功能标识元数据
│   └── 0002_another-feature/
└── templates/               # 自定义 Prompt 模板覆盖（可选）
```

## 安全模型

GBA 通过 `AgentPreset` 对每个阶段的工具权限进行严格控制：

| 预设 | 允许的工具 | 使用场景 |
|------|-----------|---------|
| `ReadOnly` | Read, Glob, Grep | `init`（分析）、`plan`（对话）、`run`（审查） |
| `WriteSpec` | + Write | `init`（生成文档）、`plan`（生成规格） |
| `FullCoding` | + Edit, Bash | `run`（阶段实现、问题修复） |
| `Verify` | Bash（只读） | `run`（验证） |

## Prompt 模板自定义

GBA 在编译时内嵌了 11 个默认 Prompt 模板。你可以在 `.gba/templates/` 目录中放置同名文件来覆盖默认模板：

```
init_system.jinja       init_analyze.jinja
init_gba_md.jinja       init_claude_md.jinja
plan_system.jinja       plan_design_spec.jinja
plan_verification.jinja
run_system.jinja        run_phase.jinja
run_review.jinja        run_verify.jinja
```

## 开发

```bash
make build          # 构建所有 crate
make check          # 运行 cargo check
make test           # 使用 cargo-nextest 运行测试
make fmt            # 格式化代码（需要 nightly）
make lint           # 运行 clippy（-D warnings）
make lint-pedantic  # 运行 clippy（pedantic 级别）
make audit          # 运行 cargo audit
```

## 许可证

本项目基于 MIT 许可证分发。

详见 [LICENSE](LICENSE.md)。

Copyright 2025 Tyr Chen
