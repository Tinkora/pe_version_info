# Tinkora PE Version Info

> **状态：Idea / L0**——当前仓库只包含产品与实现规划，尚未包含可用实现。

[English](README.md)

[![在 Ko-fi 上支持 Tinkora](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/tinkora)

一个面向 Windows PE 文件的跨平台工具规划：读取和修改 EXE/DLL 的 `VERSIONINFO`，替换程序图标，并将整个流程提供给人类用户和 AI Agent 使用。

规划中的产品分为三层：

- `pevi`：使用 Rust 编写的确定性 CLI/库，可在 Windows、macOS、Linux 上修改已有 `.exe` 或 `.dll`。
- `pevi` Codex Skill：配置模板与安全的 Agent 操作规范，便于接入重复的构建流程。
- 可选 MCP Server/UI：为支持 MCP Apps 的宿主提供结构化检查、预览、确认和文件选择。

首版计划使用 [`editpe`](https://github.com/Systemcluster/editpe) 进行跨平台 PE 资源解析与重建。 [`winresource`](https://github.com/BenjaminRi/winresource) 更适合 Rust 应用构建时嵌入 Windows 资源，不作为修改任意既有 EXE 的主引擎。

本项目尚未达到生产可用状态。在签名失效、备份、原子替换和写后验证全部实现并测试前，不要用它修改已签名的正式发布文件。

## 规划

详细实现计划见 [`docs/superpowers/plans/2026_08_13_pe_version_info.md`](docs/superpowers/plans/2026_08_13_pe_version_info.md)。

架构决策与调研记录见 [`docs/decisions/`](docs/decisions/)。
