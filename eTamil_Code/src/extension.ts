// eTamil support for VS Code.
//
// Three things changed the shape of this file from what it replaced.
//
// 1. Nothing here restates the language. Keywords, spellings, builtins and the
//    standard library all come from src/generated/language-data.ts, which is
//    produced from the compiler by scripts/generate_editor_support.py. The
//    previous version kept its own list, and it drifted until a third of the
//    language was missing and the romanized spellings it offered were the ones
//    the compiler rejects.
//
// 2. Diagnostics come from the compiler's own front end, via `etamil --check`,
//    which stops after the type checker. There is no second parser here to
//    keep in step, and opening a file never runs it.
//
// 3. Activation is scoped to eTamil files, and nothing is installed without
//    being asked. The previous version activated in every window through
//    `onStartupFinished` and immediately offered to run a shell command that a
//    workspace's own settings could supply.
//
// 4. The compiler travels inside the VSIX. `src/toolchain.ts` resolves it —
//    setting, then carried, then PATH — so a fresh install checks and runs a
//    file with nothing else installed, and `eTamil: Install the compiler`
//    copies what is already here rather than sending anyone to a download
//    page. The download and build routes remain, for a platform this VSIX was
//    not built for.
//
// 5. `src/fonts.ts` draws the ASCII that is eTamil script in the eTamil font,
//    leaving the editor's ISO font everywhere the `_` and `__` marks say
//    English. docs/reference/SCRIPT_RULES.md. The font itself travels in the
//    VSIX and `src/fontinstall.ts` puts it where the operating system can see
//    it, which VS Code has no API to do for us.

import * as path from 'path';
import * as vscode from 'vscode';

import { check, toPosition } from './compiler';
import { openExample, showDocumentation } from './docs';
import { installFont } from './fontinstall';
import { registerScriptFont } from './fonts';
import {
  COVERAGE,
  completionProvider,
  definitionProvider,
  documentSymbolProvider,
  hoverProvider,
  signatureHelpProvider,
} from './language';
import { registerScriptMode } from './scriptMode';
import {
  bundledLibrary,
  carried,
  compilerPath,
  initialise,
  installCarried,
  terminalCommandLine,
} from './toolchain';

const LANGUAGE = 'etamil';
const SKIP_INSTALL_PROMPT = 'etamil.skipInstallPrompt';

/**
 * The prebuilt packages, which need neither Rust nor a C toolchain.
 *
 * The asset names carry no version on purpose: that is what lets GitHub's
 * `/releases/latest/download/` redirect resolve, so these constants stay
 * correct after every release without this file being edited. The version is
 * still recoverable from README.txt in the archive and from `etamil --version`.
 */
const RELEASES_URL = 'https://github.com/Maruff/eTamil_lang/releases/latest';

const DOWNLOADS: Record<string, { asset: string; commands: string }> = {
  win32: {
    asset: 'etamil-windows-x64.zip',
    commands: [
      'Expand-Archive etamil-windows-x64.zip -DestinationPath .',
      '.\\etamil-windows-x64\\install.ps1',
    ].join('\n'),
  },
  linux: {
    asset: 'etamil-linux-x64.tar.gz',
    commands: ['tar -xzf etamil-linux-x64.tar.gz', './etamil-linux-x64/install.sh'].join(
      '\n'
    ),
  },
  // A static aarch64 build, for a Raspberry Pi 4 or 5 on 64-bit Raspberry Pi
  // OS. Without this key an ARM machine falls back to `linux` and is offered
  // the x64 archive, which cannot run there.
  'linux-arm64': {
    asset: 'etamil-linux-arm64.tar.gz',
    commands: ['tar -xzf etamil-linux-arm64.tar.gz', './etamil-linux-arm64/install.sh'].join(
      '\n'
    ),
  },
  // macOS is two genuinely different binaries, so the key carries the
  // architecture as well. The extra xattr line is not optional: these are not
  // notarized, and Gatekeeper refuses a quarantined binary outright rather
  // than asking.
  'darwin-arm64': {
    asset: 'etamil-macos-arm64.tar.gz',
    commands: [
      'tar -xzf etamil-macos-arm64.tar.gz',
      './etamil-macos-arm64/install.sh',
      'xattr -dr com.apple.quarantine ~/.local/lib/etamil',
    ].join('\n'),
  },
  'darwin-x64': {
    asset: 'etamil-macos-x64.tar.gz',
    commands: [
      'tar -xzf etamil-macos-x64.tar.gz',
      './etamil-macos-x64/install.sh',
      'xattr -dr com.apple.quarantine ~/.local/lib/etamil',
    ].join('\n'),
  },
};

