// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Where the VM's input and output actually go.
//!
//! The interpreter used to call `std::fs` and `println!` directly, which tied
//! it to an operating system: it could not be built for the browser, and it
//! could not be tested without touching the disk. Both are now behind this
//! module.
//!
//! This is a `#[cfg]`-selected module rather than a trait the VM holds. A trait
//! would be the more general answer, but it would mean threading a `&mut dyn
//! Host` through every method of a 2,000-line interpreter and changing every
//! signature on the way. Swapping `fs::write(..)` for `host::write(..)` changes
//! the call and nothing else. If the VM ever needs two hosts alive at once --
//! two programs running side by side in one process -- this becomes a trait,
//! and the call sites do not move again.
//!
//! On wasm, output accumulates in a buffer the embedder drains, and files live
//! in memory. Both are `thread_local`: wasm32 is single-threaded, so there is
//! nothing to lock, and a native build gets one host per thread, which is what
//! a thread-per-request server wants.

// --- Native -----------------------------------------------------------------

#[cfg(not(target_family = "wasm"))]
mod imp {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::fs;
    use std::fs::OpenOptions;
    use std::io::Write as _;
    use std::path::Path;

    thread_local! {
        /// Where output goes when there is no console to send it to.
        ///
        /// `None` — the ordinary case — means print to stdout as before, so the
        /// command line, the REPL and the servers are unaffected. An embedder
        /// that has no stdout worth writing to calls `begin_capture` and drains
        /// the buffer afterwards.
        ///
        /// This exists for Android. An app is a native build, so it takes this
        /// module's native half and its `println!` goes to a stdout nobody
        /// reads; the browser half already solved the same problem, but a phone
        /// wants the rest of the native build — real files, real sockets — so it
        /// cannot simply borrow it.
        static CAPTURE: RefCell<Option<String>> = const { RefCell::new(None) };

        /// Lines handed to `உள்ளிடு` before it would reach for stdin.
        ///
        /// An embedder supplies a program's input up front, because there is
        /// nobody at a terminal to type it while the program runs.
        static INPUT: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
    }

    pub fn print_line(text: &str) {
        CAPTURE.with(|capture| match &mut *capture.borrow_mut() {
            Some(buffer) => {
                buffer.push_str(text);
                buffer.push('\n');
            }
            None => println!("{}", text),
        })
    }

    pub fn read_line() -> Result<String, String> {
        // Queued input first, whether or not output is being captured: a caller
        // may want to feed a program its answers even on the command line.
        if let Some(line) = INPUT.with(|input| input.borrow_mut().pop_front()) {
            return Ok(line);
        }

        // Falling through to stdin is right at a terminal and wrong in an app,
        // where it would block on a handle that never produces a byte and hang
        // the caller for good. Capture is what distinguishes the two, so an
        // exhausted queue reports itself the way the browser does rather than
        // waiting forever.
        if capturing() {
            return Err("உள்ளிடு தரவு தீர்ந்தது  \
                        (no input left to read)"
                .to_string());
        }

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;
        Ok(input)
    }

