# Packaging

How the downloadable eTamil packages are built and published. Read this before
cutting a release — a couple of the decisions here are load-bearing for links
published elsewhere.

## What a package is

One archive per platform, holding everything eTamil needs to run:

| | |
|---|---|
| `etamil` / `etamil.exe` | the compiler, one binary |
| `nUlakam/` | the standard library and the accounting framework, written in eTamil |
| `examples/` | every example from the repository |
| `install.ps1` / `install.sh` | the installer |
| `README.txt` | version, licence, what the installer does |

There is nothing to install alongside it. On Windows the C runtime is linked
statically, so the Visual C++ Redistributable is not needed; on Linux the binary
is built against musl, so it is one static ELF that does not care what glibc the
build machine had. That is the whole point of the package: **an eTamil user
should not need Rust, MSVC or LLVM to run eTamil.**

Neither installer needs administrator rights. Windows installs to
`%LOCALAPPDATA%\Programs\eTamil`, Linux to `$PREFIX` (`~/.local` by default).
Both put `etamil` on `PATH` and set `ETAMIL_PATH`, which is what lets
`இறக்கு "nUlakam/paNam/paNam.qmz"` resolve from any directory. Uninstalling is
deleting the directory and undoing those two variables.

## Building

```bash
./packaging/build.sh
```

Writes `dist/etamil-<os>-x64.{zip,tar.gz}` plus a `.sha256`. `dist/` is
gitignored.

Build each platform on that platform, or use `.github/workflows/release.yml`.
On Linux, use musl so the binary is not tied to the build machine's glibc:

```bash
rustup target add x86_64-unknown-linux-musl
sudo apt install musl-tools                 # for the linker
TARGET=x86_64-unknown-linux-musl ./packaging/build.sh
```

The Linux archive is a single-click installer for Ubuntu, Debian, Fedora, and
other x86_64 Linux distributions. Debian and Ubuntu users can also take the
`.deb` and Fedora, RHEL and openSUSE users the `.rpm` (both below); the archive
itself works on any other distribution.
The macOS and Windows archives are built on native CI runners because their
SDKs and linkers are not available on Linux. Those files are created only when
the release workflow runs for a version tag; they are not present in a normal
checkout until published as release assets.

Then check the archive from a clean extraction, not from the build tree — the
build tree has `nUlakam/` sitting next to it and will mask a packaging mistake:

```bash
cd /tmp && tar -xzf .../dist/etamil-linux-x64.tar.gz && cd etamil-linux-x64
./etamil --version
printf 'இறக்கு "nUlakam/paNam/paNam.qmz";\nஅச்சு ரூபாய்(12345678.50);\n' > /tmp/t.qmz
./etamil --vm /tmp/t.qmz          # expect ₹1,23,45,678.50
```

On Windows, `ldd` has no equivalent; `dumpbin /dependents etamil.exe` should
list only OS DLLs — `KERNEL32`, `ADVAPI32`, `bcrypt`, `ntdll`, `ws2_32` and
friends. If `VCRUNTIME140.dll` appears, the static CRT flag did not take effect
and the package will fail on a clean machine.

## The names are deliberately unversioned

`etamil-windows-x64.zip`, not `etamil-0.3.0-windows-x64.zip` — and the directory
inside the archive matches. This is what makes GitHub's *latest* redirect
resolve:

```
https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-windows-x64.zip
```

That URL is written into four places that are expensive to keep in step:
`README.md`, the website (`_config.yml` in the site repository, used by
`install.md` and `ta/install.md`), and the VS Code extension
(`DOWNLOADS` in `eTamil_Code/src/extension.ts`). Versioned asset names would
mean editing all four every release, and a stale link is worse than no link.

The version is not lost: it is in `README.txt` inside the archive, in the release
tag, and in `etamil --version`. **If you rename the assets, those four places
have to change with them.**

## The Debian package

`packaging/build-deb.sh` repackages a Linux archive as a `.deb`; it compiles
nothing, so the arm64 package builds on an x86-64 machine. The package depends on
no others, because the binary is static musl, and it finds the library in
`/usr/share/etamil` by itself.

```bash
bash packaging/build-deb.sh dist/etamil-linux-x64.tar.gz
sudo apt install ./dist/etamil_<version>_amd64.deb
```

The release workflow's `deb` job builds both architectures, installs the amd64
one and runs it, then attaches them to the release. They are not in an apt
repository, so `apt update` will not upgrade them; install a newer `.deb` over
the old one.

## The RPM package

`packaging/build-rpm.sh` repackages a Linux archive as an `.rpm`, the same way: it
compiles nothing, so the aarch64 package builds on an x86-64 machine, and it
depends on no other package. It needs `rpmbuild` (`apt install rpm` on Debian and
Ubuntu, `dnf install rpm-build` on Fedora).

```bash
bash packaging/build-rpm.sh dist/etamil-linux-x64.tar.gz
sudo dnf install ./dist/etamil-<version>-1.x86_64.rpm
```

A pre-release version such as `1.5.0-rc1` is written `1.5.0~rc1`, because RPM does
not allow `-` in a version. The release workflow's `rpm` job builds both
architectures, installs the x86_64 one in a Fedora container and runs it, then
attaches them to the release. As with the `.deb`, there is no repository, so a
newer package is installed over the old one.

## The Docker image

`packaging/Dockerfile` unpacks the Linux package on Alpine; nothing is compiled
in it, so the image holds exactly the binary published as the Linux archives. It
runs as a non-root user with `/work` as the working directory and
`ETAMIL_PATH` pointing at the bundled library.

```bash
bash packaging/build.sh        # needs dist/etamil-linux-<arch>.tar.gz
docker build -f packaging/Dockerfile -t etamil .
docker run --rm -v "$PWD:/work" etamil program.qmz
```

A local build is single-architecture, because `build.sh` builds only the current
machine. The release workflow's `docker` job builds amd64 and arm64 from both
archives and pushes `ghcr.io/<owner>/etamil` with the version and `latest` tags
on a `v*` tag; a manual run builds and smoke-tests without pushing. The
registry package is private until it is made public once in the GitHub package
settings.

## Publishing

Attach the archives to a GitHub release. Do not commit them — a binary in git
history is permanent, every clone pays for it forever, and removing one means
rewriting history for everyone who already cloned.

With the [GitHub CLI](https://cli.github.com):

```bash
gh release create v0.3.0 dist/etamil-*-x64.zip dist/etamil-*-x64.tar.gz dist/*.sha256 \
  --title "eTamil 0.3.0" --generate-notes
```

To add the Linux archive to a release that already exists:

```bash
gh release upload v0.3.0 dist/etamil-linux-x64.tar.gz dist/etamil-linux-x64.tar.gz.sha256
```

`--generate-notes` builds the notes from the commits since the last tag, which
is a starting point rather than a release note. Prefer `--notes-file` pointing at
something written for readers.

Or in a browser: **Releases → Draft a new release**, tag `v0.3.0`, then drag the
files from `dist/` onto the attachment box.

### Checklist

1. `cargo test` passes and `./scripts/run_examples.sh` is green.
2. Version bumped in `etamil_compiler/Cargo.toml`, and in the site's
   `_config.yml` (`brand.version`).
3. `./packaging/build.sh` on Windows; the same with `TARGET=x86_64-unknown-linux-musl`
   on Linux.
4. Both archives tested from a clean extraction, on a machine without Rust if
   one is available.
5. Release created, all assets and `.sha256` files attached.
6. Download links checked — they are only as good as the asset names.
