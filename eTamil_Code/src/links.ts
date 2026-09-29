// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// Where the documentation actually is.
//
// The extension knows the language — every keyword, every spelling, every
// `செயல்` in nUlakam — and none of that tells anyone *how to write a program*.
// That is on etamil.in, and until this file existed the extension never
// mentioned it. Someone could install the extension, get completions for 681
// standard library functions, and have no idea there was a manual.
//
// One table, used by the documentation command, by the hovers and by the
// README, so a moved page is one edit rather than a search. `test/links.test.js`
// asserts the shape of every entry; the URLs themselves were checked by hand
// and each returned 200.
//
// Nothing here imports the vscode API.

/** One place to send someone, and why they would go. */
export interface Link {
  label: string;
  detail: string;
  url: string;
}

const SITE = 'https://etamil.in';

/** The repository, which is not on the website. */
export const REPOSITORY = 'https://github.com/Maruff/eTamil_lang';

/**
 * Everything the documentation command offers, in the order it offers it.
 *
 * The manual is first because it is the answer to the question someone with an
 * empty `.qmz` file is actually asking. The playground is second because it is
 * the fastest way to see the language run without trusting anything on your
 * own machine.
 */
export const LINKS: Link[] = [
  {
    label: 'User manual',
    detail: 'Installation to database-backed HTTP services, in order',
    url: `${SITE}/language/manual/`,
  },
  {
    label: 'Playground',
    detail: 'Run eTamil in the browser — the compiler, built to WebAssembly',
    url: `${SITE}/start/`,
  },
  {
    label: 'Language tour',
    detail: 'The language in editable, runnable examples',
    url: `${SITE}/language/`,
  },
  {
    label: 'Keyword reference',
    detail: 'Every keyword in all three spellings — Tamil, ezuqqu, English',
    url: `${SITE}/language/keywords/`,
  },
  {
    label: 'Finance and accounting',
    detail: 'The accounting framework: ledgers, GST, tax',
    url: `${SITE}/finance/`,
  },
  {
    label: 'Databases and the HTTP server',
    detail: 'Building services in eTamil',
    url: `${SITE}/server/`,
  },
  {
    label: 'Status and roadmap',
    detail: 'What works today, and what does not yet',
    url: `${SITE}/status/`,
  },
  {
    label: 'Source code',
    detail: 'The compiler, the standard library and this extension',
    url: REPOSITORY,
  },
];

/** The manual, which several places want by itself. */
export const MANUAL = LINKS[0].url;

/** The keyword reference, which every keyword hover points at. */
export const KEYWORDS_REFERENCE = `${SITE}/language/keywords/`;