    pub fn read_to_string(path: &str) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| e.to_string())
    }

    pub fn write(path: &str, contents: &[u8]) -> Result<(), String> {
        fs::write(path, contents).map_err(|e| e.to_string())
    }

    pub fn append_line(path: &str, data: &str) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        writeln!(file, "{}", data).map_err(|e| e.to_string())
    }

    /// The entries of a directory, as bare names rather than paths.
    ///
    /// Bare names because joining is the caller's business and doing it here
    /// would decide the separator for them — `nUlakam/paNam/paNam.qmz` is how an
    /// author writes a path on every platform, and `\` leaking out of this
    /// function would put a backslash in the middle of one.
    ///
    /// Sorted, because a directory has no order of its own and two runs that
    /// disagree about it would make any program built on this
    /// non-reproducible.
    pub fn read_dir(path: &str) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        Ok(names)
    }

    /// Whether a path exists at all. Not a result: "no" is an answer, not a
    /// failure, and a permission error is indistinguishable from absence to
    /// the program either way.
    pub fn exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    /// What a path is: whether it is a directory, how large, and when it was
    /// last modified in seconds since 1970.
    ///
    /// The time is seconds rather than an ISO date because that is the form
    /// arithmetic works on; `கோப்புமுறை.qmz` converts it for display. A file
    /// system that cannot report a modification time yields `None` rather
    /// than zero — 1970 is a date, and a program comparing against it would
    /// quietly believe the file was ancient.
    pub fn metadata(path: &str) -> Result<(bool, u64, Option<i64>), String> {
        let data = fs::metadata(path).map_err(|e| e.to_string())?;
        let modified = data
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since| since.as_secs() as i64);
        Ok((data.is_dir(), data.len(), modified))
    }

    /// Flush both streams and end the process.
    ///
    /// Exiting does not unwind, so anything still buffered would be lost --
    /// including the summary line that explains the status.
    ///
    /// Under capture the process is not the program's to end: in an app it is
    /// the app, and `வெளியேறு` would take the whole thing down with it and lose
    /// the output that says why. So it reports the status instead, exactly as
    /// the browser does.
    pub fn exit(status: i32) -> Result<(), String> {
        if capturing() {
            return if status == 0 {
                Ok(())
            } else {
                Err(format!(
                    "நிரல் {} நிலையுடன் நின்றது  (the program exited with status {})",
                    status, status
                ))
            };
        }

        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        std::process::exit(status);
    }

    // --- Embedding ----------------------------------------------------------
    //
    // Only an embedder calls these. Nothing in the compiler does, and with
    // `begin_capture` never called this module behaves as it did before they
    // existed.

    /// Is output being collected rather than printed?
    pub fn capturing() -> bool {
        CAPTURE.with(|capture| capture.borrow().is_some())
    }

    /// Start collecting output, discarding anything already collected.
    pub fn begin_capture() {
        CAPTURE.with(|capture| *capture.borrow_mut() = Some(String::new()));
    }

    /// Stop collecting, and return everything collected since `begin_capture`.
    ///
    /// Also drops any input left unread, so one run cannot answer the next
    /// one's `உள்ளிடு`.
    pub fn end_capture() -> String {
        INPUT.with(|input| input.borrow_mut().clear());
        CAPTURE
            .with(|capture| capture.borrow_mut().take())
            .unwrap_or_default()
    }

    /// Queue one line for `உள்ளிடு`, in the order the program will read them.
    ///
    /// The trailing newline is added here because that is what a real
    /// `read_line` returns, and `உள்ளிடு` trims it.
    pub fn push_input(line: &str) {
        INPUT.with(|input| input.borrow_mut().push_back(format!("{}\n", line)));
    }
}

// --- Browser ----------------------------------------------------------------

#[cfg(target_family = "wasm")]
mod imp {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::VecDeque;

    thread_local! {
        /// Everything the program has printed, waiting to be drained.
        static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
        /// The program's filesystem. Starts empty every run.
        static FILES: RefCell<HashMap<String, Vec<u8>>> =
            RefCell::new(HashMap::new());
        /// Lines waiting for `உள்ளிடு`, in the order it will read them.
        ///
        /// A page has no console to type at, so it supplies a program's
        /// input before the run rather than during it — the same arrangement
        /// the native half offers an embedder, and the reason this is a queue
        /// and not a callback: nothing here can block waiting for a key.
        static INPUT: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
    }

    pub fn print_line(text: &str) {
        OUTPUT.with(|out| {
            let mut out = out.borrow_mut();
            out.push_str(text);
            out.push('\n');
        });
    }

    /// Take the next queued line.
    ///
    /// There is no stdin to fall back to, so this is the native half's
    /// capturing branch and nothing else: what was queued, then an error.
    /// An error rather than an empty string means `உள்ளிடு` says why it
    /// did nothing instead of silently yielding "".
    pub fn read_line() -> Result<String, String> {
        if let Some(line) = INPUT.with(|input| input.borrow_mut().pop_front()) {
            return Ok(line);
        }
        Err("உள்ளிடு தரவு தீர்ந்தது  \
             (no input left to read)"
            .to_string())
    }

    pub fn read_to_string(path: &str) -> Result<String, String> {
        FILES.with(|files| {
            files
                .borrow()
                .get(path)
                .ok_or_else(|| format!("கோப்பு '{}' இல்லை  (no such file '{}')", path, path))
                .and_then(|bytes| {
                    String::from_utf8(bytes.clone()).map_err(|_| {
                        format!(
                            "கோப்பு '{}' UTF-8 அல்ல  ('{}' is not valid UTF-8)",
                            path, path
                        )
                    })
                })
        })
    }

