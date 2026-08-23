# Security Policy

## Supported versions

cforge is pre-1.0 and moves fast. Only the **latest release** receives fixes —
if you are on an older tag, upgrade first (`cforge self-update`) and confirm the
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
- **`cforge self-update`** — it downloads and replaces the running binary.

Out of scope: vulnerabilities in CMake, the compilers, or the package managers
themselves. Report those upstream. Also out of scope is anything requiring the
attacker to already have write access to your project directory — cforge
intentionally trusts the project it's pointed at.

## Verifying a release

Release binaries are built by
[`release.yml`](.github/workflows/release.yml) on GitHub-hosted runners and
attached to the tag. Each binary ships with a `<asset>.sha256` file
containing its SHA-256 digest, generated on the same runner that built it.

`install.sh`, `install.ps1`, and `cforge self-update` all verify that digest
before installing anything, and are **fail-closed**: a mismatched checksum,
or one that cannot be fetched at all, aborts the install and leaves any
existing binary untouched. Fail-open on a missing checksum would be no
protection — an attacker positioned to substitute the binary can equally
make the checksum request fail.

**Finding the current release:** cforge is pre-1.0 and its releases are
tagged as GitHub pre-releases (see above), so the usual
`releases/latest/download/...` shortcut doesn't resolve — that URL
specifically excludes pre-releases. All three installers resolve the actual
current tag first (github.com's redirect on `releases/latest`, falling back
to the API's release list if that doesn't point at one), then download from
`releases/download/<tag>/<asset>` directly. This is why it's also fail-closed:
"could not determine which release to install" refuses to guess rather than
falling back to something unverified.

To check a manual download yourself, get the current tag from the
[Releases page](https://github.com/ayush-kumar-21/cforge-cli/releases) and
substitute it below:

```sh
TAG=v0.1.0   # whatever the Releases page currently shows
curl -fsSLO https://github.com/ayush-kumar-21/cforge-cli/releases/download/$TAG/cforge-linux-x86_64
curl -fsSL  https://github.com/ayush-kumar-21/cforge-cli/releases/download/$TAG/cforge-linux-x86_64.sha256
sha256sum cforge-linux-x86_64   # must match the .sha256 contents
```

Also check that the artifact came from the Releases page itself rather than a
fork or mirror.

What this does *not* cover: the binaries are not code-signed or notarized,
so macOS Gatekeeper still treats them as unidentified, and the checksum
proves only that you received what the release workflow published — it is
not a signature over who published it.
