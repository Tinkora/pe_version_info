# Tinkora PE Version Info

> **状态：Draft**——native core/CLI 和 Draft Codex Skill 已通过托管 native、文档和供应链检查；Alpha 仍需独立 Windows 资源与 Authenticode 证据、干净消费环境/Skill 验收，以及完整发布治理证据。

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

## 开发检查

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
```

另请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)、[SECURITY.md](SECURITY.md)、[SUPPORT.md](SUPPORT.md) 和 [CHANGELOG.md](CHANGELOG.md)。