    pub fn write(path: &str, contents: &[u8]) -> Result<(), String> {
        FILES.with(|files| {
            files
                .borrow_mut()
                .insert(path.to_string(), contents.to_vec());
        });
        Ok(())
    }

    pub fn append_line(path: &str, data: &str) -> Result<(), String> {
        FILES.with(|files| {
            let mut files = files.borrow_mut();
            let entry = files.entry(path.to_string()).or_default();
            entry.extend_from_slice(data.as_bytes());
            entry.push(b'\n');
        });
        Ok(())
    }

    /// The entries of a directory in the in-memory file set.
    ///
    /// There are no directories here — the browser side is a flat map of path
    /// to bytes — so a directory is inferred: anything whose path begins with
    /// `path/`. The segment immediately after the prefix is the entry name,
    /// and a name that has more path after it is a subdirectory, reported
    /// once. That makes the browser behave like the native side for the thing
    /// programs actually do, which is walk a tree.
    pub fn read_dir(path: &str) -> Result<Vec<String>, String> {
        let prefix = if path.is_empty() || path == "." {
            String::new()
        } else {
            format!("{}/", path.trim_end_matches('/'))
        };

        let mut names: Vec<String> = FILES.with(|files| {
            files
                .borrow()
                .keys()
                .filter_map(|key| key.strip_prefix(&prefix))
                .filter(|rest| !rest.is_empty())
                .map(|rest| match rest.find('/') {
                    Some(cut) => rest[..cut].to_string(),
                    None => rest.to_string(),
                })
                .collect()
        });

        if names.is_empty() && !prefix.is_empty() {
            return Err(format!(
                "கோப்பகம் '{}' இல்லை  (no such directory '{}')",
                path, path
            ));
        }

        names.sort();
        names.dedup();
        Ok(names)
    }

    pub fn exists(path: &str) -> bool {
        FILES.with(|files| {
            let files = files.borrow();
            files.contains_key(path)
                || files
                    .keys()
                    .any(|key| key.starts_with(&format!("{}/", path.trim_end_matches('/'))))
        })
    }

    /// A path's kind and size in the in-memory file set.
    ///
    /// The modification time is `None`, not zero: nothing here was ever
    /// written to a disk that recorded when. Reporting 1970 would be a date,
    /// and a program comparing against it would quietly believe every file in
    /// the browser was ancient.
    pub fn metadata(path: &str) -> Result<(bool, u64, Option<i64>), String> {
        FILES.with(|files| {
            let files = files.borrow();
            if let Some(bytes) = files.get(path) {
                return Ok((false, bytes.len() as u64, None));
            }
            let prefix = format!("{}/", path.trim_end_matches('/'));
            if files.keys().any(|key| key.starts_with(&prefix)) {
                return Ok((true, 0, None));
            }
            Err(format!("கோப்பு '{}' இல்லை  (no such file '{}')", path, path))
        })
    }

    /// A non-zero exit is the program's own failure and is reported as one.
    /// There is no process to end.
    pub fn exit(status: i32) -> Result<(), String> {
        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "நிரல் {} நிலையுடன் நின்றது  (the program exited with status {})",
                status, status
            ))
        }
    }

    /// Take everything printed so far, leaving the buffer empty.
    pub fn take_output() -> String {
        OUTPUT.with(|out| std::mem::take(&mut *out.borrow_mut()))
    }

    /// Clear output, files and queued input. Called before each run so one
    /// program cannot see the last one's leftovers.
    pub fn reset() {
        OUTPUT.with(|out| out.borrow_mut().clear());
        FILES.with(|files| files.borrow_mut().clear());
        INPUT.with(|input| input.borrow_mut().clear());
    }

    /// Seed a file before a run, so an example can read input it did not write.
    pub fn put_file(path: &str, contents: &str) {
        let _ = write(path, contents.as_bytes());
    }

    /// Queue one line for `உள்ளிடு`, in the order the program reads them.
    ///
    /// The trailing newline is added here because that is what a real
    /// `read_line` returns, and `உள்ளிடு` trims it.
    pub fn push_input(line: &str) {
        INPUT.with(|input| input.borrow_mut().push_back(format!("{}\n", line)));
    }

    /// Paths the program wrote, sorted, so the embedder can show them.
    pub fn file_names() -> Vec<String> {
        FILES.with(|files| {
            let mut names: Vec<String> = files.borrow().keys().cloned().collect();
            names.sort();
            names
        })
    }
}

pub use imp::*;
