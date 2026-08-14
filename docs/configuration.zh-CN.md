# 配置文件与 CLI 契约

## 1. 配置格式

使用 TOML 供人类编辑，使用 JSON Schema 供机器校验。相对路径以配置文件所在目录为基准，而不是以进程当前目录为基准。允许绝对路径，但不建议提交到项目仓库。

```toml
schema_version = 1
input = "dist/MyApp.exe"
output = "dist/MyApp-versioned.exe"

[policy]
overwrite_output = false
preserve_unknown_strings = true

[version]
file_version = "2.2.0.0"
product_version = "2.2.0.0"
language = "en-US"
code_page = 1200

[version.strings]
FileDescription = "My application"
ProductName = "My application"
OriginalFilename = "MyApp.exe"
CompanyName = "Example Company"
LegalCopyright = "Copyright © 2026 Example Company"

[icon]
source = "assets/logo.png"
fit = "contain"
background = "transparent"
target_sizes = [16, 24, 32, 48, 64, 128, 256]
```

未请求的 `[version]` 或 `[icon]` 段必须完整省略。`pevi init` 生成的模板会默认注释这两个修改段，因此仅创建模板不会修改 VERSIONINFO 或图标。

首个发布候选支持 `en-US`/`1200` 和 PNG、JPEG、ICO。SVG 与 PDF 在后续 feature gate 完成前作为不支持格式失败。未知语言、未知格式和不符合约束的尺寸必须报错，不能静默回退。`[version.strings]` 不得以任何大小写形式定义 `FileVersion` 或 `ProductVersion`；这两个规范值只来自类型化版本字段。

## 2. 安全规则

输入和输出不能相同，除非同时指定 `--in-place --confirm-in-place`。已签名输入必须同时指定允许标志和“确认签名失效”标志。`apply` 在既没有 VERSIONINFO 也没有图标修改时会拒绝执行。默认保留未知 VERSIONINFO 字段及无关资源；更新主图标时会先移除旧主图标及其不再共享的资源，默认不裁剪图标。

## 3. 命令

`init` 生成模板；`inspect` 只读检查；`plan` 只解析并展示将要变化；`apply` 执行写入；`verify` 写后校验；`convert-icon` 只转换图标源文件。

所有 JSON 输出都使用 `data` 包装实际命令结果，并带 `schema_version`、输入/输出 SHA-256（适用时）、签名状态、警告和稳定错误码。`plan` 是摘要，不承诺逐字段 diff。

`apply` 的 `version` 和 `icon` 在未请求对应资源时为 `null`；存在时记录已经在输出中验证通过的请求值。图标对象包含 `source_format`、`renderer`、`target_sizes` 和 `cropped`，不会包含图像源字节。

PE 解析只报告 certificate table 是否存在，不验证 Authenticode 信任或摘要。Windows 独立流程会验证修改前摘要和测试链，并在不修改系统信任存储的前提下证明重建输出不再含签名。
