;;; etamil-mode.el --- Major mode for eTamil  -*- lexical-binding: t -*-

;; Copyright (C) 2026 Mohammed Maruff (Esan Maruff)

;; Author: Mohammed Maruff (Esan Maruff) <esan@etamil.in>
;; URL: https://github.com/Maruff/eTamil_lang
;; Version: 0.1.0
;; Package-Requires: ((emacs "27.1"))
;; Keywords: languages
;; SPDX-License-Identifier: AGPL-3.0-or-later

;;; Commentary:

;; A major mode for eTamil, a programming language whose keywords are Tamil words,
;; for `.qmz' files.
;;
;; - Highlighting of keywords, types, constants and built-in functions, in Tamil
;;   script and in Latin letters, from word lists generated from the compiler's own
;;   lexer (etamil-words.el), so a keyword the compiler accepts is never missed.
;; - `//' line comments, strings, numbers and percentages such as 7.5%.
;; - Indentation by bracket depth.
;; - C-c C-c runs the file and C-c C-k checks it (`etamil --check', which never
;;   runs the program), in a compilation buffer.
;; - Diagnostics, completion and hover from the language server `etamil-lsp'
;;   through Eglot: M-x eglot in an eTamil buffer.
;;
;; Install by putting this directory on `load-path' and requiring the mode:
;;
;;   (add-to-list 'load-path "/path/to/eTamil_Emacs")
;;   (require 'etamil-mode)
;;
;; `.qmz' files then open in `etamil-mode'.

;;; Code:

(require 'etamil-words)

(defvar electric-indent-chars)
(defvar eglot-server-programs)

(defgroup etamil nil
  "Editing eTamil programs."
  :group 'languages
  :prefix "etamil-")

(defcustom etamil-indent-offset 4
  "Columns to indent by for each open bracket."
  :type 'integer
  :safe #'integerp
  :group 'etamil)

(defcustom etamil-executable "etamil"
  "The eTamil compiler, used to run and check a file."
  :type 'string
  :group 'etamil)

(defcustom etamil-lsp-executable "etamil-lsp"
  "The eTamil language server, started by Eglot."
  :type 'string
  :group 'etamil)

;;; Syntax

(defvar etamil-mode-syntax-table
  (let ((table (make-syntax-table)))
    ;; The Tamil block: letters, vowel signs and the virama are all part of a word.
    ;; Without this a name can be cut in the middle of a letter.
    (modify-syntax-entry '(#xB80 . #xBFF) "w" table)
    (modify-syntax-entry ?_ "_" table)
    ;; `//' to the end of the line is the only comment: there is no block comment.
    (modify-syntax-entry ?/ ". 12" table)
    (modify-syntax-entry ?\n ">" table)
    ;; Strings may span lines.
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    (modify-syntax-entry ?{ "(}" table)
    (modify-syntax-entry ?} "){" table)
    (modify-syntax-entry ?\( "()" table)
    (modify-syntax-entry ?\) ")(" table)
    (modify-syntax-entry ?\[ "(]" table)
    (modify-syntax-entry ?\] ")[" table)
    (dolist (char '(?+ ?- ?* ?= ?< ?> ?& ?% ?! ?: ?, ?\; ?. ??))
      (modify-syntax-entry char "." table))
    table)
  "Syntax table for `etamil-mode'.")

;;; Highlighting

(defconst etamil--word-faces
  (let ((table (make-hash-table :test 'equal :size 4096)))
    (dolist (entry (list (cons etamil-keywords 'font-lock-keyword-face)
                         (cons etamil-types 'font-lock-type-face)
                         (cons etamil-constants 'font-lock-constant-face)
                         (cons etamil-builtins 'font-lock-builtin-face)))
      (dolist (word (car entry))
        (puthash word (cdr entry) table)))
    table)
  "Each known word, mapped to its face.
A lookup, not one large regexp: there are well over a thousand words, and
matching a symbol and asking a hash table is both simpler and faster.")

(defun etamil--face-for (word)
  "The face for WORD, or nil if it is not one of the language's own words."
  (gethash word etamil--word-faces))

(defvar etamil-font-lock-keywords
  `(;; 50000, 7.5 and 18%.  A percentage ends in `%', which is not part of a symbol.
    ("\\_<[0-9]+\\(?:\\.[0-9]+\\)?\\(?:%\\|\\_>\\)" . font-lock-constant-face)
    ;; Any other symbol is looked up; a name the author chose has no face.
    ("\\_<\\(?:\\sw\\|\\s_\\)+\\_>"
     (0 (etamil--face-for (match-string-no-properties 0)) nil)))
  "Font Lock keywords for `etamil-mode'.
Strings and comments come from the syntax table, which runs first, so a keyword
inside either is left alone.")

;;; Indentation

(defun etamil--indentation ()
  "The column the line at point should start at.
One level per open bracket, and a line that begins with a closing bracket sits one
level out."
  (save-excursion
    (back-to-indentation)
    (let ((depth (car (syntax-ppss))))
      (when (looking-at "[]})]")
        (setq depth (1- depth)))
      (* (max depth 0) etamil-indent-offset))))

(defun etamil-indent-line ()
  "Indent the current line by bracket depth.
A line that begins inside a string, which can span lines, is left as written."
  (unless (nth 3 (save-excursion (syntax-ppss (line-beginning-position))))
    (let ((offset (- (current-column) (current-indentation))))
      (indent-line-to (etamil--indentation))
      (when (> offset 0)
        (forward-char offset)))))

;;; Running and checking

(defun etamil--command (mode file)
  "The shell command that does MODE, `run' or `check', to FILE."
  (mapconcat #'identity
             (list (shell-quote-argument etamil-executable)
                   (pcase mode
                     ('run "--vm")
                     ('check "--check")
                     (_ (error "Unknown mode %S" mode)))
                   (shell-quote-argument file))
             " "))

(defun etamil--compile (mode)
  "Save the buffer, then run the compiler on it as MODE."
  (unless buffer-file-name
    (user-error "Save the buffer to a .qmz file first"))
  (when (buffer-modified-p)
    (save-buffer))
  ;; From the file's own folder, so relative paths inside the program work.
  (let ((default-directory (file-name-directory buffer-file-name)))
    (compile (etamil--command mode buffer-file-name))))

(defun etamil-run-file ()
  "Run the file in this buffer."
  (interactive)
  (etamil--compile 'run))

(defun etamil-check-file ()
  "Check the file in this buffer without running it."
  (interactive)
  (etamil--compile 'check))

;;; The mode

(defvar etamil-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "C-c C-c") #'etamil-run-file)
    (define-key map (kbd "C-c C-k") #'etamil-check-file)
    map)
  "Keymap for `etamil-mode'.")

;;;###autoload
(define-derived-mode etamil-mode prog-mode "eTamil"
  "Major mode for editing eTamil programs.

\\{etamil-mode-map}"
  :syntax-table etamil-mode-syntax-table
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "//+\\s-*")
  (setq-local font-lock-defaults '(etamil-font-lock-keywords))
  (setq-local indent-line-function #'etamil-indent-line)
  (setq-local electric-indent-chars
              (cons ?\} (bound-and-true-p electric-indent-chars))))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.qmz\\'" . etamil-mode))

;; Eglot is built in from Emacs 29, and a package from earlier. Nothing here
;; needs it loaded: the entry is made when it is.
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               `(etamil-mode . (,etamil-lsp-executable))))

(provide 'etamil-mode)

;;; etamil-mode.el ends here
