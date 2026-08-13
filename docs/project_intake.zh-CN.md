# PE Version Info 项目准入说明

[English](project_intake.md)

状态：批准进入窄范围 Draft 实现

决策日期：2026-08-13

复核日期：2026-11-13

## 可复现工作流

目标用户是收到编译后 Windows `.exe` 或 `.dll` 的发布工程师或 coding
agent。每次发布时，用户需要检查 `VERSIONINFO`，更新用户明确提供的产品字段
和主图标，写入独立输出文件，并在签名前验证结果。

输入是本地 PE 文件、带版本的配置文件和可选本地图标；输出是新的 PE 文件与
机器可读报告。失败的修改可能发布错误的 Explorer 元数据、破坏无关资源，或在
发布工程师不知情时使 Authenticode 签名失效。

该工作流在每次 Windows 发布时发生，并且经常从非 Windows CI 主机执行。正常
工作流不需要网络、托管服务、账户或上传二进制文件。

## 替代方案

| 替代方案 | 优势 | 对本工作流的缺口 |
| --- | --- | --- |
| [`rcedit`](https://github.com/electron/rcedit) | 成熟的常用版本字段和图标 CLI | 官方文档面向 Windows 二进制并直接修改目标文件；缺少本项目的配置、plan、事务性输出和签名确认契约。 |
| [Resource Hacker](https://www.angusj.com/resourcehacker/) | 成熟的 Windows GUI/CLI 资源编辑器 | 聚焦 Windows，难以作为带版本、确定性、机器可读的 CI 契约。 |
| [`winresource`](https://github.com/BenjaminRi/winresource) | 适合 Rust build script 的资源集成 | 在 Rust Windows 构建期间工作并依赖 `rc.exe` 或 MinGW；不是任意既有 PE 的通用构建后编辑器。 |
| [`editpe`](https://github.com/Systemcluster/editpe) | 跨平台解析并重建既有 PE 资源的库 | 它是实现基础，不是完整产品工作流；调用方仍需负责校验、签名策略、原子输出、稳定报告和自动化体验。 |

## 决策

继续作为独立仓库实现，因为该工具具有独立 CLI 发布、公共配置和报告 schema、
PE/签名信任边界、fixture 矩阵与平台发布节奏。它不适合浏览器工具箱，因为可执行
文件应留在本地文件系统中，修改过程需要事务性 OS 文件操作。

差异不在于重新发明 PE parser，而是使用成熟的 `editpe` 引擎建立单一、可审计的
工作流：安全输出默认值、明确签名后果、稳定错误、确定性图标转换，以及可供人和
Agent 使用的精简接口。

## 首发范围

首个发布候选必须包含：

- 在 macOS、Linux、Windows 检查 PE32/PE32+ EXE/DLL；
- 合并 `en-US` / UTF-16LE `VERSIONINFO` 并保留未知字符串；
- 有界解码 PNG、JPEG、ICO 主图标，默认不裁切；
- 默认独立输出、原子替换、写后解析和输入/输出 hash；
- 默认拒绝已签名输入，覆盖时要求两部分显式确认；
- 原生 `pevi` CLI、版本化 JSON 契约和 Codex Skill 草案。

首个发布候选明确不包含：

- MCP transport 或 MCP Apps UI；
- 远程文件上传、遥测、分析或自动更新检查；
- Authenticode 密码学信任校验或签名；
- packed executable 和任意资源编辑；
- 在所有目标的 renderer 上限、许可证和可复现 runtime 分发得到证明前，默认启用
  SVG 或 PDF。

## 信任与资源边界

- 所有 PE、配置和图标都作为不可信输入处理；
- 没有显式 in-place 和确认参数时，绝不覆盖输入；
- 绝不把 certificate table 描述成有效或可信签名；
- 畸形资源和解码上限错误必须显式失败，不得 panic；
- 在高成本分配或修改前限制 PE 字节、图标字节、尺寸、像素数、目标帧数和
  VERSIONINFO 字符串长度；
- 正常 CLI 操作保持离线，普通诊断不泄露无关本地路径。

## 验证与成熟度

在准确推送 commit 的托管 Rust、文档和供应链检查通过，并且干净消费环境能运行
文档中的 inspect、plan、apply、verify 流程前，仓库保持 **Draft**。Alpha 还要求
fresh-agent Skill 验收、独立 Windows 资源检查、真实 Authenticode 修改前后证据和
完整候选发布证据。全部满足后才可称为 **Alpha** 和 **Human-usable**。

Codex Skill 只是说明层。没有经过测试的 MCP transport 和 tool registration 时，
不得声称 **Agent-callable** 或 **Dual-use**。

首个候选成功条件：

- 成功、无效输入、边界、畸形输入、签名与写入失败均有行为测试；
- 提交的 fixture 记录来源、许可证、架构、签名状态和 SHA-256；
- macOS、Linux、Windows 托管任务通过所有公开声明的原生行为；
- 干净 checkout 无需未提交或机器本地状态即可生成相同公共 schema 并完成 CLI
  工作流；
- 后续如获授权发布产物，必须包含 checksum、SBOM、许可证证据和 provenance。

## 停止条件

在 2026-11-13 复核采用情况。如果 90 天内没有重复使用、Release 下载、可执行反馈，
也没有替代方案无法满足的已记录工作流，则合并、收窄或归档项目。不得仅为制造活跃
而扩展格式、语言、MCP 或 UI。
