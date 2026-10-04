;;; etamil-mode-tests.el --- Tests for etamil-mode  -*- lexical-binding: t -*-

;; SPDX-License-Identifier: AGPL-3.0-or-later

;;; Commentary:

;; Run from the repository root:
;;
;;   emacs -Q --batch -L eTamil_Emacs -l ert -l eTamil_Emacs/etamil-mode-tests.el \
;;         -f ert-run-tests-batch-and-exit

;;; Code:

(require 'ert)
(require 'cl-lib)
(require 'etamil-mode)

(defconst etamil-test-sample
  "// simple interest
செயல் வட்டி_கணக்கு(அசல், வீதம், ஆண்டு) {
வட்டி = அசல் * வீதம் * ஆண்டு;
(வட்டி > 10000) எனில் {
அச்சு(\"வரி\");
}
திரும்பு வட்டி;
}
தொகை = 50000;
அச்சு(வட்டி_கணக்கு(தொகை, 7.5%, 3));
"
  "A program with a comment, a function, a condition, a string and the literals.")

(defmacro etamil-test-with-sample (&rest body)
  "Run BODY in an `etamil-mode' buffer holding the sample, fontified."
  (declare (indent 0))
  `(with-temp-buffer
     (insert etamil-test-sample)
     (etamil-mode)
     (font-lock-ensure)
     (goto-char (point-min))
     ,@body))

(defun etamil-test-face (needle)
  "The face on the first NEEDLE in the buffer, as one symbol or nil."
  (goto-char (point-min))
  (search-forward needle)
  (let ((face (get-text-property (match-beginning 0) 'face)))
    (if (consp face) (car face) face)))

;;; The word lists

(ert-deftest etamil-words-are-lists-of-strings ()
  (dolist (list (list etamil-keywords etamil-types etamil-constants etamil-builtins))
    (should (consp list))
    (should (cl-every #'stringp list))))

(ert-deftest etamil-words-are-in-one-list-only ()
  (let ((all (append etamil-keywords etamil-types etamil-constants etamil-builtins)))
    (should (= (length all) (length (delete-dups (copy-sequence all)))))))

(ert-deftest etamil-words-know-the-core-keywords ()
  (should (member "எனில்" etamil-keywords))
  (should (member "செயல்" etamil-keywords))
  (should (member "அச்சு" etamil-keywords)))

;;; Syntax

(ert-deftest etamil-tamil-letters-and-marks-are-word-constituents ()
  (should (eq (char-syntax ?க) ?w))
  (should (eq (char-syntax #xBCD) ?w)) ; the virama, which joins a consonant to the next
  (should (eq (char-syntax ?_) ?_)))

(ert-deftest etamil-line-comment-and-string ()
  (etamil-test-with-sample
    (search-forward "simple")
    (should (nth 4 (syntax-ppss)))
    (search-forward "வரி")
    (should (nth 3 (syntax-ppss)))
    (search-forward "திரும்பு")
    (should-not (nth 3 (syntax-ppss)))
    (should-not (nth 4 (syntax-ppss)))))

(ert-deftest etamil-a-single-slash-is-not-a-comment ()
  (with-temp-buffer
    (insert "x = 6 / 2;")
    (etamil-mode)
    (goto-char (point-max))
    (should-not (nth 4 (syntax-ppss)))))

(ert-deftest etamil-comment-settings ()
  (with-temp-buffer
    (etamil-mode)
    (should (equal comment-start "// "))))

;;; Highlighting

(ert-deftest etamil-keywords-are-highlighted ()
  (etamil-test-with-sample
    (should (eq (etamil-test-face "செயல்") 'font-lock-keyword-face))
    (should (eq (etamil-test-face "எனில்") 'font-lock-keyword-face))
    (should (eq (etamil-test-face "திரும்பு") 'font-lock-keyword-face))
    (should (eq (etamil-test-face "அச்சு") 'font-lock-keyword-face))))

(ert-deftest etamil-a-name-the-author-chose-has-no-face ()
  (etamil-test-with-sample
    (should-not (etamil-test-face "வட்டி_கணக்கு"))))

(ert-deftest etamil-literals-comments-and-strings-are-highlighted ()
  (etamil-test-with-sample
    (should (eq (etamil-test-face "// simple") 'font-lock-comment-face))
    (should (eq (etamil-test-face "\"வரி\"") 'font-lock-string-face))
    (should (eq (etamil-test-face "50000") 'font-lock-constant-face))
    (should (eq (etamil-test-face "7.5%") 'font-lock-constant-face))
    (should (eq (etamil-test-face "10000") 'font-lock-constant-face))))

(ert-deftest etamil-a-keyword-inside-a-string-or-comment-keeps-that-face ()
  (with-temp-buffer
    (insert "// எனில் here\nஅச்சு(\"செயல்\");\n")
    (etamil-mode)
    (font-lock-ensure)
    (should (eq (etamil-test-face "எனில்") 'font-lock-comment-face))
    (should (eq (etamil-test-face "செயல்") 'font-lock-string-face))))

(ert-deftest etamil-latin-spellings-are-highlighted-too ()
  (with-temp-buffer
    (insert "eZil\n")
    (etamil-mode)
    (font-lock-ensure)
    (should (eq (etamil-test-face "eZil") 'font-lock-keyword-face))))

;;; Indentation

(ert-deftest etamil-indents-by-bracket-depth ()
  (with-temp-buffer
    (insert "செயல் f(a) {
(a > 1) எனில் {
அச்சு(a);
}
திரும்பு a;
}
")
    (etamil-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string)
                   "செயல் f(a) {
    (a > 1) எனில் {
        அச்சு(a);
    }
    திரும்பு a;
}
"))))

(ert-deftest etamil-indent-offset-is-respected ()
  (with-temp-buffer
    (insert "{\nx;\n}\n")
    (etamil-mode)
    (setq-local etamil-indent-offset 2)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string) "{\n  x;\n}\n"))))

(ert-deftest etamil-leaves-the-inside-of-a-multi-line-string-alone ()
  (with-temp-buffer
    (insert "x = \"line one\n   line two\";\n")
    (etamil-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string) "x = \"line one\n   line two\";\n"))))

;;; Commands and registration

(ert-deftest etamil-builds-the-run-and-check-commands ()
  (let ((etamil-executable "etamil"))
    (should (equal (etamil--command 'run "/p/a.qmz") "etamil --vm /p/a.qmz"))
    (should (equal (etamil--command 'check "/p/a b.qmz") "etamil --check /p/a\\ b.qmz"))
    (should-error (etamil--command 'other "/p/a.qmz"))))

(ert-deftest etamil-running-an-unsaved-buffer-is-refused ()
  (with-temp-buffer
    (etamil-mode)
    (should-error (etamil-run-file) :type 'user-error)))

(ert-deftest etamil-qmz-files-open-in-the-mode ()
  (should (eq (assoc-default "program.qmz" auto-mode-alist #'string-match-p) 'etamil-mode)))

(ert-deftest etamil-the-language-server-is-registered-with-eglot ()
  (skip-unless (require 'eglot nil t))
  (should (assq 'etamil-mode eglot-server-programs)))

(ert-deftest etamil-the-mode-has-its-keys ()
  (should (eq (lookup-key etamil-mode-map (kbd "C-c C-c")) 'etamil-run-file))
  (should (eq (lookup-key etamil-mode-map (kbd "C-c C-k")) 'etamil-check-file)))

(provide 'etamil-mode-tests)

;;; etamil-mode-tests.el ends here