/**
 * The package for this machine.
 *
 * Keyed by platform and architecture, falling back to platform alone: Windows
 * ships one x64 build, while Linux and macOS each ship arm64 and x64 packages,
 * and picking by platform alone would offer ARM Linux and half of macOS users
 * a binary that cannot run.
 */
function downloadFor(
  platform: string,
  arch: string
): { asset: string; commands: string } | undefined {
  return DOWNLOADS[`${platform}-${arch}`] ?? DOWNLOADS[platform];
}

/** How long to wait after a keystroke before checking. */
const CHECK_DEBOUNCE_MS = 400;

let diagnostics: vscode.DiagnosticCollection;
let output: vscode.LogOutputChannel;

export function activate(context: vscode.ExtensionContext): void {
  output = vscode.window.createOutputChannel('eTamil', { log: true });
  diagnostics = vscode.languages.createDiagnosticCollection(LANGUAGE);
  context.subscriptions.push(output, diagnostics);
  initialise(context);

  output.info(
    `eTamil support ready: ${COVERAGE.keywords} keywords across ` +
      `${COVERAGE.spellings} spellings, ${COVERAGE.builtins} builtins, ` +
      `${COVERAGE.stdlib} standard library functions.`
  );
  output.info(
    carried()
      ? `compiler: ${compilerPath()} (carried in this extension)`
      : `compiler: ${compilerPath()} — nothing carried for ` +
          `${process.platform}-${process.arch}`
  );

  registerLanguageFeatures(context);
  registerScriptFont(context, output);
  registerScriptMode(context, output);
  registerDiagnostics(context);
  registerCommands(context);
}

export function deactivate(): void {
  diagnostics?.dispose();
}

// ---------------------------------------------------------------------------
// Language features
// ---------------------------------------------------------------------------

