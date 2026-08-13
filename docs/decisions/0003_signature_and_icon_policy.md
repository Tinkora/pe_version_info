# ADR 0003: Reject Signed Inputs and Do Not Crop Icons by Default

Date: 2026-08-13  
Status: Accepted for implementation planning

## Context

Editing a PE resource changes bytes covered by Authenticode. Cropping a logo to make it square can also destroy intentional whitespace and make a product icon appear optically smaller or larger.

## Decision

Reject signed inputs by default and require explicit acknowledgement to proceed. Convert icon sources with aspect-ratio-preserving `contain` fit and transparent letterboxing by default. Make cropping (`cover`) opt-in and record it in the report.

## Consequences

- The normal pipeline becomes “build → pevi apply/verify → sign”, which avoids stale signatures.
- Product logos retain their source composition unless the user deliberately requests cropping.
- Generated ICOs may contain transparent margins for non-square sources, which is an intentional and inspectable result.

