# Changelog

## 1.4.0

The extension moves to 1.4.0 with the language. See the repository's CHANGELOG
for nuNNaRivu, kOppumuRY and the rest of 1.4.0.

### Added

- Completions and highlighting for the 14 new builtins, the
  `nUlakam/nuNNaRivu` modules and `nUlakam/kOppumuRY.qmz`: 96 builtins and 806
  library functions.

### Changed

- The carried compiler is 1.4.0.

## 1.3.0

The extension moves to 1.3.0 with the language. See the repository's CHANGELOG
for artino, eTamil compiled to Arduino boards, and the rest of 1.3.0.

### Added

- **eTamil: Build for a board (artino)** and **Build and upload to a board**:
  pick an Uno, Nano, Mega, Pico or Pico 2 and, to upload, a serial port. They
  need an `etamil` built with `--features llvm` set in `etamil.compilerPath`;
  the carried compiler has no LLVM.
- Completions and highlighting for the 20 new builtins and the
  `nUlakam/vaZporuL` board files: 82 builtins and 722 library functions.

### Changed

- The carried compiler is 1.3.0.

## 1.2.0

The extension moves to 1.2.0 with the language. The language itself is
unchanged, and so is what the extension highlights and completes. See the
repository's CHANGELOG for the Raspberry Pi package and the rest of 1.2.0.

### Changed

- Hovers for standard-library functions show what each function is for, where
  271 of them used to repeat the signature, and 153 functions that had no
  English documentation have it now.
- The carried compiler is 1.2.0.

## 1.1.0

The language gains four things, and the extension knows all of them: `நிலை`
bindings that do not change, functions as values, record shapes (`வடிவம்`) and
their methods. See the repository's CHANGELOG for what each one does.

### Added

- Highlighting for `நிலை` and `வடிவம்` in every spelling (`nilY`, `_const`,
  `vativam`, `_shape`).
- Completions and signature help for `nUlakam/aNi.qmz`'s map, filter and fold:
  `ஒவ்வொன்றுக்கும்`, `வடிகட்டு` and `மடி`.
- The carried compiler is 1.1.0, which also runs files saved with CRLF line
  endings.
- Three new example programs under `examples/language/`, for **eTamil: Open an
  example**.

## 1.0.3

A second face, which draws Tamil where the first drew none, and reads a
consonant cluster the way Tamil is written. The language is unchanged: the
compiler accepts exactly what it accepted before.

### Added

- **`ican qamiz Smart` ships beside the plain face.** It carries the same 95
  printable ASCII characters, adds 72 characters of the Tamil block, and applies
  three contextual rules in OpenType: a consonant that no vowel follows takes
  the pulli, a vowel that no consonant precedes is drawn at full size upon the
  baseline in its own advance, and the inherent `a` after a consonant draws
  nothing. `vaNakkam` sets as வணக்கம் while the eight bytes on disk are
  unchanged.

  **These are the font's conventions and not the language's.** The compiler does
  not accept them in keywords, function names or variables; it requires the
  canonical form, in which every vowel is written, because that is what keeps
  the encoding reversible. They are for setting Tamil in a document.

  Every encoded glyph keeps the outline the plain face gives it. The alternative
  forms live in unencoded glyphs that only the substitutions reach, so a
  renderer that does not apply `calt` shows exactly the plain rendering, and the
  two faces are interchangeable for anyone who wants the old behaviour.

- **`src/bundle.ts` now holds a `FACES` table** rather than a single font name,
  and `fontinstall.ts` installs every face in it. **eTamil: Install the eTamil
  font** copies both, registers both on Windows, and offers to set
  `etamil.eTamilFont` to `ican qamiz Smart`.

- **`etamil.eTamilFontFeatures`** turns the contextual rules on or off. The
  features are injected into the decoration CSS for eTamil spans alone, so
  nothing outside those spans and no global editor setting is touched — in
  particular `editor.fontLigatures` is left alone.

### Fixed

- **The Tamil coverage of the Smart face was reported as 87 characters.** It is
  72. The higher figure came from counting the code points a cmap format 4
  segment spans without resolving each to a glyph id; fifteen of them resolve to
  glyph 0. 201 mapped code points minus 72 Tamil leaves the 129 the plain face
  maps, which is the cross-check. `fonts/README.md` lists the fifteen.

## 1.0.2

The carried font, which said two contradictory things about its own licence.
Nothing in the language, the compiler or the extension's behaviour changes.

