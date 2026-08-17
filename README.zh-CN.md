# Tinkora PE Version Info

> **状态：Alpha CLI prerelease（`v0.1.0-alpha.4`）**——native CLI 已通过三平台、干净消费环境、Windows 资源和 Authenticode 证据；Codex Skill/plugin 仍是 Draft，不是 Agent-callable MCP 发布物。

[English](README.md)

<!-- markdownlint-disable MD033 -->
<p align="center">
  <a href="https://ko-fi.com/tinkora" target="_blank" rel="noopener noreferrer">
    <img
      src="https://ko-fi.com/img/githubbutton_sm.svg"
      alt="在 Ko-fi 上支持 Tinkora"
      width="520"
    >
  </a>
</p>
<!-- markdownlint-enable MD033 -->

PE Version Info（`pevi`）在 native 主机上检查并安全更新 Windows PE32/PE32+ EXE/DLL 的 `VERSIONINFO` 和图标。Core 使用 `editpe`，CLI 为自动化输出一个稳定 JSON 对象。

当前候选范围：

- Rust native core 和 `pevi` CLI：`inspect`、`plan`、`apply`、`verify`、`init`、`convert-icon`。
- `en-US`/UTF-16LE（`040904B0`）VERSIONINFO、未知字符串保留，以及 PNG/JPEG/ICO 图标输入。
- 默认独立输出、事务性写入、有界解码；原地编辑或存在证书表时必须提供两组显式授权。
- Draft Codex Skill/plugin 编排；不宣称 Agent-callable MCP 支持。

SVG、PDF、MCP/UI 和人工 Explorer UI 复核属于后续范围。Alpha 仍要求独立 Windows API 或检查器提供资源与 Authenticode 证据。不要绕过“修改后验证、再签名”的流程处理已签名正式文件。

## 快速开始

```bash
cargo build --locked --release -p pevi_cli
target/release/pevi --help
target/release/pevi inspect --input fixtures/pe32_unsigned.exe --format json
```

执行修改前请阅读[配置](docs/configuration.zh-CN.md)、[安全与兼容性](docs/security_and_compatibility.zh-CN.md)和[发布流程](docs/RELEASING.zh-CN.md)。

## 安装发布二进制

[最新 prerelease](https://github.com/Tinkora/pe_version_info/releases) 提供
Linux x86-64、macOS Apple Silicon 和 Windows x86-64 归档。下载归档及对应的
`.sha256` 文件，校验后再把 `pevi` 放入 `PATH`：

```bash
gh release download v0.1.0-alpha.4 \
  --repo Tinkora/pe_version_info \
  --pattern 'pevi-v0.1.0-alpha.4-*' \
  --dir release-assets
cd release-assets
sha256sum --check --strict ./*.sha256
tar -xzf pevi-v0.1.0-alpha.4-x86_64-unknown-linux-gnu.tar.gz
install -m 0755 pevi "$HOME/.local/bin/pevi"
pevi --version
```

Windows 用户请使用 `Get-FileHash` 校验 `.exe.sha256`，再把可执行文件放到用户
拥有且已加入 `PATH` 的目录。发布资产还包含 `SHA256SUMS`、SPDX SBOM、许可证证据
和 GitHub attestations；完整命令见[发布流程](docs/RELEASING.zh-CN.md)。

## 开发检查

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
```

另请阅读 [贡献指南](CONTRIBUTING.zh-CN.md)、[安全政策](SECURITY.zh-CN.md)、
[支持说明](SUPPORT.zh-CN.md) 和 [变更日志](CHANGELOG.md)。
