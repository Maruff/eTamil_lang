// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// Which compiler runs, and what it can see.
//
// Until this file existed the extension only knew how to *ask* for a compiler:
// it looked for `etamil` on the PATH and, failing that, offered a download
// page. That is a poor first five minutes. The extension now carries the
// binary and the standard library in the VSIX, so a fresh install can check
// and run a file with nothing else installed at all.
//
// The order is settings, then carried, then PATH:
//
//   `etamil.compilerPath`   an explicit choice always wins, so a developer
//                           working on the compiler runs their own build.
//   the carried binary      what a new user gets, with no setup.
//   `etamil` on the PATH    a separate installation, or a platform this VSIX
//                           was not built for.
//
// `ETAMIL_PATH` matters as much as the binary. `இறக்கு "nUlakam/paNam/paNam.qmz"`
// resolves beside the importing file, then along `ETAMIL_PATH`, then beside
// the executable; a carried compiler whose library the caller cannot see is a
// compiler that refuses every import in the standard library.

import { existsSync, promises as fs } from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';

import { bundleLayout, installLayout, pathAdvice } from './bundle';

let extensionPath = '';

/** Remember where the extension was unpacked. Called once, from `activate`. */
export function initialise(context: vscode.ExtensionContext): void {
  extensionPath = context.extensionPath;
}

function layout() {
  return bundleLayout(extensionPath, process.platform, process.arch);
}

async function exists(candidate: string): Promise<boolean> {
  try {
    await fs.stat(candidate);
    return true;
  } catch {
    return false;
  }
}

/**
 * The carried binary, if this VSIX was built for this platform.
 *
 * Synchronous on purpose. It is read on every `--check`, which happens on a
 * keystroke timer, and an `existsSync` on a path that is almost always the
 * same is cheaper than the promise machinery around it. The answer is cached
 * after the first call: the contents of a VSIX do not change while it is
 * loaded, and an update replaces the whole extension host.
 */
let carriedCompiler: string | null | undefined;

export function bundledCompiler(): string | undefined {
  if (carriedCompiler === undefined) {
    const candidate = layout().compiler;
    carriedCompiler = existsSync(candidate) ? candidate : null;
  }
  return carriedCompiler ?? undefined;
}

let carriedLibrary: string | null | undefined;

/** The carried `runtime` directory, if `nUlakam` is inside it. */
export function bundledLibrary(): string | undefined {
  if (carriedLibrary === undefined) {
    const candidate = layout().library;
    carriedLibrary = existsSync(path.join(candidate, 'nUlakam')) ? candidate : null;
  }
  return carriedLibrary ?? undefined;
}

let carriedExamples: string | null | undefined;

/** The carried `runtime/examples` directory, if it is there. */
export function bundledExamples(): string | undefined {
  if (carriedExamples === undefined) {
    const candidate = layout().examples;
    carriedExamples = existsSync(candidate) ? candidate : null;
  }
  return carriedExamples ?? undefined;
}

/**
 * The compiler command.
 *
 * The setting is read at machine scope only. A repository that could point
 * this at an arbitrary executable through its own `.vscode/settings.json`
 * would be a remote code execution vector, which is exactly the shape of the
 * `installCommand` problem this extension used to have.
 */
export function compilerPath(): string {
  const configured = vscode.workspace
    .getConfiguration('etamil')
    .get<string>('compilerPath');
  if (configured && configured.trim().length > 0) {
    return configured.trim();
  }
  return bundledCompiler() ?? 'etamil';
}

/**
 * The environment to run it in.
 *
 * `ETAMIL_PATH` is added only when the carried library is present and the user
 * has not set one of their own — theirs names a library they chose, and
 * silently prepending ours would import a different `nUlakam` than the one
 * they are reading.
 */
export function compilerEnv(): NodeJS.ProcessEnv {
  const library = bundledLibrary();
  if (!library || process.env.ETAMIL_PATH) {
    return process.env;
  }
  return { ...process.env, ETAMIL_PATH: library };
}

/**
 * A command line that runs `command` with `args` in the user's terminal.
 *
 * Quoting is the whole of it, and it is not the same everywhere. The carried
 * binary lives inside the extension directory, whose path contains the user's
 * name and so may contain a space — `C:\Users\Ada Lovelace\.vscode\…` — and an
 * unquoted path with a space is two arguments. Quoting it is enough for cmd
 * and for every POSIX shell, but **not** for PowerShell, where a command line
 * beginning with a quoted string is an expression that evaluates to that
 * string and prints it. PowerShell needs the call operator in front, and cmd
 * treats a leading `&` as a command separator and fails, so the two cannot
 * share a spelling.
 *
 * `vscode.env.shell` is the default shell VS Code will open, which is the one
 * this text is about to be typed into.
 */
export function terminalCommandLine(command: string, args: string[]): string {
  const shell = vscode.env.shell.toLowerCase();
  const powershell = shell.endsWith('powershell.exe') || shell.endsWith('pwsh.exe');
  const quoted = [command, ...args].map((part) => `"${part}"`).join(' ');
  return powershell ? `& ${quoted}` : quoted;
}

/** Whether anything is carried at all, for the install command's wording. */
export function carried(): boolean {
  return bundledCompiler() !== undefined;
}

/**
 * Copy the carried toolchain to a per-user location and say how to reach it.
 *
 * This is the "install" half of carrying it. In the editor nothing needs
 * installing — the extension runs the binary where it lies — but a terminal,
 * a build script or another editor cannot see inside a VSIX, and the VSIX is
 * replaced wholesale on every extension update, so anything pointing into it
 * would break. A copy under `~/.local` or `%LOCALAPPDATA%\Programs` is stable.
 *
 * The PATH line is shown, not applied. Editing a shell profile or the user's
 * registry environment is a change to how every program on the machine
 * starts, and an extension should not make it on a VSIX's behalf.
 */
export async function installCarried(
  output: vscode.LogOutputChannel
): Promise<boolean> {
  const source = layout();
  const compiler = bundledCompiler();
  if (!compiler) {
    return false;
  }

  const target = installLayout(
    process.platform,
    os.homedir(),
    process.env.LOCALAPPDATA
  );

  return vscode.window.withProgress(
    { location: vscode.ProgressLocation.Notification, title: 'Installing the eTamil compiler' },
    async () => {
      try {
        await fs.mkdir(target.bin, { recursive: true });
        const binary = path.join(target.bin, path.basename(compiler));
        await fs.copyFile(compiler, binary);
        if (process.platform !== 'win32') {
          // A file copied out of a VSIX is not executable: the archive does
          // not carry the bit, and neither does `copyFile`.
          await fs.chmod(binary, 0o755);
        }

        if (await exists(path.join(source.library, 'nUlakam'))) {
          await fs.rm(target.lib, { recursive: true, force: true });
          await fs.cp(source.library, target.lib, { recursive: true });
        }

        output.info(`installed ${binary}`);
        const copy = 'Copy the PATH lines';
        const advice = pathAdvice(process.platform, target);
        const picked = await vscode.window.showInformationMessage(
          `eTamil: installed to ${target.bin}. The extension already uses its own ` +
            'copy; these lines are for using etamil from a terminal.',
          copy
        );
        if (picked === copy) {
          await vscode.env.clipboard.writeText(advice);
        }
        return true;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        output.error(`install failed: ${message}`);
        void vscode.window.showErrorMessage(`eTamil: could not install the compiler — ${message}`);
        return false;
      }
    }
  );
}
