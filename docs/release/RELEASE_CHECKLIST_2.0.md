# Releasing eTamil 2.0.0: the checklist

Nothing here has been done. This is the order to do it in, and what must be true before each
step. "Enabled" in "once all the platforms are enabled" means **a row of the install table in
`RELEASE_NOTES_2.0.0.md` has been run on a clean machine and worked**. Until a row has, take it out
of the notes, the README and the website rather than release a promise.

The prepared pieces, each on its own branch and each written for the release state:

| Piece | Branch | Merge when |
|---|---|---|
| Changelog, release notes, this checklist, README | `release/2.0-prep` (this repository) | last, after everything it describes |
| Website | `release/2.0-site` (`Maruff/eTamil.in`) | with the release, after the version bump |
| Wikipedia article | `docs/wikipedia-2.0`, stacked on `docs/wikipedia-1.4.2` (PR #48) | after #48 merges and the release is published |

## 1. Get the work in

- [ ] **Start CI on every pushed branch** (Actions → CI → Run workflow, or a draft PR). None of the
      new workflows has run. Read each result and fix what fails: Emacs, the Jupyter kernel, the AWS
      lint, the generated-file checks, and **Dependency audit** (a separate workflow).
- [ ] **Merge in the order in `pr-descriptions/README.md`**: `feature/lsp` first; then the
      packaging branches (`docker-image`, `rpm-package`, `apt-repository`, `homebrew-tap-update`
      are stacked in that order); the rest are independent. Keep every job when a later one
      conflicts in `release.yml` (see that README).
- [ ] **Merge the other repositories:** `Maruff/eTamil_vsCode` (`language-server`, then
      `publish-workflow`), `Maruff/etamil-tmlanguage`, `Maruff/eTamil.in` (`ide-vocabulary`, then
      `ide-projects`). The tap and `setup-etamil` are already on `main`.
- [ ] **Land the A1 Retail Banking work.** It is the unpushed commits on the local
      `docs/wikipedia-1.4.2` branch (XML and e-Sign primitives, the `vawki`, `miZkYyoppam`,
      `kataZqakaval` and `nErativari` libraries, their samples and about 200 lines of `CHANGELOG.md`).
      The notes and README already describe it. Its changelog entries belong under the `2.0.0`
      heading: when `CHANGELOG.md` conflicts with this branch's, keep both. It is not on `main` yet,
      and PR #48 carries only the Wikipedia text.
- [ ] **Decide the audit policy for RUSTSEC-2023-0071** (the `rsa` crate, mitigated by blinding, no
      fixed release). The banking branch deliberately does not ignore it, so the new
      `Dependency audit` workflow will fail once it merges. Either accept it in both
      `.cargo/audit.toml` files with that mitigation as the reason, or leave the audit failing and say
      so. The release notes already disclose it.
- [ ] **Have a Tamil reader review the Tamil glosses** in the `uqavi/` samples: they are
      machine-written.
- [ ] **Drop anything not merged** from `CHANGELOG.md`, `RELEASE_NOTES_2.0.0.md`, the README and the
      website. Each entry is there because its branch is expected to merge.

## 2. Accounts, secrets and one-time setup

All listed with their reasons in `pr-descriptions/OWNER-CHECKLIST.md`. Before a tag:

- [ ] Secrets on `eTamil_lang`: `NPM_TOKEN`, `NUGET_API_KEY`, `APT_GPG_PRIVATE_KEY`,
      `HOMEBREW_TAP_TOKEN`. A tag **fails** without the apt key, by design.
- [ ] Secrets on `eTamil_vsCode`: `VSCE_PAT` and `OVSX_PAT`; the Open VSX namespace `etamil` and its
      publisher agreement exist.
- [ ] The GHCR package will need making public once, after the first image is pushed.

## 3. Bump the version

The version is set by hand in these places (the workflows derive the rest: npm takes it from the
tag, NuGet reads `etamil_compiler/Cargo.toml`):

- [ ] `etamil_compiler/Cargo.toml` (`version`); `Cargo.lock` follows the next time it builds
- [ ] `android/rust/Cargo.toml`, its `Cargo.lock`, and `android/app/build.gradle.kts`
      (`versionName`, and raise `versionCode`, currently 7)
- [ ] `CITATION.cff` (`version` and the release date)
- [ ] `README.md`, the last line (`**Version**: ...`)
- [ ] `eTamil_vsCode`: `package.json` and `package-lock.json`, and its `CHANGELOG.md`. The publish
      workflow refuses a release whose tag differs from this.
- [ ] `CHANGELOG.md`: change `## 2.0.0 — unreleased` to `## 2.0.0 — <date>`
- [ ] The website's `brand.version` in `_config.yml`. `scripts/check_site_counts.py --check`
      compares it with the compiler's version, so it fails until both agree.
- [ ] After the banking branch lands, the builtin and library-function counts change (its changelog
      says so; the figures are not written here because the count guard would flag them on `main`):
      run `python scripts/check_site_counts.py --fix` and read what it rewrote.
- [ ] Re-run `python scripts/generate_editor_support.py --check` and
      `python scripts/check_site_counts.py --check --site <site checkout on main>`.

## 4. Rehearse without a tag

- [ ] **Actions → Release packages → Run workflow** on `main`, by hand, with no tag. It builds the
      Linux, macOS and Windows archives, the Docker image, the `.deb`, the `.rpm`, the NuGet and npm
      packages and the apt index, and publishes none of them. This is the first real run of all of
      those, so expect to fix things.
- [ ] Download its artifacts and **run the install table** from `RELEASE_NOTES_2.0.0.md`, row by row,
      on clean machines (a fresh VM or container is enough for Linux): each archive, `brew`, `apt`
      against the signed index, the `.rpm` in a Fedora container, the Docker image on amd64 and arm64,
      `npx`, `dotnet tool`, and the GitHub Action. A macOS and a Windows machine are needed for their
      rows. Record what failed.
- [ ] **Smoke-test an editor from each family:** VS Code (install the built `.vsix`), a JetBrains IDE
      (install the plugin zip, open a `.qmz`, run it), Visual Studio, and Emacs. Highlighting, the
      language server, and run.
- [ ] Open a notebook with the Jupyter kernel and press the stop button once.
- [ ] If the AWS template is claimed: deploy it to a scratch account once, then delete the stack.

## 5. Publish

- [ ] Merge `release/2.0-prep` (changelog, notes, README) and the site branch.
- [ ] Tag `v2.0.0`. The release workflow publishes every package and attaches the apt index,
      checksums and the keyring.
- [ ] Publish the extension: Actions → Publish (in `eTamil_vsCode`) with `release_tag` = `v2.0.0` and
      `publish` on.
- [ ] Make the GHCR package public; check `docker pull ghcr.io/maruff/etamil:2.0.0` without logging in.
- [ ] Draft and publish the GitHub release with `RELEASE_NOTES_2.0.0.md` as its body, after
      deleting the **DRAFT** banner and re-checking "Known limitations".
- [ ] Move the `setup-etamil` major tag only if the Action changed; list the Action on the
      Marketplace if not yet.

## 6. After

- [ ] Re-run every row of the install table against the **published** artifacts, not the rehearsal's.
- [ ] Wikipedia: merge #48, then apply `docs/wikipedia-2.0`, putting the release date in the two
      places marked `RELEASE_DATE_TODO` (`grep -rn RELEASE_DATE_TODO docs/wikipedia`). Then submit to
      the article: see the notes in that branch about sources.
- [ ] Update the status rows in the README and on the website from 🟡 to ✅ for what the rehearsal and
      the published artifacts verified, and only for that.
- [ ] Update `WBS_Tracker_v3.xlsx`.
- [ ] Announce.

## If it goes wrong

- A tag that built nothing usable: delete the GitHub release and the tag, fix, re-tag as `v2.0.1`
  rather than re-using `v2.0.0`; **a version published to npm, NuGet, a Marketplace or Open VSX
  cannot be re-used**, so a bad one is superseded, not replaced.
- A bad apt index: delete the `Packages`, `Release`, `InRelease` and `Release.gpg` assets from the
  release; apt then reports no update rather than a broken one.
- `brew` and the tap: revert the tap's `main` to the previous commit.