function registerLanguageFeatures(context: vscode.ExtensionContext): void {
  const settings = vscode.workspace.getConfiguration('etamil');

  // Honoured, rather than declared and ignored: the previous version shipped
  // three settings that no code read, so turning them off did nothing.
  if (settings.get<boolean>('intelliSense', true)) {
    context.subscriptions.push(
      // No triggerCharacters. VS Code triggers on word characters already, and
      // the language configuration's wordPattern now includes Tamil, which is
      // what makes filtering on a Tamil prefix work. The old provider listed
      // ~50 trigger characters and still only covered independent vowels.
      vscode.languages.registerCompletionItemProvider(LANGUAGE, completionProvider()),
      vscode.languages.registerSignatureHelpProvider(
        LANGUAGE,
        signatureHelpProvider(),
        '(',
        ','
      )
    );
  }

  context.subscriptions.push(
    vscode.languages.registerHoverProvider(LANGUAGE, hoverProvider()),
    vscode.languages.registerDocumentSymbolProvider(LANGUAGE, documentSymbolProvider()),
    vscode.languages.registerDefinitionProvider(LANGUAGE, definitionProvider())
  );
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

function registerDiagnostics(context: vscode.ExtensionContext): void {
  const pending = new Map<string, NodeJS.Timeout>();
  // Reported once per session. A missing binary is a real thing to say, but
  // saying it on every keystroke would be worse than silence.
  let warnedUnavailable = false;

  const run = async (document: vscode.TextDocument) => {
    if (document.languageId !== LANGUAGE) {
      return;
    }
    if (!vscode.workspace.getConfiguration('etamil').get<boolean>('checkOnType', true)) {
      return;
    }

    const result = await check(document.getText(), document.uri.fsPath);

    if (result.unavailable) {
      diagnostics.delete(document.uri);
      if (!warnedUnavailable) {
        warnedUnavailable = true;
        output.warn(result.unavailable);
        // "Don't ask again" has to actually not ask again. The previous
        // version wrote a workspaceState flag for exactly this and then never
        // read it, so the prompt returned on every startup.
        if (!installPromptSilenced(context)) {
          const choice = await vscode.window.showWarningMessage(
            `eTamil: ${result.unavailable}`,
            'Install the compiler',
            "Don't ask again"
          );
          if (choice === 'Install the compiler') {
            await vscode.commands.executeCommand('etamil.install');
          } else if (choice === "Don't ask again") {
            await context.workspaceState.update(SKIP_INSTALL_PROMPT, true);
          }
        }
      }
      return;
    }

    warnedUnavailable = false;
    diagnostics.set(
      document.uri,
      result.errors.map((error) => {
        const diagnostic = new vscode.Diagnostic(
          toPosition(document, error),
          error.message,
          vscode.DiagnosticSeverity.Error
        );
        diagnostic.source = 'etamil';
        return diagnostic;
      })
    );
  };

  const schedule = (document: vscode.TextDocument) => {
    if (document.languageId !== LANGUAGE) {
      return;
    }
    const key = document.uri.toString();
    const existing = pending.get(key);
    if (existing) {
      clearTimeout(existing);
    }
    pending.set(
      key,
      setTimeout(() => {
        pending.delete(key);
        void run(document);
      }, CHECK_DEBOUNCE_MS)
    );
  };

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => void run(document)),
    vscode.workspace.onDidSaveTextDocument((document) => void run(document)),
    vscode.workspace.onDidChangeTextDocument((event) => schedule(event.document)),
    vscode.workspace.onDidCloseTextDocument((document) => {
      const key = document.uri.toString();
      const existing = pending.get(key);
      if (existing) {
        clearTimeout(existing);
        pending.delete(key);
      }
      diagnostics.delete(document.uri);
    }),
    new vscode.Disposable(() => {
      for (const timer of pending.values()) {
        clearTimeout(timer);
      }
      pending.clear();
    })
  );

  // Whatever is already open when the extension wakes up.
  for (const document of vscode.workspace.textDocuments) {
    void run(document);
  }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

function registerCommands(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand('etamil.run', () => runCurrentFile('--vm')),
    vscode.commands.registerCommand('etamil.serve', () => runCurrentFile('--async')),
    vscode.commands.registerCommand('etamil.buildForBoard', () => buildForBoard(context, false)),
    vscode.commands.registerCommand('etamil.uploadToBoard', () => buildForBoard(context, true)),
    vscode.commands.registerCommand('etamil.install', () => offerInstall(context)),
    vscode.commands.registerCommand('etamil.installFont', () =>
      installFont(output, context.extensionPath)
    ),
    vscode.commands.registerCommand('etamil.documentation', () => showDocumentation()),
    vscode.commands.registerCommand('etamil.openExample', () => openExample()),
    vscode.commands.registerCommand('etamil.showOutput', () => output.show())
  );
}

/** The terminal named eTamil: ours carries ETAMIL_PATH, so a standard library import resolves. */
function etamilTerminal(): vscode.Terminal {
  const library = bundledLibrary();
  return (
    vscode.window.terminals.find((candidate) => candidate.name === 'eTamil') ??
    vscode.window.createTerminal({
      name: 'eTamil',
      env: library && !process.env.ETAMIL_PATH ? { ETAMIL_PATH: library } : undefined,
    })
  );
}

