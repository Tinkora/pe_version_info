# 成熟度与能力标签

当前仓库状态为 **Draft**。核心成功、无效输入、边界和失败行为已经在准确推送的候选 commit 上通过托管 native、文档和供应链检查。Alpha 仍要求干净消费环境与 fresh-agent Skill 验收、独立 Windows 资源检查、真实 Authenticode 修改前后证据，以及完整发布产物和受保护 tag 证据。候选未通过全部门槛前，不得宣称 Alpha 或 Human-usable 发布成熟度。

能力标签彼此独立：仓库包含 **Draft Codex Skill** 和版本化 JSON Schema，但没有 MCP transport，因此不是 **Agent-callable**。SVG/PDF 和 MCP/UI 属于后续范围；独立 Windows 资源与 Authenticode 验证是 Alpha 门槛。

晋级必须基于准确 commit 的可审查证据、当前发布/安全/支持/变更日志文档、可复现产物和受保护发布治理。本地测试成功或未执行的 workflow 不能晋级成熟度。
