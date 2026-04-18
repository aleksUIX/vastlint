# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest minor (0.x) | ✅ |
| Older minor releases | ⚠️ Best effort |

Security fixes are released as patch versions and back-ported to the current
minor series. Older minor releases receive fixes on a best-effort basis.

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report security issues privately via
[GitHub Security Advisories](https://github.com/aleksUIX/vastlint/security/advisories/new).

You will receive a response within **48 hours** acknowledging your report.
We aim to release a fix within **7 days** for critical issues and **30 days**
for others. We will credit reporters in the release notes unless you prefer to
remain anonymous.

## Supply-Chain Security

vastlint targets **SLSA Build Level 2** for all release artifacts.

### Verifying release artifacts

Every binary, library, and `.vsix` attached to a
[GitHub Release](https://github.com/aleksUIX/vastlint/releases) has a signed
SLSA provenance attestation stored in GitHub's attestation store. To verify
any artifact:

```sh
gh attestation verify <file> --repo aleksUIX/vastlint
```

### Verifying the npm package

The `vastlint` npm package is published with provenance. Verify the published
package is linked to its CI run:

```sh
npm audit signatures vastlint
```

### Build integrity

- Release builds are performed exclusively by GitHub Actions from tagged source.
- The release workflow requires all smoke tests to pass before any artifact is
  published or uploaded.
- All release artifacts are built from a clean checkout of the tagged commit —
  no local developer machines are involved in producing release binaries.

## Security Standards

We are working toward alignment with the following:

| Standard | Status |
|----------|--------|
| [SLSA Build L2](https://slsa.dev) | ✅ Implemented |
| [SLSA Build L3](https://slsa.dev) | 🔄 In progress |
| [OpenSSF Best Practices (Passing)](https://www.bestpractices.dev) | 🔄 In progress |
| [OpenSSF Scorecard](https://scorecard.dev) | 🔄 In progress |
