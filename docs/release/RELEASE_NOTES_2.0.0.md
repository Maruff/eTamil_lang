# eTamil 2.0.0

> **DRAFT.** Written before the release, from the state of the repositories on 2026-10-04.
> The release checklist (`RELEASE_CHECKLIST_2.0.md`) says what must be true before this is
> published, and the "Known limitations" section must be re-checked against it. Every line in
> "How to get it" is a promise to the reader: run each command on a clean machine first.

eTamil is a programming language whose vocabulary is Tamil, aimed at Indian accounting,
tax and FinTech. **2.0 is the release that puts it where you already work**: install it
from your package manager, edit it in your editor, run it in a notebook, serve it from a
container or from AWS, and read a specification of what the language is.

The language and the standard library carry on from 1.4.2, with **libraries for retail banking**,
XML and the e-Sign primitives, the project-office libraries and a clock builtin added since. **We know of no breaking change**; 2.0 marks the reach of the
release, not an incompatibility. If a program that ran on 1.4.2 does not run on 2.0, that is a
bug: please report it.

## How to get it

| Where | Command |
|---|---|
| **Windows, Linux, macOS** (any) | Download the archive for your platform from this page and run its `install.ps1` or `install.sh` (see the README). |
| **Homebrew** (macOS, Linux) | `brew install Maruff/etamil/etamil` |
| **Debian, Ubuntu** (apt) | Add the repository and its key, then `sudo apt install etamil` (steps in `packaging/README.md`). `apt upgrade` finds later releases. |
| **Fedora, RHEL, openSUSE** | `sudo dnf install ./etamil-2.0.0-1.x86_64.rpm` (or `aarch64`), from the files on this page. |
| **Docker** | `docker run --rm -v "$PWD":/work ghcr.io/maruff/etamil:2.0.0 --vm hello.qmz` |
| **npm** | `npx etamil hello.qmz`, or `npm install -g etamil` |
| **.NET** | `dotnet tool install -g etamil` (and `etamil-lsp` for the language server) |
| **GitHub Actions** | `uses: Maruff/setup-etamil@v1` |
| **VS Code, Cursor, Windsurf, VSCodium** | Search for "eTamil" in the extensions view. The extension carries the compiler: nothing else to install. |

Every package is the same static compiler (musl on Linux, so it does not depend on your
glibc), the standard library `nUlakam/` and the examples, with `ETAMIL_PATH` set so
`இறக்கு "nUlakam/..."` resolves from any directory. Each archive has a `.sha256` beside it.

## What is new

### Everywhere you work

- **A language server, `etamil-lsp`**, in every package: the compiler's own diagnostics in
  Tamil and English, completion of keywords (each with a statement template), builtins and
  standard-library functions (which add their `இறக்கு` line), hover documentation, and go to
  definition. It speaks LSP, so it works in any editor that does.
- **JetBrains IDEs:** a plugin with highlighting, the language server (through LSP4IJ) and a
  **run configuration**: Run | Run 'file.qmz' (Ctrl+Shift+F10) runs the file, checks it without
  running it, or serves it as an HTTP server.
- **Visual Studio:** extensions for the language and the language server.
- **Emacs:** `etamil-mode`: highlighting in both scripts, indentation, `C-c C-c` to run and
  `C-c C-k` to check, and Eglot set up for the server.
- **Neovim, Helix, Zed:** the tree-sitter grammar gains locals, tags and injections queries and
  C, Node and Rust bindings.
- **VS Code and its forks:** the extension is built for all five platforms from a release, and
  published to the Visual Studio Marketplace and to Open VSX, which is where Cursor, Windsurf and
  VSCodium look.
- **Jupyter:** an eTamil kernel. A cell runs in the compiler's own shell, so variables and
  functions persist between cells and a cell stops at its first error.

### The language, written down

- **A specification.** `docs/reference/LANGUAGE_SPEC.md` covers the lexical structure, the
  grammar and the operator precedence, and `docs/reference/etamil.ebnf` is the grammar. They are
  generated from the tree-sitter grammar, which takes its keywords from the lexer and parses every
  program in `nUlakam/` and `examples/`; CI fails if they drift. The grammar is shown to *accept*
  real programs. It is not shown to *reject* exactly what the compiler rejects: `etamil --check`
  is the judge of what is valid.

### Retail banking, in eTamil

The A1 Retail Banking work adds libraries for the arithmetic and the bookkeeping of a retail bank's
loans and deposits, written in eTamil like the rest of the standard library. None of a bank's rates,
thresholds or card figures is written into any of them: they arrive as parameters, and where a
convention changes the money, it is stated at the top of the file.

- **Signing a loan with Aadhaar e-Sign.** `miZkYyoppam` builds a signed e-Sign request, reads the
  answer and checks that it is genuine. A retail loan's signing is a record that moves from ready to
  sent to signed, failed or expired: the agreement and the Key Fact Statement are hashed from their
  files and signed in one request, a document changed after hashing cannot be sent, and an answer is
  refused unless a verifier you supply proves it genuine.
- **Loans:** repayment allocation (what is overdue, the penal charge, where a payment goes in the
  order a policy lists); day-count conventions (ACT/365, ACT/360, ACT/ACT, 30/360, 30E/360, with no
  default); the Key Fact Statement, with the annual rate solved from the monthly cash flows and a
  text that is byte-for-byte the same from the same facts, so it can be hashed; fee amortisation by
  the effective interest method, including on floating-rate loans; **floating-rate loans**, with
  their resets and the whole schedule; modifying a loan, with or without a change in its cash
  flows, and derecognising one that changes substantially; partial prepayment, including between
  instalments; forgiving part of a loan in a restructuring; writing off a whole loan and the
  recoveries after it; and selling the claim on a loan.
