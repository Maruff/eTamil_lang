// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// The two commands that answer "…and now what?"
//
// The extension could complete 681 standard library functions and never once
// say that a manual existed. Both of these are about that gap: one opens the
// documentation on etamil.in, the other opens one of the twenty-nine example
// programs the extension carries, so that the first eTamil file somebody reads
// is a working one rather than an empty buffer.

import { promises as fs } from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';

import { LINKS } from './links';
import { bundledExamples } from './toolchain';

/** Offer the documentation, and open what is chosen in a browser. */
export async function showDocumentation(): Promise<void> {
  const choice = await vscode.window.showQuickPick(
    LINKS.map((link) => ({
      label: link.label,
      detail: link.detail,
      description: link.url.replace(/^https:\/\//, ''),
      url: link.url,
    })),
    {
      title: 'eTamil documentation',
      placeHolder: 'Opens in your browser',
      matchOnDetail: true,
    }
  );
  if (choice) {
    await vscode.env.openExternal(vscode.Uri.parse(choice.url));
  }
}

/**
 * Open one of the carried example programs.
 *
 * Opened read-only from inside the extension would be the safe thing and the
 * useless thing: an example is for editing and running, and the extension
 * directory is replaced on every update. So the file is copied to a folder the
 * author chooses, the first time, and opened from there.
 */
export async function openExample(): Promise<void> {
  const examples = bundledExamples();
  if (!examples) {
    const browse = 'Browse them online';
    const picked = await vscode.window.showInformationMessage(
      'eTamil: this build does not carry the examples.',
      browse
    );
    if (picked === browse) {
      await vscode.env.openExternal(
        vscode.Uri.parse('https://github.com/Maruff/eTamil_lang/tree/main/examples')
      );
    }
    return;
  }

  const files = await collect(examples);
  if (files.length === 0) {
    void vscode.window.showInformationMessage('eTamil: no examples found.');
    return;
  }

  const choice = await vscode.window.showQuickPick(
    files.map((file) => {
      const relative = path.relative(examples, file).split(path.sep);
      return {
        label: relative[relative.length - 1],
        description: relative.slice(0, -1).join('/'),
        file,
      };
    }),
    { title: 'eTamil examples', placeHolder: 'Opens a copy you can edit and run' }
  );
  if (!choice) {
    return;
  }

  const destination = await vscode.window.showSaveDialog({
    title: 'Save a copy to work on',
    defaultUri: vscode.Uri.file(path.join(firstFolder(), choice.label)),
    filters: { eTamil: ['qmz'] },
  });
  if (!destination) {
    return;
  }

  await fs.copyFile(choice.file, destination.fsPath);
  const document = await vscode.workspace.openTextDocument(destination);
  await vscode.window.showTextDocument(document);
}

/** Where the save dialog starts: the workspace, or the home directory. */
function firstFolder(): string {
  const folder = vscode.workspace.workspaceFolders?.[0];
  return folder ? folder.uri.fsPath : os.homedir();
}

/** Every `.qmz` under a directory, sorted so the list is stable. */
async function collect(root: string): Promise<string[]> {
  const found: string[] = [];
  const walk = async (directory: string): Promise<void> => {
    const entries = await fs.readdir(directory, { withFileTypes: true });
    for (const entry of entries) {
      const full = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        await walk(full);
      } else if (entry.name.endsWith('.qmz')) {
        found.push(full);
      }
    }
  };
  await walk(root);
  return found.sort((a, b) => a.localeCompare(b));
}
