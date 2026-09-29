# Installation

## Prebuilt packages

Each release publishes a package that needs neither Rust nor a C toolchain:

| Platform | Package |
|---|---|
| Windows x64 | `etamil-windows-x64.zip` |
| Linux x64 | `etamil-linux-x64.tar.gz` |
| Linux arm64 — Raspberry Pi 4/5 on 64-bit Raspberry Pi OS | `etamil-linux-arm64.tar.gz` |
| macOS Apple Silicon | `etamil-macos-arm64.tar.gz` |
| macOS Intel | `etamil-macos-x64.tar.gz` |

Download from
`https://github.com/Maruff/eTamil_lang/releases/latest/download/<package>`.
On Linux, `uname -m` says which one you need: `aarch64` means arm64, `x86_64`
means x64. A Raspberry Pi running the 32-bit OS reports `armv7l`; there is no
package for that, so build from source or move to the 64-bit OS.

```bash
tar -xzf etamil-linux-arm64.tar.gz
./etamil-linux-arm64/install.sh
```

Both Linux packages are static musl binaries with the PostgreSQL and MySQL
drivers built in. The [README](../../README.md#installation) covers Windows and
macOS, including the macOS quarantine step.

The rest of this page is for building from source. It takes one command once
the prerequisites are in place.

## Prerequisites

| | Requirement | Why |
|---|---|---|
| Rust | **1.85 or newer** | the crate uses edition 2024 |
| C toolchain | platform linker | Rust needs a system linker; some dependencies build C |
| LLVM 18 | *optional* | only for the `--llvm` backend, Linux/macOS only |

Install Rust from [rustup.rs](https://rustup.rs).

### Linux

```bash
sudo apt install build-essential      # Debian/Ubuntu
sudo dnf groupinstall "Development Tools"   # Fedora
```

### macOS

```bash
xcode-select --install
```

### Windows

Rust's default toolchain is `x86_64-pc-windows-msvc`, which needs the Microsoft linker. Installing Rust alone is **not** enough — without `link.exe` even `cargo check` fails, because procedural-macro crates are compiled and linked as DLLs during the build.

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Open a new terminal afterwards so `PATH` picks up both tools.

## Build

```bash
git clone https://github.com/Maruff/eTamil_lang.git
cd etamil_compiler/etamil_compiler
cargo build --release
```

The binary lands at `target/release/etamil` (`etamil.exe` on Windows).

### What that build does not include

`cargo build --release` builds the default features — `sqlite` and
`http-client` — and nothing else. **The released packages are not that build.**
`release.yml` sets `FEATURES: postgres,mysql`, so every downloaded binary, and
the one running on the droplet, carries both database drivers.

A program that opens PostgreSQL therefore works against a released binary and
fails against a plain local one. To build what actually ships:

```bash
cargo build --release --features postgres,mysql
```

LLVM stays out of the packages on purpose: it needs LLVM installed on whatever
machine runs the compiler. Add `--features llvm` yourself when you want it.

## Install the binary

### Linux / macOS

```bash
cd ..
./install.sh
```

`install.sh` copies the binary to `~/.local/bin/etamil`, or to `/usr/local/bin/etamil` when run with `sudo`. It builds first if the binary is missing.

### Windows

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\bin"
Copy-Item "target\release\etamil.exe" "$env:USERPROFILE\bin\etamil.exe"
```

Add `%USERPROFILE%\bin` to your user `PATH` through **Settings → System → About → Advanced system settings → Environment Variables**, then open a new terminal.

## Verify

```bash
etamil --version
etamil --help
```

Then run a real program:

```bash
echo "950000" | etamil --vm examples/basic_samples/example.qmz
```

Expected output ends with:

```
High Tax Bracket
30000

✓ Execution completed successfully
```

## Optional: the LLVM backend

Off by default. It requires **LLVM 18 specifically** — `llvm-sys` is pinned to `"180"`, so a newer LLVM will not link — and it is not available on Windows.

```bash
cargo build --release --features llvm
```

The LLVM backend currently covers a smaller subset than the VM. Verify the
build and IR path with a minimal arithmetic program:

```bash
printf 'எண் x = 2 + 3;\nஅச்சு x;\n' >/tmp/etamil_llvm_smoke.qmz
target/release/etamil --llvm /tmp/etamil_llvm_smoke.qmz
```

This writes `output.ll`. Programs using unsupported features are refused with
an explicit diagnostic; use `--vm` for the full language.

## Troubleshooting

**`etamil: command not found`** — the install directory is not on `PATH`.

```bash
export PATH="$PATH:$HOME/.local/bin"
echo 'export PATH="$PATH:$HOME/.local/bin"' >> ~/.bashrc
```

**`linker 'link.exe' not found` (Windows)** — Visual Studio Build Tools with the C++ workload is missing. See the Windows prerequisites above.

**`edition2024 is required`** — Rust is older than 1.85. Run `rustup update`.

**Build fails after changing dependencies**

```bash
cargo clean && cargo build --release
```

## Uninstall

```bash
rm ~/.local/bin/etamil          # user install
sudo rm /usr/local/bin/etamil   # system install
```

---

Next: [Quick Start](QUICKSTART.md) · [Command Reference](../reference/COMMANDS.md)
