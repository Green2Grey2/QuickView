# Security Policy

## Reporting a vulnerability

If you believe you have found a security vulnerability, please do **not** open a public issue.

Instead, report it via [GitHub Security Advisories](https://github.com/Green2Grey2/QuickView/security/advisories/new) with:
- a description of the issue
- steps to reproduce
- affected versions

## Scope

QuickView handles untrusted input (image files) and shells out to OCR tooling.
Please be mindful of:
- image decoding attack surface
- sandboxing and least privilege
- safe handling of temporary files