### Fixed

- **`ican qamiz` claimed all rights reserved while declaring the OFL.** The
  font's `name[0]` read `Copyright (c) 2026, eTamil.in. All rights reserved.`
  while `name[13]` and `name[14]` named the SIL Open Font License 1.1, and the
  `OFL.txt` shipped beside it read `Copyright (c) 2026, eTamil.in.` with no
  reservation at all. A font offered under the OFL cannot also reserve every
  right. `name[0]` now matches `OFL.txt` exactly.

  The font was not regenerated from `ican_qmz.sfd` to do it. That source holds
  201 glyphs, some seventy of them encoded at Tamil codepoints, against the 132
  and the empty Tamil block this build ships — the divergence `fonts/README.md`
  already warns about. Rebuilding would have produced a different font. Only
  the `name` table was rewritten: the other sixteen tables are byte-identical,
  all eighteen table checksums verify, and `head.checksumAdjustment` was
  recomputed.

- **The codepoint count in `fonts/README.md` was wrong.** It said 143, which is
  the union of all three cmap subtable key sets. The Mac subtable is keyed by
  MacRoman bytes rather than Unicode, so those keys are not codepoints and do
  not add to the total. Both Unicode subtables map the same 129.

### Changed

- **The font is now `fonts/ican_qamiz-Regular-2.1.1.ttf`.** Version 2.1.0 is
  kept beside it under its old name so that the binary 1.0.1 shipped is still
  in the tree; the two differ in the `name` table and nowhere else. The name
  carries the version because two files that differ only in metadata are
  unreadable otherwise.

  `src/bundle.ts` names the file that ships, and `test/bundle.test.js` now
  reads that name instead of repeating it, so a future rename cannot leave the
  test asserting against a file the extension no longer installs.

  Anyone who ran **eTamil: Install the eTamil font** under 1.0.1 has the 2.1.0
  file in their per-user font directory already. Running it again installs
  2.1.1 beside it rather than replacing it, because the filename changed and
  both declare the family `ican qamiz`. Deleting `ican_qamiz-Regular.ttf` from
  that directory leaves the corrected one.

## 1.0.1

The listing, which 1.0.0 shipped without. Nothing in the language, the compiler
or the extension's behaviour changes; 1.0.0 is a working package and stays
installable.

### Fixed

- **`.qmz` files had no icon.** The extension's own icon was always correct, but
  `contributes.languages` carried no `icon` of its own, so every eTamil file
  drew the generic blank sheet — in the explorer, in tabs, in Open Editors and
  in Quick Open. The same mark now serves both themes: it has its own dark
  ground rather than being a white glyph on transparency, which would disappear
  into a light theme.

  This shows where no file icon theme is active, or where the active theme does
  not claim `.qmz`. Seti, the VS Code default, claims unknown extensions and
  keeps drawing its own; winning there needs an icon theme contribution, which
  this is not.

### Added

- **The listing says what eTamil is.** It opened with one sentence and a feature
  table, so a reader learned what the extension does and nothing about what the
  language is for: money that is exact rather than binary floating point, a
  standard library that is the domain rather than a framework found later, and
  three spellings for every keyword.

- A `galleryBanner` in the project's navy, `#002140`, instead of the default
  grey.

## 1.0.0

The first release numbered with the language: eTamil 1.0, and the extension that
carries it. The Marketplace has been serving 0.2.0, which could highlight eTamil
and nothing else — everything below had to be installed by hand first, or was
not possible at all. Installing this one is the whole setup.

### Added

- **The eTamil font travels with the extension.** `ican qamiz`, under the SIL
  Open Font License, is the face in which the ASCII letters carry Tamil glyphs:
  `c` draws ச, `q` draws த, `Z` draws ன. It is in `fonts/`, with `OFL.txt`
  beside it and `fonts/README.md` recording what the file declares about
  itself.

  **eTamil: Install the eTamil font** puts it where the operating system can
  see it — `~/Library/Fonts`, `~/.local/share/fonts` followed by `fc-cache`, or
  `%LOCALAPPDATA%\Microsoft\Windows\Fonts` plus the `HKCU` value Windows needs
  to list a font for one user — and then offers to set `etamil.eTamilFont`. Per
  user throughout, so no administrator rights, and behind a dialog that names
  the files and the registry value before writing either.

  There is an install command because shipping a font is not installing one.
  VS Code's editor is not a webview, no API registers a font with it, and both
  `editor.fontFamily` and a decoration's `font-family` resolve against the
  fonts the operating system knows. A font inside an extension is a file. VS
  Code has to be restarted afterwards: the font list is read when the window
  starts.

  Setting `etamil.eTamilFont` to a font that is not installed used to be
  indistinguishable from the setting doing nothing — no error anywhere, the
  editor simply drawing in its own font. For the font the extension carries
  that is now noticed, and the offer to install it is made.

