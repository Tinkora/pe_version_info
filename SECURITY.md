# Security Policy

This project handles user-provided PE binaries as untrusted data. Report vulnerabilities privately through the repository's GitHub Security Advisories channel when it is enabled; do not disclose exploit details or credentials in a public issue.

The CLI is local-only and offline. It bounds PE size, icon bytes/dimensions/pixels, and VERSIONINFO strings; it rejects malformed input without mutation. Authenticode certificate-table presence is detected but not cryptographically validated. Resource edits can invalidate signatures and require explicit dual acknowledgement.

Supported scope follows the current Draft documentation. SVG/PDF/MCP/UI and Windows Explorer verification are not promised. Include the affected commit, platform, command, input class, impact, and a minimal reproduction without attaching private binaries or secrets.
