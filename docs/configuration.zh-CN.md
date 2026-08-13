# 配置文件与 CLI 契约

## 1. 配置格式

使用 TOML 供人类编辑，使用 JSON Schema 供机器校验。相对路径以配置文件所在目录为基准，而不是以进程当前目录为基准。允许绝对路径，但不建议提交到项目仓库。

```toml
schema_version = 1
input = "dist/MyApp.exe"
output = "dist/MyApp-versioned.exe"

[policy]
allow_signed_input = false
overwrite_output = false
backup_input = false
preserve_unknown_strings = true
replace_all_icon_groups = false

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
source = "assets/logo.pdf"
pdf_page = 1
fit = "contain"
background = "transparent"
target_sizes = [16, 24, 32, 48, 64, 128, 256]
```

首版支持 `en-US`/`1200`、PNG、JPEG、ICO、SVG 和 PDF。未知语言、未知格式和不符合约束的尺寸必须报错，不能静默回退。

## 2. 安全规则

输入和输出不能相同，除非同时指定 `--in-place --confirm-in-place`。已签名输入必须同时指定允许标志和“确认签名失效”标志。默认保留未知 VERSIONINFO 字段及无关资源，默认不裁剪图标。

## 3. 命令

`init` 生成模板；`inspect` 只读检查；`plan` 只解析并展示将要变化；`apply` 执行写入；`verify` 写后校验；`convert-icon` 只转换图标源文件。

所有 JSON 输出都带 `schema_version`、输入/输出 SHA-256、变更字段、签名状态、警告和稳定错误码。
