# Security Policy

## Supported versions

cforge is pre-1.0 and moves fast. Only the **latest release** receives fixes —
if you are on an older tag, upgrade first (`cforge update`) and confirm the
issue still reproduces.

## Reporting a vulnerability

**Do not open a public issue for a security problem.**

Report it privately via
[GitHub's advisory form](https://github.com/ayush-kumar-21/cforge-cli/security/advisories/new).
That creates a private thread visible only to the maintainer until a fix ships.

Expect an initial response within a week. If the report is valid you'll be
credited in the advisory unless you'd rather not be.

## Scope

cforge installs toolchains and executes build commands on your machine, so the
areas most worth scrutiny are:

- **`install.sh` / `install.ps1`** — these run via `curl | sh` and fetch release
  binaries over the network. Anything that lets an attacker substitute a binary
  or escalate during install is in scope.
- **Dependency installation** — cforge shells out to package managers
  (Homebrew, apt, pacman, dnf, yum, winget, vcpkg) with constructed arguments.
  Argument or command injection reachable from project files such as `libs.txt`
  or `langs.txt` is in scope.
- **`cforge update`** — it downloads and replaces the running binary.

Out of scope: vulnerabilities in CMake, the compilers, or the package managers
themselves. Report those upstream. Also out of scope is anything requiring the
attacker to already have write access to your project directory — cforge
intentionally trusts the project it's pointed at.

## Verifying a release

Release binaries are built by
[`release.yml`](.github/workflows/release.yml) on GitHub-hosted runners and
attached to the tag. Check that the artifact you downloaded came from the
[Releases page](https://github.com/ayush-kumar-21/cforge-cli/releases) rather than a
fork or mirror.