- The font is the reason to state plainly something the rules doc had wrong:
  **`ican qamiz` has no Tamil glyphs at all.** It maps 129 codepoints, every
  one of them ASCII, and nothing in U+0B80–U+0BFF. The extension paints the
  eTamil face over eTamil-script ASCII and leaves everything else in the
  editor's own font, so the base font is the one carrying every `மொத்தம்` in
  the file, and this works. Painted the other way — the editor set to the
  eTamil font, English painted back to ISO — every Tamil letter in every file
  would have fallen back to an arbitrary face. A design decision that was made
  for a different reason turns out to have been the only workable one.

  One caveat, in the file rather than in the code: `ican qamiz` is
  proportional. A painted run does not sit on the monospace grid, so text after
  it on the same line shifts. A monospaced build of the face would make it
  exact with no change here.

- **eTamil: Open an example.** The extension carries the repository's
  twenty-nine example programs beside the standard library, and this opens a
  copy of one — a copy, because the extension directory is replaced on every
  update and an example is for editing. The accounting framework, the HTTP
  server, the GST invoice and the project-costing worked example are all in
  there.

- **eTamil: Documentation…** opens the manual, the browser playground, the
  language tour, the keyword reference, the finance and server guides, the
  status page or the source, on [etamil.in](https://etamil.in). Every keyword
  hover now links the reference for that word as well.

  This is a gap that had been open since the extension was written. It could
  complete 681 standard library functions and never once say that a manual
  existed, which is a good part of why people do not find one. `src/links.ts`
  is the single table all of it reads from, and `test/links.test.js` holds the
  README to the same table so the two cannot drift apart.

### Fixed

- **Counts in the documentation that had drifted a long way.** The extension's
  own README offered "23 host builtins and all 122 `செயல்` functions"; it is 62
  and 681. The repository README said the VS Code extension completed 254
  `nUlakam` functions and that the lexer had 524 spellings; 681 and 541.
  `docs/ARCHITECTURE.md` said 201 keywords; 202. Every figure now comes from
  the generated language data, which is derived from `lexer.rs`, `parser.rs`,
  `interpreter.rs` and `nUlakam` itself.

  Not touched: `docs/ROADMAP.md` recording "524 spellings, up from 505". That
  is the account of one completed change, and moving it to 541 would credit it
  with seventeen spellings it did not add.

  **The website is further behind and is not in this repository.**
  `etamil.in/language/manual/` states 254 `nUlakam` functions and
  `etamil.in/language/keywords/` says "202 tokens across 505 spellings"; both
  are 681 and 541 now. Worth a pass before this release goes out.

### Changed

- The carried font is the corrected build: **version 2.1.0**, designer Esan
  Maruff, manufacturer eTamil India, `eTamil.in`. The earlier file credited Ek
  Type's designers in its name table while its copyright named only eTamil.in,
  which is the mismatch OFL 1.1 asks a derivative to avoid; that is settled.

  Its glyph coverage is unchanged, and worth stating plainly because it was
  expected to change: **there is still no Tamil in the font.** All three cmap
  subtables were read — (0,3) and (3,1) format 4, (1,0) format 0 — and between
  them they map 143 codepoints, every one ASCII or Latin-1, with nothing in
  U+0B80–U+0BFF.

  The glyphs exist in the design. `ican_qmz.sfd` holds 201 glyphs of which some
  seventy are encoded at Tamil codepoints; the exported `.ttf` has 132 and maps
  none of them. It is an export that needs redoing, not a design that needs
  drawing, and nothing in this repository can fix it. It is also not blocking,
  for the reason in `fonts/README.md`: the extension paints the eTamil face
  over eTamil-script ASCII only, so Unicode Tamil is drawn by the editor's own
  font and always was.


- **The extension carries the compiler.** The `etamil` binary for your platform
  and the whole nUlakam standard library now travel inside the VSIX, so error
  checking, **eTamil: Run this file** and Go to Definition into the library all
  work the moment the extension finishes installing. No Rust, no release
  download, no `PATH`.

  One VSIX per platform, built with `vsce package --target`, so a Windows user
  never downloads a macOS binary. `scripts/package_extension.py` stages the
  files and drives it.

  The order of resolution is `etamil.compilerPath`, then the carried binary,
  then `etamil` on the `PATH`. An explicit setting always wins, so a developer
  working on the compiler runs their own build; a platform this VSIX was not
  built for falls back to the `PATH` and to the download and source-build
  routes, which are unchanged.

  `ETAMIL_PATH` is set to the carried library whenever you have not set one
  yourself, which is what makes `இறக்கு "nUlakam/paNam.qmz"` resolve. A
  terminal opened by **eTamil: Run this file** gets it too.

- **eTamil: Install the compiler for use outside the editor** copies the
  carried binary and library to `~/.local`, or `%LOCALAPPDATA%\Programs\eTamil`
  on Windows, and shows the two lines that put them on `PATH` and
  `ETAMIL_PATH`. Nothing is downloaded and nothing is compiled, so it is the
  one install route that cannot fail for a reason outside the machine. It
  shows the lines rather than writing them: how every program on a machine
  starts is not something an extension should change on its own.

- **`etamil.eTamilFont`** renders a file in two fonts at once. eTamil is
  written three ways and two of them are the same bytes — `செயல்` is Tamil,
  `ceyal` is the same word under the ezuqqu scheme, `_length` is English — and
  there is an eTamil font in which the ASCII letters carry Tamil glyphs, under
  which an English word is nonsense: `sum` draws as ஸும்.

  Set this to that font's family name and the ASCII that is eTamil is drawn in
  it, while a name marked `_english`, a comment wrapped in `__ … __`, every
  string literal and the licence header stay in your ordinary editor font. The
  marks are the language's own, specified in `docs/reference/SCRIPT_RULES.md`
  and gated by `scripts/check_script_rules.py`.

  The editor's font is not changed — it stays the ISO stack, and only the
  eTamil-script ASCII is painted over the top. That direction is deliberate. A
  span the scanner fails to recognise then renders eTamil as plain Latin,
  which is the ordinary view of the file; painted the other way round, the same
  miss would render English in Tamil glyphs, which is unreadable.

- The grammar scopes both marks — `meta.english.comment.etamil` for the text
  between `__` and `__`, and `variable.other.english.etamil` for a name marked
  with a leading underscore — so a theme can colour them whether or not the
  eTamil font is in use.

### Changed

- Go to Definition falls back to the carried standard library, so it works for
  a reader who has the extension and no checkout of the repository.

- The command line **eTamil: Run this file** types into the terminal is now
  quoted for the shell that terminal runs. The carried binary lives under the
  user's home directory, which may contain a space, and PowerShell needs the
  call operator in front of a quoted path where `cmd` must not have it.

## 0.4.0

### Added

- **eTamil: Install the compiler** now offers the prebuilt package first. The
  old dialog offered only `cargo build --release`, which asks for Rust and a C
  toolchain before the author has run a single line of eTamil. The package needs
  neither.

  It opens the release URL in a browser and, if asked, copies the two
  extract-and-install commands to the clipboard. It deliberately does not fetch
  the archive or pipe anything remote into a shell: the author downloads it,
  sees what they have, and runs the installer themselves.

  On a platform with no prebuilt package — macOS today — the same entry opens
  the releases page and points at the source build.

### Changed

- The generated keyword data now carries **523 spellings across 202 tokens**, up
  from 505. Eighteen keywords gained their on-scheme romanization — `utal`,
  `paqil`, `vazi` and the rest — while keeping the spelling they had, so both
  highlight and complete. Nothing that lexed before stops lexing.

- Completions show a `செயல்`'s declared signature where it has one, because
  parameters and return types can now be declared:
  `செயல் வரி(எண் தொகை) எண் { … }`.

## 0.3.0

The keyword data is now generated from the compiler instead of maintained by
hand, and the extension gained the features that needed it.

### Fixed — the extension described a language two revisions old

- **67 of the lexer's 201 keywords were missing from the grammar**, including
  `செயல்` and `திரும்பு` (functions), `ஒவ்வொரு`/`இல்` (iteration), `இறக்கு`
  (modules), `மற்றும்`/`அல்லது`/`இல்லை` (logical operators), `ஈர்ம` (the
  boolean type), and every database type and table operation. Because the
  grammar ended in a catch-all Tamil rule, all 67 rendered as variable names —
  so `nUlakam/kaNakkiyal/`, the accounting framework, opened with its control
  flow uncoloured.
- **28 romanized spellings were the pre-`Z` scheme.** `eZil` got no
  highlighting; `enil`, which the compiler rejects, got highlighted as a
  keyword. A third set of spellings (`paNgu`, `vazhi`) belonged to neither
  scheme.
- `பொது` was offered as the boolean type. It is not a keyword at all — the
  completion inserted a parse error. The real type, `ஈர்ம`, appeared nowhere.
- The `சுற்று` snippet and completion emitted a C-style three-clause `for`
  loop. eTamil writes `(cond) சுற்று { }`.
- `/* */` block comments were declared in the grammar, the language
  configuration and a snippet. The lexer has no block comment, so commenting a
  region greyed it out and then failed to lex.
- `'...'` was treated as a string. eTamil has no single-quoted string, and a
  stray apostrophe started a phantom string that swallowed the rest of the file.
- The file snippets opened without a mode, which defaults to `read`. Because the
  mode is not enforced, the write then succeeded as an *append* — so the CSV
  template added a fresh header row on every run.
- `மாறி` and `நிலை` were highlighted as declarations. The parser treats both as
  ordinary names, and neither has any statement syntax.
- Financial nouns were scoped as keywords, though the compiler deliberately
  allows them as names. They now use a `support.type.domain` scope, and hover
  says they are free to reuse.
- Percentage literals never highlighted: the pattern ended in `\b` after `%`,
  which cannot match before `;` or end of line.
- `1_000` was highlighted as a number. The lexer rejects digit separators.
- `indentationRules` looked for `எனில்` followed by `(`, but eTamil puts the
  condition first, so they never fired.
- The declared language icon pointed at an `icons/` directory that does not
  exist.
- `license` said MIT. The repository is AGPL-3.0-or-later, deliberately.
- `.eslintrc.json` had no TypeScript parser, so `eslint src --ext .ts` failed on
  the first type annotation. It had never linted this code.

### Fixed — security and behaviour

- `etamil.installCommand` was a resource-scoped string executed in a terminal,
  so a cloned repository's own `.vscode/settings.json` could choose the command.
  Removed. The install command is now fixed in code, or typed by the author in
  the moment.
- `capabilities.untrustedWorkspaces` is declared unsupported. The extension runs
  a compiler binary, so it must not be active in an untrusted workspace.
- Activation is `onLanguage:etamil` only. `onStartupFinished` meant the
  extension woke in every window and immediately offered to run
  `git clone && cargo build`.
- `etamil.autoInstallOnActivation`, `etamil.syntaxHighlight` and
  `etamil.showIntelliSense` were declared and never read. Removed the first two;
  `etamil.intelliSense` now actually gates the providers.
- "Skip" wrote a `workspaceState` flag that was never read, so the prompt came
  back on the next startup. "Don't ask again" now means it.
- The 60-second verification poll was not cancellable. Removed.
- Completions no longer offer bare operators (`+`, `-`, `*`) as items, and are
  no longer rebuilt on every keystroke.

### Added

- **Errors as you type**, from `etamil --check` — a new compiler mode that stops
  after the type checker, so an editor never runs the program it is checking.
  Positions convert from the compiler's code-point columns to UTF-16 exactly.
- **Completions** for all 23 host builtins and all 122 `செயல்` functions in
  `nUlakam`, with parameter names and doc comments read from their source.
- **Spelling-aware templates**: a romanized prefix inserts romanized
  placeholders.
- **Request-variable completions** inside a `வழி` handler — `request_body`,
  `query_params`, `headers`, `path_params` and the rest.
- **Signature help**, **Go to Definition** into `nUlakam`, **document outline**
  of every `செயல்`, and **eTamil: Run this file** / **Serve this file**.
- A Tamil-first font stack for eTamil files, because most monospace faces carry
  no Tamil glyphs and fall back mid-line.
- `test/grammar.test.js` — scope assertions via vscode-textmate, including that
  `\b` behaves correctly around Tamil combining marks, and a no-holes sweep over
  the standard library.
- `test/snippets.test.js` — every snippet body is compiled by the real compiler.

## 0.2.0 and earlier

Syntax highlighting, snippets and a keyword completion list, maintained by hand.