/**
 * Run the active file in a terminal.
 *
 * A terminal, not a task or a child process: an eTamil program reads stdin
 * (`உள்ளிடு` is in the language's headline example) and a server keeps
 * running, and both need a real console the author can type into and stop.
 */
async function runCurrentFile(mode: '--vm' | '--async'): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== LANGUAGE) {
    void vscode.window.showInformationMessage('eTamil: open an eTamil file first.');
    return;
  }

  if (editor.document.isDirty) {
    await editor.document.save();
  }

  // A terminal started by us carries ETAMIL_PATH, so an இறக்கு of a standard
  // library module resolves against the carried nUlakam. One the author opened
  // themselves does not, which is why this looks for ours by name.
  const terminal = etamilTerminal();
  terminal.show(true);

  // Every part is quoted, and quoted the way this shell wants — the carried
  // binary sits under the user's home directory, which may have a space in it.
  // Nothing else in the command line comes from the workspace; `compilerPath`
  // is machine-scoped for the same reason.
  terminal.sendText(
    terminalCommandLine(compilerPath(), [mode, editor.document.uri.fsPath]),
    true
  );
}

/** The boards `etamil --artino` builds for: the only values --board is given. */
const BOARDS: vscode.QuickPickItem[] = [
  { label: 'uno', description: 'Arduino Uno' },
  { label: 'nano', description: 'Arduino Nano' },
  { label: 'mega', description: 'Arduino Mega 2560' },
  { label: 'pico', description: 'Raspberry Pi Pico' },
  { label: 'pico2', description: 'Raspberry Pi Pico 2' },
];

/**
 * Build the active file as firmware with artino, and upload it if asked.
 *
 * The board comes from the fixed list above and the port is typed here, both
 * remembered in the workspace's state, never read from its settings: a cloned
 * repository's settings are its author's, and this runs a command. The port
 * must look like one, so nothing typed can end the quoting around it. The
 * sketch goes to artino-build beside the file.
 */
async function buildForBoard(context: vscode.ExtensionContext, upload: boolean): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== LANGUAGE) {
    void vscode.window.showInformationMessage('eTamil: open an eTamil file first.');
    return;
  }
  if (editor.document.isDirty) {
    await editor.document.save();
  }

  const last = context.workspaceState.get<string>('etamil.board');
  const items = BOARDS.map((board) => ({ ...board, picked: board.label === last }));
  const board = await vscode.window.showQuickPick(items, {
    placeHolder: last ? `Which board? Last time: ${last}` : 'Which board?',
  });
  if (!board) {
    return;
  }
  await context.workspaceState.update('etamil.board', board.label);

  const file = editor.document.uri.fsPath;
  const args = ['--artino', '--board', board.label, '--out', path.join(path.dirname(file), 'artino-build')];
  if (upload) {
    const port = await vscode.window.showInputBox({
      prompt: "The board's serial port",
      placeHolder: process.platform === 'win32' ? 'COM5' : '/dev/ttyACM0',
      value: context.workspaceState.get<string>('etamil.port'),
      validateInput: (value) =>
        /^[A-Za-z0-9_./\\:-]+$/.test(value) ? undefined : 'A port such as COM5 or /dev/ttyACM0',
    });
    if (!port) {
      return;
    }
    await context.workspaceState.update('etamil.port', port);
    args.push('--upload', port);
  }
  args.push(file);

  const terminal = etamilTerminal();
  terminal.show(true);
  terminal.sendText(terminalCommandLine(compilerPath(), args), true);
}

