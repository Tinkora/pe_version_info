# Tinkora PE Version Info 产品规格

状态：Alpha CLI / Draft Codex Skill
日期：2026-08-13
仓库：<https://github.com/Tinkora/pe_version_info>

## 1. 要解决的问题

Windows 资源管理器通过 PE 文件中的 `VERSIONINFO` 资源显示产品信息。现有方案通常只能覆盖局部场景：

- Rust 应用可以在 Windows 构建阶段使用 `winresource` 嵌入资源；
- `rcedit` 可以修改常见字段和图标，但不能覆盖所有 VERSIONINFO 语言与资源表要求；
- GUI 资源编辑器难以复现到 CI，也不适合 AI Agent 稳定调用。

因此每个发布流程都要重复编写脆弱脚本：设置产品名、版本、描述、版权、语言和图标，检查结果，并证明最终 EXE 真的包含这些值。

## 2. 目标用户

1. 需要在 Windows、macOS 或 Linux 上修改已有 `.exe`/`.dll` 的开发者或发布工程师；
2. 需要确定性配置文件、预览报告、机器可读错误且不依赖隐藏 GUI 状态的 AI coding agent；
3. 希望在 CI 中固定一个版本化二进制和一条可复现命令的构建维护者；
4. 希望上传/选择图标源文件、逐项编辑字段、预览并确认写入的 ChatGPT/Codex 用户。

## 3. 明确不做的事情

- 不做 PE 反汇编、加壳、恶意软件分析或可执行文件优化；
- 不承诺修改资源后仍保持 Authenticode 签名有效；
- 不推测用户没有提供的法律、版权和产品信息；
- 不承诺支持所有文件格式。首个发布候选支持 PNG、JPEG、ICO；SVG 和 PDF 在 renderer 上限、许可证与 runtime 分发可复现后再加入，其他格式必须明确失败；
- 不静默覆盖输入文件；
- CLI/Skill 模式默认不把本地 EXE 或图标内容上传到远端服务。

## 4. 产品组成

### 4.1 Core 库

`pe_version_info_core` 负责 PE32/PE32+ 检测、资源解析与重建、VERSIONINFO 更新、主图标替换、签名策略和稳定诊断。Core 不依赖桌面 UI、MCP 或网络。

### 4.2 CLI

`pevi` 是自动化规范接口：

```text
pevi init --output pevi.toml
pevi inspect --input dist/app.exe --format json
pevi plan --config pevi.toml
pevi apply --config pevi.toml --output dist/app-versioned.exe
pevi verify --input dist/app-versioned.exe --format json
pevi convert-icon --input assets/logo.png --output build/logo.ico
```

CLI 必须支持 Windows、macOS、Linux，并对校验、格式、签名策略和写入错误返回非零退出码。

### 4.3 Codex Skill

Skill 指导 Agent 创建/读取配置、解析路径、先 `inspect` 再 `apply`、使用 `plan` 摘要解释变化、避免静默覆盖、写入后执行 `verify`，以及准确报告不支持的格式和签名后果。

### 4.4 可选 MCP Server/UI

MCP Server 暴露同一组结构化操作。只有检查、编辑、确认等需要可视化的工具附加 UI；没有 UI 时工具仍必须可用。

ChatGPT/Codex 自定义 UI 运行在 MCP Apps iframe 中。文件选择/上传能力取决于宿主，必须检测 `window.openai.selectFiles`/`uploadFile`，并提供路径输入或 CLI 回退。

## 5. 成功标准

Alpha 阶段至少完成：三平台构建；三平台读写 fixture EXE；PNG/JPEG/ICO 图标转换且不隐式裁切；使用独立于 `editpe` 的 Windows API 或检查器验证 VERSIONINFO 和主图标；真实 Authenticode fixture 在修改前具有完整摘要并通过自定义测试链验证，显式授权修改后由独立 Windows API 报告重建输出不再含签名，且测试不修改系统信任存储；默认拒绝已签名输入；机器可读的 `plan` 摘要；干净消费环境完成 CLI 流程，fresh agent 无需阅读项目源码即可完成 Skill 流程；准确候选 commit 的托管 native、文档和供应链检查全部通过；候选产物包含 checksum、SBOM、许可证证据、可用的 provenance/attestation 和受保护的 `v*` tag 治理。

如果某种格式需要大型原生运行时、存在不清晰的再分发权利或渲染不确定，应停止扩展格式矩阵，保留明确支持列表并给出转换建议。

## 6. 用户可见的兼容性契约

首个稳定 schema 版本为 `1`。配置键、CLI JSON、MCP 工具名称或错误码的破坏性变更必须升级到 schema `2` 或主版本。

仓库处于 Draft/Alpha 期间，schema `1` 仍是候选契约，首个稳定版本前可以修正破坏性错误；此类变更必须写入 changelog，并同步更新生成的 schema 和双语文档。项目离开 1.0 之前成熟度后，执行稳定版本规则。

工具必须保留所有未被配置明确要求移除的资源；当输出到独立路径时必须保留原文件，并使用临时文件与原子 rename 流程完成替换。
