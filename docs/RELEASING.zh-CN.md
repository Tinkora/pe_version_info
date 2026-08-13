# PE Version Info 发布流程

在准确推送的 commit 通过托管 native、文档和供应链检查前，仓库保持 Draft；
本地成功不等于发布授权。发布工作流会校验带 `v` 前缀的 SemVer 是否与 workspace
版本一致，构建候选产物并生成聚合发布证据；它不会自动创建 tag 或 GitHub Release。

## 候选流程

1. 使用锁定的 Rust 1.95.0 工具链构建已审核 commit。
2. 用 `pevi inspect` 检查目标 PE。
3. 用 `pevi plan` 审查稳定 JSON 契约。
4. 默认输出到新文件，执行 `pevi apply` 后再执行 `pevi verify`。
5. 资源修改和验证完成后才进行 Authenticode 签名。
6. 执行独立签名检查并记录输入/输出 SHA-256。
7. 使用准确的 workspace 版本触发 `Release candidate build`，例如
   `v0.1.0-alpha.1`。
8. 下载三个目标二进制和聚合证据 artifact，核验 `SHA256SUMS`、
   `sbom.spdx.json`、`license_inventory.json` 和
   `THIRD_PARTY_NOTICES.md`。
9. 使用 GitHub CLI 对每个候选二进制分别验证 build provenance 和 SBOM
   attestation。
10. 只有在核验仓库规则、受保护环境和托管检查后，授权维护者才能发布预发布版本。

对每个下载的二进制验证两种 predicate，并把 signer 限定为本仓库的发布
workflow：

```bash
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml \
  --predicate-type https://spdx.dev/Document/v2.3
```

不要原地修改已签名的正式文件。保留旧产物及校验和用于回滚，也不要移动不可变 tag。