/**
 * Offer to install the compiler: download a package, or build from source.
 *
 * Three things are deliberate here. Nothing runs unless the author asked for it
 * in this dialog — there is no activation-time prompt and no automatic run. Any
 * command that does run is fixed in code, never read from settings: a
 * `.vscode/settings.json` inside a cloned repository is workspace data, and
 * executing a string from it is remote code execution. The custom option asks
 * the author to type the command, so what runs is something a person just chose.
 *
 * The download option holds that line too. It opens the release URL in a
 * browser and, if asked, copies the extract-and-install commands to the
 * clipboard. It does not fetch the archive, and it does not pipe anything
 * remote into a shell — the author downloads, sees what they have, and runs the
 * installer themselves.
 *
 * Since the extension began carrying the compiler, the first option is neither
 * a download nor a build: it copies the binary and the standard library that
 * are already on the disk, inside this VSIX, to `~/.local` or
 * `%LOCALAPPDATA%\Programs\eTamil`. Nothing is fetched and nothing is
 * compiled, so it is the one route that cannot fail for a reason outside the
 * machine. The others stay for the platforms no VSIX was built for.
 */
async function offerInstall(context: vscode.ExtensionContext): Promise<void> {
  const clone = 'git clone https://github.com/Maruff/eTamil_lang.git';
  const build =
    process.platform === 'win32'
      ? 'cd etamil_compiler\\etamil_compiler && cargo build --release'
      : 'cd etamil_compiler/etamil_compiler && cargo build --release';

  const download = downloadFor(process.platform, process.arch);

  const choices: Array<
    vscode.QuickPickItem & { command?: string; url?: string; copy?: boolean }
  > = [];

  if (carried()) {
    choices.push({
      label: 'Install the carried compiler',
      detail: 'Copies the binary and nUlakam out of this extension — nothing to download',
      description: 'already on this machine',
      copy: true,
    });
  }

  choices.push(
    download
      ? {
          label: 'Download the installer',
          detail: `${download.asset} — extract it, then run the install script`,
          description: 'no Rust, no C toolchain, no build',
          url: `${RELEASES_URL}/download/${download.asset}`,
        }
      : {
          label: 'Open the releases page',
          detail: `No prebuilt package for ${process.platform}-${process.arch} yet`,
          description: 'build from source instead',
          url: RELEASES_URL,
        },
    {
      label: 'Build from source',
      detail: `${clone} && ${build}`,
      description: 'needs Rust 1.85+ and a C toolchain',
      command: `${clone} && ${build}`,
    },
    {
      label: 'Enter a command…',
      detail: 'Type the command to run, if you install it another way',
    },
    {
      label: "Don't ask again",
      detail: 'Silence the missing-compiler warning in this workspace',
    }
  );

  const choice = await vscode.window.showQuickPick(choices, {
    title: 'Install the eTamil compiler',
    placeHolder: 'Nothing happens until you pick one',
  });

  if (!choice) {
    return;
  }

  if (choice.label === "Don't ask again") {
    await context.workspaceState.update(SKIP_INSTALL_PROMPT, true);
    return;
  }

  if (choice.copy) {
    await installCarried(output);
    return;
  }

  if (choice.url) {
    await vscode.env.openExternal(vscode.Uri.parse(choice.url));

    if (download) {
      const copy = 'Copy the commands';
      const picked = await vscode.window.showInformationMessage(
        `eTamil: once ${download.asset} has downloaded, extract it and run the ` +
          'install script — no administrator rights needed. Then reopen an eTamil file.',
        copy
      );
      if (picked === copy) {
        await vscode.env.clipboard.writeText(download.commands);
      }
    }
    return;
  }

  let command = choice.command;
  if (!command) {
    command = await vscode.window.showInputBox({
      title: 'Install command',
      prompt: 'Runs in a terminal in the current folder',
      placeHolder: 'cargo install --path etamil_compiler',
    });
  }
  if (!command) {
    return;
  }

  const terminal = vscode.window.createTerminal({ name: 'eTamil install' });
  terminal.show(true);
  terminal.sendText(command, true);

  void vscode.window.showInformationMessage(
    'eTamil: once the build finishes, reopen an eTamil file to pick it up. ' +
      'If the binary is not on your PATH, set etamil.compilerPath.'
  );
}

/** Whether the missing-compiler prompt has been silenced for this workspace. */
function installPromptSilenced(context: vscode.ExtensionContext): boolean {
  return context.workspaceState.get<boolean>(SKIP_INSTALL_PROMPT, false);
}