- **Deposits:** savings, current, BSBDA, salary, minor, joint and NRI products; term and recurring
  deposits, including what comes back when one is broken early; the life of a deposit account from
  open to dormant to closed; and tax deducted at source on deposit interest, with the per-customer
  threshold, senior citizens, and 15G and 15H declarations.
- **Mandates:** UPI AutoPay and NACH as one shape, with a check that says why a debit may or may not
  be presented today.
- **Customer and credit:** KYC status and consent by purpose, and the lending side of a credit
  bureau (the checks on a pull, single, waterfall and parallel strategies, and reuse of a stored
  report), with the bureau's own connector left to the caller.
- **The books:** accrual and provision events, and a GL mapping that turns each banking event into a
  balanced journal entry, refusing one that does not balance and saying by how much.
- **In the compiler:** XML in the language (`எக்ஸ்எம்எல்_படி`, `எக்ஸ்எம்எல்_ஆக்கு`, and a canonical form
  that an XML signature covers; a DOCTYPE is refused), and the primitives an e-Sign integration is
  built from: SHA-256 of text and of a file, hex to base64, and RSA-SHA256 signing and verification
  over keys of 2048 bits or more.
- **A worked example for every public function** of these libraries, run by `scripts/run_samples.sh`.

**What is not verified.** Nothing has run against an e-Sign provider's sandbox or a bureau. The
e-Sign attribute names are the 2.1 names as understood and need checking against the provider's
specification, and the response verifier has been tested only against signatures the library made
itself. Several readings of the tax rules are flagged in the files for a tax team to confirm. The
Tamil glosses beside the English in the samples are machine-written and should be read by a Tamil
reviewer.

### Deploying

- **AWS:** a CloudFormation template runs an eTamil HTTP program on ECS Fargate behind a load
  balancer: health checks, rolling deploys that roll back on failure, optional HTTPS, optional
  secrets from Secrets Manager. See `deploy/aws/`.
- **Docker** images for `linux/amd64` and `linux/arm64`, running as a non-root user.

### Libraries and the compiler, since 1.4.2

- **A project office, in eTamil.** `nUlakam/qittam` gains task roll-up, the critical path on a
  working calendar, RAG health, change control, scope and the portfolio; `nUlakam/oruwkiNYppu` is
  an Azure DevOps integration; `nUlakam/qayArippu` holds product versions and customer
  commitments; and `examples/aluvalakam/` runs a SQLite-backed service on them.
- **`இப்போதைய_நொடி()`**, whole seconds since 1970, for anything stored and read back after a
  restart. The XML and e-Sign primitives below add nine more builtins.
- **Fixes:** a baselined task can be finished; base64 wrapped with Windows line endings decodes;
  the VS Code extension offers ARM Linux the arm64 package.

### Security

- **One known advisory, mitigated.** The `rsa` crate, added for e-Sign, carries
  [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071) (the Marvin attack: timing
  differences in the private-key operation can leak the key) and **there is no fixed release**. It is
  mitigated, not fixed: signing uses a random source, which turns on blinding, and the signature is
  byte-identical to the unblinded one. So **1.4.2's "no known vulnerabilities" does not hold for
  2.0**. RSA is native only, not in the browser build.
- **A response from an e-Sign provider is not believed until it is proven genuine.** The response
  address can be reached by anyone who knows the transaction, so a forged "signed" answer would
  otherwise sign a loan. The library requires a verifier, with no default.
- **A credit report is never returned for the wrong applicant;** a report with no owner fails closed.
- **Dependencies are now audited in CI,** on every lockfile change and weekly. On the code as it stood
  before the banking libraries the first audit found no vulnerability, and led to updating `anyhow` in
  the compiler and two yanked crates in the Android library. Advisories accepted for now, with
  reasons, are in `.cargo/audit.toml`: `rustls-pemfile` (unmaintained; replacing it is a code change)
  and two `lru` advisories reached only through the optional MySQL driver. The `rsa` advisory above is
  the one the audit reports and the release must decide about (see the release checklist).

## Known limitations

State these as of the release; **re-check each one before publishing**, and delete any that is no
longer true.

- **The banking libraries are unverified against any provider or bureau** (see "What is not verified" under Retail banking), and their Tamil glosses await a Tamil reviewer.
- **No debugger.** The compiler has none, so no editor offers breakpoints.
- **The macOS binaries are not notarized.** Gatekeeper refuses the first run; the README says how
  to clear it once.
- **Editors:** the JetBrains plugin and the Visual Studio extensions are installed from files, not
  from their marketplaces. Highlighting and the language server have been tested headlessly, not in
  every IDE.
- **The apt repository holds only the newest release,** and the rpm files are not in a
  repository: install a newer one over the old.
- **The browser editor** (etamil.in) cannot import the standard library, because the wasm build has
  no module loader. Files in a project do not import each other there.
- **The Jupyter kernel** reads one line at a time: a statement split over several lines outside
  braces is not supported, and output appears when an input finishes.
- **The LLVM backend** is not in the packaged builds, and still refuses I/O statements.

## Checksums

Each archive is published with its SHA-256 beside it. The apt index is signed; its public key is
`etamil-archive-keyring.gpg` on this page.

## Thank you

Everyone who ran an early build, reported what broke, or wrote a program in Tamil and told us what
the language got wrong.
