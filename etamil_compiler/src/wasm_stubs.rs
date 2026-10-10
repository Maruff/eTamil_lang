//! Browser stand-ins for the modules that need a socket.
//!
//! `db`, `redis` and `http` are real modules on a native build and absent on
//! wasm. The interpreter holds their types in its own struct fields
//! (`cache: Option<redis::Connection>`, a `db::pool::Lease` per connection) and
//! calls into them from match arms all through `execute`. Gating every one of
//! those would put roughly forty-five `#[cfg]` attributes inside a 2,000-line
//! file.
//!
//! So instead of removing the modules, this re-creates just the surface the
//! interpreter actually touches -- sixteen items -- as types that compile and
//! functions that fail. The interpreter needs no change at all, and a program
//! that asks for a database in the browser gets a sentence explaining why it
//! cannot have one rather than a compile error nobody sees.
//!
//! `net.rs` already did exactly this for its own HTTP client, under
//! `#[cfg(not(feature = "http-client"))]`. This is the same idea, one level up.
//!
//! When the browser VM grows real storage -- IndexedDB behind `db`, say -- these
//! stop being stubs and become implementations. Nothing else moves.

/// Everything here is unreachable by construction: the only constructors return
/// `Err`. Kept type-accurate rather than deleted so the interpreter compiles
/// against one shape on both targets.
macro_rules! unavailable {
    ($what:literal, $english:literal) => {
        Err(format!(concat!(
            $what,
            " உலாவியில் கிடைக்காது  (",
            $english,
            " is not available in the browser)"
        )))
    };
}

pub mod db {
    use crate::vm::Value;

    /// Mirrors the native trait, including its `Send` bound.
    pub trait Database: Send {
        fn execute(&mut self, sql: &str, params: &[Value]) -> Result<i64, String>;
        fn query(&mut self, sql: &str, params: &[Value]) -> Result<Vec<Value>, String>;
        fn close(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn register_function(
            &mut self,
            _name: &str,
            _arity: i32,
            _numbers: Numbers,
            _function: RowFunction,
        ) -> Result<(), String> {
            unavailable!("தரவுதளம்", "a database")
        }
        fn unregister_functions(&mut self) {}
    }

    /// Mirrors the native types, so the interpreter's paths resolve unchanged.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Numbers {
        ExactText,
        Native,
    }

    pub type RowFunction = Box<dyn FnMut(&[Value]) -> Result<Value, String> + Send + 'static>;

    pub mod function {
        use super::RowFunction;
        use crate::vm::value::FunctionValue;
        use crate::vm::{Bytecode, Value};

        /// Never reached: `register_function` above refuses first. It has to exist
        /// because the interpreter names it.
        pub fn row_function(
            _program: &Bytecode,
            _function: FunctionValue,
            _arity: usize,
        ) -> RowFunction {
            Box::new(|_: &[Value]| unavailable!("தரவுதளம்", "a database"))
        }
    }

    /// Native `params_from` converts an eTamil array into bind parameters. There
    /// is nothing to bind to here, but it is called before the query is sent, so
    /// it has to exist and it may as well be honest.
    pub fn params_from(_value: &Value) -> Result<Vec<Value>, String> {
        unavailable!("தரவுதளம்", "a database")
    }

    pub mod pool {
        use super::Database;

        /// Holds a connection natively. Unconstructible here, because
        /// `checkout` is the only way to make one and it always fails.
        pub struct Lease {
            #[allow(dead_code)]
            handle: Box<dyn Database>,
        }

        impl std::fmt::Debug for Lease {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Lease")
            }
        }

        impl Lease {
            /// Named to match `db::pool::Lease` — the interpreter calls this
            /// on whichever of the two it was compiled against.
            pub fn connection(&mut self) -> &mut dyn Database {
                self.handle.as_mut()
            }

            pub fn close(self) -> Result<(), String> {
                Ok(())
            }
        }

        pub fn checkout(_db_type: &str, _connection: &str) -> Result<Lease, String> {
            unavailable!("தரவுதளம்", "a database")
        }
    }
}

pub mod redis {
    use crate::vm::Value;

    /// Same shape as the native reply, so the interpreter's match arms over it
    /// still compile.
    #[derive(Debug)]
    pub enum Reply {
        Simple(String),
        Error(String),
        Integer(i64),
        Bulk(String),
        Nil,
        Array(Vec<Reply>),
    }

    impl Reply {
        pub fn to_value(&self) -> Value {
            // Unreachable: no Connection can exist to produce a Reply.
            Value::String(String::new())
        }
    }

    #[derive(Debug)]
    pub struct Connection {
        address: String,
    }

    impl Connection {
        pub fn open(_address: &str) -> Result<Self, String> {
            unavailable!("ரெடிஸ்", "Redis")
        }

        pub fn address(&self) -> &str {
            &self.address
        }

        pub fn command(&mut self, _command: &str, _arguments: &[String]) -> Result<Reply, String> {
            unavailable!("ரெடிஸ்", "Redis")
        }
    }
}

pub mod http {
    /// Password hashing and token signing. Both are pure computation and could
    /// in principle run here, but they live in `http::auth`, which pulls the
    /// server with it. A playground has no users to authenticate.
    pub mod auth {
        pub fn hash_password(_password: &str) -> Result<String, String> {
            unavailable!("கடவுச்சொல் உறுதி", "authentication")
        }

        pub fn verify_password(_password: &str, _password_hash: &str) -> Result<bool, String> {
            unavailable!("கடவுச்சொல் உறுதி", "authentication")
        }

        pub fn issue_token(_payload_json: &str, _ttl_seconds: i64) -> Result<String, String> {
            unavailable!("குறிதாங்கி", "token issuing")
        }

        pub fn read_token(_token: &str) -> Result<String, String> {
            unavailable!("குறிதாங்கி", "token reading")
        }

        pub fn token_header(_token: &str) -> Result<(String, String), String> {
            unavailable!("குறிதாங்கி", "token reading")
        }

        pub fn verify_rsa_token(
            _token: &str,
            _modulus: &str,
            _exponent: &str,
            _issuer: &str,
            _audience: &str,
        ) -> Result<String, String> {
            unavailable!("குறிதாங்கி", "token verification")
        }
    }
}
