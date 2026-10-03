# eTamil in CI

Three ready-made templates live in `packaging/ci/`. Each one installs eTamil from
the release packages, checks every `.qmz` file with `etamil --check` (which
parses and type-checks and never runs the program), and then runs your entry
point. Nothing needs Rust or a C toolchain.

| System | Template | You also copy |
|---|---|---|
| GitHub Actions | `github-actions.yml` | nothing; it uses `Maruff/setup-etamil@v1` |
| GitLab CI | `gitlab-ci.yml` | `install-etamil.sh`, `check-qmz.sh` into `ci/` |
| Anything with a shell | `generic.sh` | `install-etamil.sh`, `check-qmz.sh` into `ci/` |

Edit the two paths in each template: `src` is where your `.qmz` files are, and
`src/main.qmz` is the program to run.

## Pinning a version

`latest` follows the newest release. For a build that must not change under you,
pin one, such as `1.4.2`: `version: 1.4.2` in GitHub Actions, or
`ETAMIL_VERSION: "1.4.2"` in GitLab. Every download is checked against the
SHA-256 published beside the release archive, and the install stops if it does
not match.

## What the scripts need

`install-etamil.sh` needs `sh`, `curl`, `tar` and `sha256sum` (or `shasum`). It
installs to `$HOME/.local` (override with `PREFIX`) and prints the two variables
to export: `PATH`, and `ETAMIL_PATH`, which lets `இறக்கு "nUlakam/..."` resolve
from any directory. It supports Linux and macOS on x64 and arm64; Windows runners
use the GitHub Action, which handles the zip package.

`check-qmz.sh [folder ...]` exits non-zero if any file has an error, and also if
it finds no `.qmz` file at all, so a typo in the path cannot pass silently.

## Mirrors and offline builds

Set `ETAMIL_RELEASE_BASE` to a folder (or URL) that holds the release archive and
its `.sha256` under their GitHub names, and the scripts download from there.

## Tests

`etamil --check` finds errors in the front end. It does not run your tests, and
the CLI has no test-runner flag, so run your own test programs as an ordinary
step and have them fail with a non-zero exit code.
