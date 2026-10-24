// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Keeping connections alive between requests.
//!
//! Every HTTP request gets a fresh VM, so a handler that opens a database
//! opened a new connection each time and threw it away — a TCP connect, a TLS
//! handshake and an authentication round trip per request, to run one query.
//!
//! What this is **not** is a shared connection. Two requests using one
//! connection would share its transaction state: a `BEGIN` in one would
//! enclose the other's queries, and a `COMMIT` in one would commit the other's
//! half-finished work. Order placement in examples/katY depends on exactly
//! that not happening. So a connection is leased *exclusively* for as long as
//! a VM holds it, and only returns to the cache when the VM is done with it.
//!
//! It is an idle cache with a cap rather than a fixed-size pool: if nothing is
//! idle, a new connection is opened rather than waiting for one. A fixed pool
//! would need blocking and a bound on how long to block, and a request that
//! deadlocks waiting for a connection is worse than one that opens a second.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::Database;

/// How many idle connections to keep per database. Beyond this, a returning
/// connection is closed instead of cached.
fn idle_cap() -> usize {
    std::env::var("ETAMIL_DB_IDLE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8)
}

/// One database, identified the way the program named it: the same connection
/// string against a different backend is a different pool.
type Key = (String, String);

/// Idle connections per key. Named because the type appears twice below and
/// clippy is right that the spelled-out form is hard to read.
type Idle = HashMap<Key, Vec<Box<dyn Database>>>;

fn cache() -> &'static Mutex<Idle> {
    static CACHE: OnceLock<Mutex<Idle>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A connection borrowed from the cache, returned when dropped.
pub struct Lease {
    /// None only after `close` has taken it.
    handle: Option<Box<dyn Database>>,
    /// Which cache to return to, or None for a connection that never came
    /// from one — see `detached`.
    key: Option<Key>,
}

impl Lease {
    /// The borrowed connection.
    ///
    /// Not `as_mut`: that is `std::convert::AsMut`'s method name, and a reader
    /// seeing `lease.as_mut()` would reasonably expect the trait.
    pub fn connection(&mut self) -> &mut dyn Database {
        // Held for the whole life of the Lease; `close` consumes self.
        self.handle
            .as_mut()
            .expect("a lease holds its connection until it is dropped")
            .as_mut()
    }

    /// Wrap a connection that did not come from the cache.
    ///
    /// For driving the VM against a stand-in backend: the `Database` trait has
    /// no driver dependency precisely so that is possible. A detached lease is
    /// neither rolled back nor cached on release — a test asserting on the SQL
    /// a program issued should not find a ROLLBACK it never wrote, and a fake
    /// has no business in a pool of real connections.
    pub fn detached(handle: Box<dyn Database>) -> Self {
        Lease {
            handle: Some(handle),
            key: None,
        }
    }

    /// Close for real, rather than returning to the cache.
    pub fn close(mut self) -> Result<(), String> {
        match self.handle.take() {
            Some(mut handle) => handle.close(),
            None => Ok(()),
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let Some(mut handle) = self.handle.take() else {
            return;
        };
        let Some(key) = self.key.clone() else {
            return; // detached: just close it
        };

        // A handler that opened a transaction and then failed — or simply
        // forgot to COMMIT — would otherwise hand the next request a
        // connection sitting mid-transaction, where its first statement joins
        // someone else's unit of work. Rolling back on return makes a leaked
        // transaction impossible.
        //
        // The error is ignored on purpose: with no transaction open, SQLite
        // reports "cannot rollback - no transaction is active", while
        // PostgreSQL and MySQL succeed with a warning. Nothing here can tell
        // the difference, and there is nothing to do about it either way.
        let _ = handle.execute("ROLLBACK", &[]);

        // A function registered for this request would otherwise be the next
        // request's, still running this request's program.
        handle.unregister_functions();

        if let Ok(mut cache) = cache().lock() {
            let idle = cache.entry(key).or_default();
            if idle.len() < idle_cap() {
                idle.push(handle);
            }
            // Otherwise `handle` drops here and the driver closes it.
        }
        // A poisoned cache means another thread panicked holding the lock.
        // Dropping the connection is the safe response; the alternative is
        // propagating a panic out of a destructor.
    }
}

/// Borrow a connection, opening one if none is idle.
/// Is this connection string a private, per-connection database?
///
/// SQLite gives every connection to `:memory:` its own database — the name is
/// not an address, it is a request for somewhere private. Caching one by that
/// name therefore conflates databases that were meant to be separate: two
/// unrelated pieces of code asking for `:memory:` were handed the *same*
/// database, so one could see the other's tables and rows.
///
/// It showed up as tests interfering with each other, which is the mild version
/// of the problem. The sharp version is a program that opens a scratch database
/// per request and finds another request's data in it.
fn is_private(connection: &str) -> bool {
    let trimmed = connection.trim();
    trimmed == ":memory:"
        || trimmed == "file::memory:"
        // A URI asking for memory without asking for a shared cache is also
        // private; one that asks for `cache=shared` is deliberately not.
        || (trimmed.contains("mode=memory") && !trimmed.contains("cache=shared"))
}

pub fn checkout(db_type: &str, connection: &str) -> Result<Lease, String> {
    let key = (db_type.to_string(), connection.to_string());

    // Never pooled, and never returned to the cache on release: see is_private.
    if is_private(connection) {
        return Ok(Lease {
            handle: Some(super::open(db_type, connection)?),
            key: None,
        });
    }

    if let Ok(mut cache) = cache().lock()
        && let Some(idle) = cache.get_mut(&key)
        && let Some(handle) = idle.pop()
    {
        return Ok(Lease {
            handle: Some(handle),
            key: Some(key),
        });
    }

    Ok(Lease {
        handle: Some(super::open(db_type, connection)?),
        key: Some(key),
    })
}

/// How many connections are cached for a database. For tests and diagnostics.
pub fn idle_count(db_type: &str, connection: &str) -> usize {
    cache()
        .lock()
        .map(|cache| {
            cache
                .get(&(db_type.to_string(), connection.to_string()))
                .map(Vec::len)
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    /// A distinct database per test: the cache is process-wide, so tests
    /// sharing a connection string would see each other's idle connections.
    fn temp_db(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("etamil_pool_{}.db", name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn a_returned_connection_is_reused() {
        let path = temp_db("reuse");
        let _ = std::fs::remove_file(&path);

        assert_eq!(idle_count("SQLite", &path), 0, "nothing cached yet");

        {
            let mut lease = checkout("SQLite", &path).unwrap();
            lease
                .connection()
                .execute("CREATE TABLE IF NOT EXISTS t (a INTEGER)", &[])
                .unwrap();
            assert_eq!(idle_count("SQLite", &path), 0, "still checked out");
        }

        assert_eq!(idle_count("SQLite", &path), 1, "returned to the cache");

        {
            let _lease = checkout("SQLite", &path).unwrap();
            assert_eq!(idle_count("SQLite", &path), 0, "taken from the cache");
        }
        assert_eq!(idle_count("SQLite", &path), 1);
    }

    /// A function registered for one request must not run, with that request's
    /// program, for the next one to borrow the same connection.
    #[test]
    fn a_registered_function_does_not_follow_the_connection() {
        use crate::db::{Numbers, RowFunction};
        use crate::vm::Value;

        let path = temp_db("function");
        let _ = std::fs::remove_file(&path);
        let function = || -> RowFunction { Box::new(|_: &[Value]| Ok(Value::Null)) };

        {
            let mut lease = checkout("SQLite", &path).unwrap();
            let db = lease.connection();
            db.register_function("veRRu", 0, Numbers::Native, function())
                .unwrap();
            assert!(db.query("SELECT veRRu()", &[]).is_ok());
        }

        // The same connection comes back out of the cache, without the function.
        assert_eq!(idle_count("SQLite", &path), 1);
        let mut lease = checkout("SQLite", &path).unwrap();
        let error = lease.connection().query("SELECT veRRu()", &[]).unwrap_err();
        assert!(error.contains("no such function"), "{error}");
    }

    /// The reason a lease is exclusive. If a handler opens a transaction and
    /// then fails, the connection must not carry that transaction to whoever
    /// borrows it next — their first statement would silently join someone
    /// else's unit of work, and a later COMMIT would publish half of it.
    #[test]
    fn a_leaked_transaction_does_not_follow_the_connection() {
        let path = temp_db("leak");
        let _ = std::fs::remove_file(&path);

        {
            let mut lease = checkout("SQLite", &path).unwrap();
            let db = lease.connection();
            db.execute("CREATE TABLE t (a INTEGER)", &[]).unwrap();
            db.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

            // Opened and deliberately never closed.
            db.execute("BEGIN", &[]).unwrap();
            db.execute("INSERT INTO t VALUES (99)", &[]).unwrap();
        }

        // The same connection comes back out of the cache.
        let mut lease = checkout("SQLite", &path).unwrap();
        let rows = lease.connection().query("SELECT a FROM t", &[]).unwrap();

        // One row: the uncommitted 99 was rolled back on return. Two would
        // mean the next borrower inherited an open transaction.
        assert_eq!(
            rows.len(),
            1,
            "the abandoned INSERT should have rolled back"
        );

        // And a fresh transaction works, which it could not if the old one
        // were still open.
        let db = lease.connection();
        db.execute("BEGIN", &[]).unwrap();
        db.execute("INSERT INTO t VALUES (2)", &[]).unwrap();
        db.execute("COMMIT", &[]).unwrap();
        assert_eq!(db.query("SELECT a FROM t", &[]).unwrap().len(), 2);
    }

    /// Two leases at once must be two connections, or they would share
    /// transaction state.
    #[test]
    fn two_leases_are_two_connections() {
        let path = temp_db("two");
        let _ = std::fs::remove_file(&path);

        let mut first = checkout("SQLite", &path).unwrap();
        first
            .connection()
            .execute("CREATE TABLE IF NOT EXISTS t (a INTEGER)", &[])
            .unwrap();

        let mut second = checkout("SQLite", &path).unwrap();
        second.connection().query("SELECT 1", &[]).unwrap();

        drop(first);
        drop(second);
        assert_eq!(idle_count("SQLite", &path), 2, "both were cached");
    }

    /// close() means close: the connection must not go back to the cache.
    #[test]
    fn closing_a_lease_does_not_cache_it() {
        let path = temp_db("close");
        let _ = std::fs::remove_file(&path);

        let lease = checkout("SQLite", &path).unwrap();
        lease.close().unwrap();

        assert_eq!(idle_count("SQLite", &path), 0);
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn an_in_memory_database_is_recognised_as_private() {
        assert!(is_private(":memory:"));
        assert!(is_private("  :memory:  "));
        assert!(is_private("file::memory:"));
        assert!(is_private("file:scratch?mode=memory"));
    }

    #[test]
    fn a_memory_database_asking_for_a_shared_cache_is_not_private() {
        // Asking for cache=shared is asking to be shared, which is the one
        // case where reusing the connection is what was wanted.
        assert!(!is_private("file:scratch?mode=memory&cache=shared"));
    }

    #[test]
    fn a_file_is_not_private() {
        assert!(!is_private("ledger.db"));
        assert!(!is_private("/var/lib/etamil/ledger.db"));
        assert!(!is_private("postgres://localhost/etamil"));
    }

    #[test]
    fn a_private_connection_is_never_left_in_the_cache() {
        // Two checkouts of :memory: must be two databases, so neither may come
        // from the other's leavings.
        let first = checkout("SQLite", ":memory:");
        assert!(first.is_ok(), "{:?}", first.err());
        drop(first);

        assert_eq!(
            idle_count("SQLite", ":memory:"),
            0,
            "an in-memory database must not be pooled for the next caller"
        );
    }
}
