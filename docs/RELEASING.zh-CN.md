# PE Version Info 发布流程

发布工作流只接受已经通过 `main` 必需检查、且受保护的 `v` 前缀 SemVer tag。
它会在三个支持的平台构建 CLI，生成并校验发布证据，然后在 `release` 环境门禁后
发布 prerelease GitHub Release。本地成功不能替代远端证据。

## 候选流程

1. 使用锁定的 Rust 1.95.0 工具链构建已审核 commit。
2. 用 `pevi inspect` 检查目标 PE。
3. 用 `pevi plan` 审查稳定 JSON 契约。
4. 默认输出到新文件，执行 `pevi apply` 后再执行 `pevi verify`。
5. 资源修改和验证完成后才进行 Authenticode 签名。
6. 执行独立签名检查并记录输入/输出 SHA-256。
7. 为准确的 workspace 版本创建受保护 tag，例如 `v0.1.0-alpha.3`，
   等待 tag 触发的 `Release` workflow。
8. 下载三个目标归档和聚合证据 artifact，核验 `SHA256SUMS`、
   `sbom.spdx.json`、`license_inventory.json` 和
   `THIRD_PARTY_NOTICES.md`。SBOM 和许可证清单包含三个发布 target 上从
   `pevi_cli` 可达的 normal/build 依赖并集，不包含仅用于开发的依赖。
   notice 文件会嵌入每个入选第三方 Cargo package 随包发布的许可证与通知
   文件；如果许可证声明无法与包内文件明确对应，证据生成会直接失败。
9. 使用 GitHub CLI 对每个候选二进制分别验证 build provenance 和 SBOM
   attestation。
10. workflow 只有在仓库规则和受保护的 `release` 环境满足后才会发布 prerelease。

对每个下载的二进制验证两种 predicate，并把 signer 限定为本仓库的发布
workflow：

```bash
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml \
  --source-ref refs/tags/v0.1.0-alpha.3 \
  --source-digest COMMIT_SHA
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml \
  --predicate-type https://spdx.dev/Document/v2.3 \
  --source-ref refs/tags/v0.1.0-alpha.3 \
  --source-digest COMMIT_SHA
```

将 `COMMIT_SHA` 替换为候选版本使用的准确审核 commit；两次校验必须使用
相同的 source ref 和 digest。

不要原地修改已签名的正式文件。保留旧产物及校验和用于回滚，也不要移动不可变 tag。
