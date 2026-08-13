# 安全、签名与兼容性

## Authenticode 策略

资源修改会改变 Authenticode 覆盖的字节，因此工具会先检测证书表；默认拒绝存在证书表的输入，只有同时提供两个显式授权标志才继续，并报告签名已失效或不存在。证书 blob 本身不等于签名有效。应在 `pevi verify` 通过后再重新签名。

## 输入/输出安全

- 默认写入新的 sibling 文件。
- 输入输出相同、symlink 或 hard-link 别名会被识别；原地写入必须同时使用 `--in-place --confirm-in-place`。
- 使用同目录临时文件，完成解析验证后再替换目标；失败时保留输入。
- 默认不覆盖已有输出，必须显式设置 `overwrite_output = true`。

## 资源限制与兼容性

工具限制 PE 文件大小、图标源字节数、栅格尺寸/像素和 VERSIONINFO 字符串长度，并拒绝畸形或截断资源。首个候选支持 macOS、Linux、Windows 主机上的 PE32/PE32+ EXE/DLL、`en-US`/UTF-16LE（`040904B0`）以及 PNG/JPEG/ICO。SVG、PDF、MCP/UI 和 Windows Explorer 视觉验证属于后续范围。
