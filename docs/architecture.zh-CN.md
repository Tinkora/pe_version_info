# 架构与技术方案

## 1. 组件关系

```text
ChatGPT/Codex 宿主
   └─ Draft Skill：指导 Agent
             │
             └── pevi CLI
                    │
            pe_version_info_core
             ├─ editpe：已有 PE 资源重建
             └─ image：PNG/JPEG/ICO
                    │
                 EXE/DLL
```

## 2. 当前工作区结构

```text
pe_version_info/
├── crates/pe_version_info_core/
├── crates/pevi_cli/
├── plugin/pe-version-info/
├── fixtures/
├── schemas/
└── docs/
```

插件只是分发元数据和 Agent 指令，Rust crate 才是行为唯一事实来源。
MCP Server 与 UI 是已记录的后续工作，当前目录和 plugin manifest 中都不存在。

## 3. 资源修改流程

先读取并识别 PE32/PE32+，然后检查证书表并执行签名策略；之后解析资源，合并 VERSIONINFO 字段，统一语言/代码页，转换图标，替换主图标组，事务性重建资源，写入临时文件，重新解析验证，最后输出报告。无关资源必须保留。

首版语言设置为 `en-US` / UTF-16LE，即 `0x0409` / `0x04B0`，字符串表键为 `040904B0`。

## 4. `editpe` 与 `winresource` 的分工

`winresource` 是 Rust 构建脚本辅助库，适合在自己编译 Windows 程序时嵌入资源，需要 Windows SDK 或 MinGW 工具链。它不是修改任意已有 EXE 的编辑器。

`editpe` 明确支持跨平台解析和修改已有 PE 资源，包括图标和版本信息，因此作为 Core 的主引擎。依赖必须固定版本并通过 fixture 测试。

## 5. 图标转换

首个发布候选使用 `image` 支持 PNG/JPEG/ICO。默认保持宽高比、不裁剪，并使用透明留白；只有用户显式选择 `cover` 时才裁剪。ICO 输入保留有效的多分辨率帧。

SVG 与 PDF 属于后续能力：SVG 计划使用 `resvg`/`usvg`，PDF 计划使用 PDFium 渲染指定页（默认第 1 页）。两者必须先证明资源上限、许可证和所有目标平台的可复现 runtime 分发。

PDFium 属于原生运行时，启用 PDF 前必须记录各平台二进制来源、许可证和校验值。

## 6. 签名与写入

已签名输入默认拒绝，因为修改资源会使 Authenticode 摘要失效。写入目标使用同目录临时文件，关闭并验证后再原子替换；默认不覆盖输入，`--in-place` 必须显式确认。
