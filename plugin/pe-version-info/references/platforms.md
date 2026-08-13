# Platforms and limits

The native core targets macOS, Linux, and Windows hosts and supports PE32 and
PE32+ EXE/DLL inputs. The committed fixture and hosted CI matrix are evidence
for each platform; a local build alone is not proof of Windows Explorer
property-dialog or icon-cache behavior.

The first candidate supports en-US / UTF-16LE (040904B0) VERSIONINFO and PNG,
JPEG, and ICO icons. SVG, PDF, MCP, and UI integration remain follow-up work.
The CLI is offline and uses bounded file, image dimension, pixel, and
VERSIONINFO string limits.

Resource edits change signed bytes. A certificate table is detected but not
cryptographically validated. Re-sign only after pevi verify and perform a
final independent signature check in the release pipeline.
