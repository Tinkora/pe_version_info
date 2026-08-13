# Test Fixtures

The committed PE fixtures are project-owned test data generated from the MIT
licensed sources in [`source/`](source/). They contain no proprietary code or
third-party application data.

| File | Architecture | VERSIONINFO | Certificate table | SHA-256 |
| --- | --- | --- | --- | --- |
| `pe32_unsigned.exe` | PE32 / i386 | `en-US`, UTF-16LE | absent | `bfb634324af8589a74c92c995d0283db6e268993dc8f343dc238395e8a9e9544` |
| `pe64_unsigned.exe` | PE32+ / x86-64 | `en-US`, UTF-16LE | absent | `38a82e366f43ba67f708860d41b209cd11b5e71ea2cd80a4e9a5fa9efb0edca8` |

Regenerate them with MinGW-w64 binutils and GCC:

```bash
./scripts/generate_fixtures.sh
```

The script passes `--no-insert-timestamp` and disables build IDs. A release or
fixture update must run the command twice and compare hashes before committing
new binaries. These files were generated with Homebrew MinGW-w64 GCC 16.1.0.

Malformed and certificate-table cases are derived from these files inside tests
so that each byte-level mutation remains explicit and reviewable.
