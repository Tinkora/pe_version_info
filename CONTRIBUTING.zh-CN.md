# 贡献

保持 `pe_version_info_core` 与平台无关，CLI handler 保持薄；只有在有明确用例和测试时才增加其他 transport。

1. 在聚焦 branch 或 worktree 中工作。
2. 修改行为前先写失败的结果导向测试。
3. 实现最小完整增量并运行对应检查。
4. 提交前运行锁定依赖的 workspace 检查。
5. 不要提交 `target/`、临时 PE、秘密或私有输入。

保持稳定 Schema 字段与错误码；若有兼容性变化，必须记录。命令、成熟度、隐私边界或限制变化时同步更新双语 README。
