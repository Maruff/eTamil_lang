// Talking to the etamil binary.
//
// One rule governs this file: the extension never runs an eTamil program. It
// only ever invokes `etamil --check`, which stops after the type checker.
// Diagnostics from `--vm` would mean that opening a file in an editor wrote
// that file's output, issued its queries and started its server.

import { spawn } from 'child_process';
import * as path from 'path';
import * as vscode from 'vscode';

import { parseErrors, type CompilerError } from './errors';
import { compilerEnv, compilerPath } from './toolchain';

export type { CompilerError };

export interface CheckResult {
  errors: CompilerError[];
  /** Set when the binary could not be run at all, rather than when it found errors. */
  unavailable?: string;
}

/**
 * Check `source` as though it lived at `documentPath`.
 *
 * The text is piped in rather than read from disk so unsaved edits are what
 * gets checked, and the working directory is the document's own so that
 * `இறக்கு "nUlakam/atippatY/col.qmz"` — a path relative to the importing file —
 * resolves the way it will when the file is run.
 *
 * The environment comes from `compilerEnv`, which names the carried standard
 * library on `ETAMIL_PATH`. Without it a carried compiler refuses every import
 * in nUlakam, and every file that imports one reports an error that is not in
 * the file.
 */
export function check(
  source: string,
  documentPath: string,
  token?: vscode.CancellationToken
): Promise<CheckResult> {
  return new Promise((resolve) => {
    const command = compilerPath();
    const child = spawn(command, ['--check'], {
      cwd: path.dirname(documentPath),
      env: compilerEnv(),
      // `shell: false` is the default and must stay that way: the command is
      // machine-scoped, but there is no reason to hand it to a shell.
      shell: false,
    });

    let stderr = '';
    let settled = false;

    const finish = (result: CheckResult) => {
      if (!settled) {
        settled = true;
        resolve(result);
      }
    };

    token?.onCancellationRequested(() => {
      child.kill();
      finish({ errors: [] });
    });

    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });

    child.on('error', (error: NodeJS.ErrnoException) => {
      finish({
        errors: [],
        unavailable:
          error.code === 'ENOENT'
            ? `'${command}' was not found. Set etamil.compilerPath, or put the binary on your PATH.`
            : `Could not run '${command}': ${error.message}`,
      });
    });

    child.on('close', () => finish({ errors: parseErrors(stderr) }));

    child.stdin.on('error', () => {
      // The process died before the source was written; `close` reports it.
    });
    child.stdin.end(source, 'utf8');
  });
}

/**
 * Turn a compiler position into an editor one.
 *
 * The compiler counts columns in Unicode **code points**, and VS Code counts
 * UTF-16 code units, so the two agree for Tamil and Latin but not for anything
 * outside the BMP — an emoji in a string would shift every column after it.
 * Walking the line converts exactly, whatever it contains.
 *
 * Worth knowing: the compiler documents these columns as counting *written
 * letters*, which they do not — `LineCursor::at` walks code points, so a
 * position past a Tamil vowel sign reads high. If that is ever corrected to
 * count grapheme clusters, this function is the one place that needs to
 * change with it.
 */
export function toPosition(document: vscode.TextDocument, error: CompilerError): vscode.Range {
  const lineIndex = Math.max(0, Math.min(error.line - 1, document.lineCount - 1));
  const text = document.lineAt(lineIndex).text;

  let utf16 = 0;
  let codePoints = 0;
  for (const character of text) {
    if (codePoints >= error.column - 1) {
      break;
    }
    utf16 += character.length;
    codePoints += 1;
  }

  const start = new vscode.Position(lineIndex, Math.min(utf16, text.length));

  // Underline the token the error is about, rather than a single caret.
  const remainder = text.slice(start.character);
  const width = /^[a-zA-Z0-9_஀-௿]+/u.exec(remainder)?.[0].length ?? 1;

  return new vscode.Range(start, start.translate(0, Math.max(1, width)));
}
