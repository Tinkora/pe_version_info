# Contributing

Keep `pe_version_info_core` platform-independent and keep CLI handlers thin. Add transports only when a concrete use case and tests require them.

1. Work in a focused branch or worktree.
2. Write a failing outcome-focused test before behavior changes.
3. Implement the smallest complete slice and run its checks.
4. Run the locked workspace checks before committing.
5. Keep generated `target/`, temporary PE files, secrets, and private inputs out of commits.

Preserve stable schema fields and error codes or document intentional compatibility changes. Update both README languages when commands, maturity, privacy boundaries, or limitations change.
