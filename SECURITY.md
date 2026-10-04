# Security policy

Security fixes target the latest released version of Cussy. Earlier 0.x releases
may require an upgrade; there is no long-term support branch yet.

## Reporting a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/cussylang/cussylang/security/advisories/new)
when it is enabled. Include the affected version, operating system, a minimal
reproduction, expected impact, and any proposed fix. Do not include credentials
or private user data.

If private reporting is unavailable, open an issue asking a maintainer for a
private reporting channel, without exploit details or sensitive attachments.
Please avoid public disclosure of an unpatched vulnerability until maintainers
have had an opportunity to investigate. This volunteer project does not promise
a fixed response time or a bounty.

## Execution boundary

Cussy is a programming language, not a sandbox. Programs can access files and
environment variables through standard-library functions. Run only code and
`.csyb` artifacts you trust, or isolate execution using your operating system.

Ordinary pointers are checked by the interpreter. The optional Unix native FFI
requires `--allow-ffi` and can call native code outside those checks. Fuel limits
bound interpreter steps; they do not provide a complete resource or security
boundary. See [architecture and limits](docs/architecture.md).
