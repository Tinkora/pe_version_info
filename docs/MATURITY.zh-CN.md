# 成熟度与能力标签

从 `v0.1.0-alpha.4` 起，native CLI 为 **Alpha**：核心成功、无效输入、边界、
干净消费环境、独立 Windows 资源以及真实 Authenticode 修改前后行为已经在 tag
commit 的托管检查中通过。Codex Skill/plugin 仍为 **Draft**，直到记录一次
fresh-agent 验收；它不是 Agent-callable MCP 发布物。

能力标签彼此独立：仓库包含 **Draft Codex Skill** 和版本化 JSON Schema，但没有
MCP transport，因此不是 **Agent-callable**。SVG/PDF 和 MCP/UI 属于后续范围。

晋级必须基于准确 commit 的可审查证据、当前发布/安全/支持/变更日志文档、可复现产物和受保护发布治理。本地测试成功或未执行的 workflow 不能晋级成熟度。
