# Maturity and capability labels

The native CLI is **Alpha** as of `v0.1.0-alpha.2`: core success, invalid-input,
boundary, clean-consumer, independent Windows resource, and real Authenticode
pre/post-edit behavior has passed hosted checks on the tagged commit. The
Codex Skill/plugin remains **Draft** until a fresh-agent acceptance run is
recorded; it is not an Agent-callable MCP release.

Capability labels are independent: the repository contains a **Draft Codex Skill**
and versioned JSON schemas, but it is not **Agent-callable** because no MCP
transport is shipped. SVG/PDF and MCP/UI remain follow-up scope.

Promotion requires reviewable evidence on the exact commit, current release/security/support/changelog docs, reproducible artifacts, and protected release governance. Local test success or an unexecuted workflow cannot promote maturity.
