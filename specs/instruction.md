## Instructions

```
cargo install cargo-generate

cargo generate tyrchen/rust-lib-template
```

## 初始化项目

这是一个 geektime bootcamp agent(GBA)，项目，它的主要功能是封装 claude agent sdk，让用户可以很方便的围绕一个 repo 来添加新的功能。请把这个 Rust 项目转换成一个 workspacee，里面包含 crates/gba-core(core execute engine)，crates/gba-pm(prompt manager)，以及 apps/gba-cli(command line interface)。生成的是一个 gba cli。所有的 deps 放在 workspace 下，各个 crate 通过 `xx= { workspace = true }` 来引用。cli使用 clap / ratatui 来构建。prompt manager 使用 minijinja 来构建。core execute engine 使用 tokio / claude-agent-sdk-rs 0.6 来构建。所有 deps 都要使用最新版本。

先不要撰写代码，生成各个 crate 的 skeleton 即可。

## 生成设计文档
根据截图，生成设计文档

- 包括核心架构的 ascii diagram 以及重要的流程
- 各个 crate 有清晰的职责和 public interface
- gba-core: 核心的执行引擎，根据不同场景下的 prompt，调用 claude agent sdk 来执行任务。务必提供精简可用的接口
- gba-pm: 提示词管理器，负责加载、渲染、管理提示词，务必提供精简可用的接口
- gba-cli: 命令行界面，负责与用户交互，并调用 gba-core 来执行任务
- 代码结构尽可能职责单一，不要出现重复代码，follow SOLID principles，尽可能使用 rust 的最新特性
- 提供开发者计划，包括每个阶段的任务

设计文档放在 ./specs 下

## 设计文档优化1

1.task kind 应该有 verification
2.任务执行结果应该记录 turns / cost，放在 state.yml 中，最后的额 PR link 也放进去
3.在 `gba run` 过程中，如果中断，下次运行可以继续恢复（在提示词中提现）
4.预先思考好所有场景下的提示词，放在 crates/gba-pm/templates 下，我来review，提示词用英文

## 提示词优化

目前这些提示词哪些是作为 system prompt 添加到 claude code 系统提示词中，哪些是作为 user prompt 来驱动完成工作？比如 `gba init` 的 user prompt 是什么？

请思考在不同的场景下，哪些需要 claude code preset，哪些不需要，哪些需要完整的工具，哪些不需要，这个应该在哪里定义，是写在 engine 中，还是配置中？

## 构建 gba

