eTamil @VERSION@  —  @OS@ x64
=====================================================================

A programming language whose vocabulary is Tamil, for Indian FinTech.

Nothing else needs installing. Rust, LLVM and Visual Studio are needed to
BUILD the compiler, not to run it. This package is self-contained.


INSTALL
-------

Windows        powershell -ExecutionPolicy Bypass -File install.ps1
Linux / macOS  ./install.sh

Then open a NEW terminal:

    etamil --version
    etamil --help


FIRST PROGRAM
-------------

Save this as vanakkam.qmz, encoded as UTF-8:

    அச்சு "வணக்கம் உலகம்!";

Run it:

    etamil --vm vanakkam.qmz


MONEY IS EXACT
--------------

Every number is a fixed-point decimal, so the arithmetic a tax program
performs comes out right:

    அச்சு 0.1 + 0.2;        →  0.3      (not 0.30000000000000004)
    அச்சு 99.99 * 3;        →  299.97

Formatting follows Indian convention — three digits, then pairs:

    இறக்கு "nUlakam/paNam/paNam.qmz";
    அச்சு ரூபாய்(12345678.5);   →  ₹1,23,45,678.50


WHAT IS IN THE BOX
------------------

etamil          the compiler; runs programs on its bytecode VM, and serves
                HTTP with --server
nUlakam/        the standard library — strings, maths, arrays, money — and
                kaNakkiyal/, an accounting framework with double entry, GST
                and the three financial statements. Written in eTamil, so
                you can read and change it without rebuilding anything.
examples/       working programs, including a GST invoice, a payroll run and
                a full accounting cycle


TRY THE EXAMPLES
----------------

    etamil --vm examples/finance/vaNikavari_pattiyal.qmz    GST invoice
    etamil --vm examples/finance/kaNakkiyal.qmz             accounting cycle
    etamil --vm examples/language/ceyalkaL.qmz              functions, errors

The examples in examples/db_samples/ write files, so run those from a
directory you do not mind writing to.


SAVE YOUR FILES AS UTF-8
------------------------

Tamil source must be UTF-8. A byte-order mark is fine — the compiler skips
it. In Notepad choose "UTF-8" when saving; VS Code shows the encoding in the
status bar.


DOCUMENTATION
-------------

Website     https://etamil.in
Source      https://github.com/Maruff/etamil_compiler
Language    docs/reference/KEYWORDS.md — all 201 keywords in three spellings
Roadmap     docs/ROADMAP.md — what is not built yet, and why


WHAT IS NOT FINISHED
--------------------

Being straight with you, since a version number does not say much:

    - PostgreSQL, MySQL, MongoDB and Redis are not included in this default
        package. SQLite works; server database drivers require a source build with
        the corresponding Cargo feature.
    - The LLVM backend (--llvm) requires a source build with LLVM 18 and is not
        included in this package. The VM is the default execution path.

Anything unimplemented fails with a clear message rather than quietly doing
nothing. In a tax calculator a silent wrong answer is worse than an error.
