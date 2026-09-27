// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
// Request Handler Module

use std::collections::HashMap;

use crate::http::{HttpRequest, HttpResponse};
use crate::parser::Stmt;
use crate::vm::{Bytecode, VM, Value};

/// Make one request readable from eTamil.
///
/// Every variable a handler can see is set here, in one place. The server and
/// this module each used to bind the request themselves, and the two copies
/// drifted: the path-parameter path bound neither the query string nor the
/// headers, and neither of them ever bound the body at all, so a POST payload
/// was unreachable from the language.
///
/// The names are deliberately plain — `query_params["id"]` reads the same in
/// Tamil or romanized source, because they are data, not keywords.
pub fn bind_request(vm: &mut VM, request: &HttpRequest, path_params: &HashMap<String, String>) {
    vm.variables.insert(
        "request_method".to_string(),
        Value::String(request.method.clone()),
    );
    vm.variables.insert(
        "request_path".to_string(),
        Value::String(request.path.clone()),
    );
    // The body arrives as text. Parsing it — as JSON or anything else — is
    // the language's own job; see nUlakam/vativam/jEcAZ.qmz.
    vm.variables.insert(
        "request_body".to_string(),
        Value::String(request.body.clone()),
    );

    // A multipart request carries form fields and files. The fields become
    // text; the files stay as bytes in the VM and the handler is told what
    // arrived, so it can save the one it wants with பதிவேற்றம்_சேமி. Both are
    // always bound, empty when the request is not multipart, so a handler
    // never has to ask whether the name exists.
    let mut fields: HashMap<String, Value> = HashMap::new();
    let mut files: Vec<Value> = Vec::new();
    vm.uploads.clear();

    if let Some(boundary) = request
        .headers
        .get("content-type")
        .and_then(|value| crate::http::multipart::boundary_of(value))
    {
        for part in crate::http::multipart::parse(&request.body_bytes, &boundary) {
            match part.filename {
                Some(filename) => {
                    let mut described = HashMap::new();
                    described.insert(
                        "குறியீடு".to_string(),
                        Value::Number(rust_decimal::Decimal::from(vm.uploads.len())),
                    );
                    described.insert("பெயர்".to_string(), Value::String(part.name.clone()));
                    described.insert("கோப்புப்_பெயர்".to_string(), Value::String(filename.clone()));
                    described.insert(
                        "வகை".to_string(),
                        Value::String(
                            part.content_type
                                .clone()
                                .unwrap_or_else(|| "application/octet-stream".to_string()),
                        ),
                    );
                    described.insert(
                        "அளவு".to_string(),
                        Value::Number(rust_decimal::Decimal::from(part.data.len())),
                    );
                    files.push(Value::Map(described.into()));

                    vm.uploads.push(crate::vm::Upload {
                        name: part.name,
                        filename,
                        content_type: part
                            .content_type
                            .unwrap_or_else(|| "application/octet-stream".to_string()),
                        data: part.data,
                    });
                }
                // A field is text by definition — it is what someone typed.
                None => {
                    fields.insert(
                        part.name,
                        Value::String(String::from_utf8_lossy(&part.data).into_owned()),
                    );
                }
            }
        }
    }

    vm.variables
        .insert("request_fields".to_string(), Value::Map(fields.into()));
    vm.variables
        .insert("request_files".to_string(), Value::Array(files));

    vm.variables.insert(
        "query_params".to_string(),
        Value::Map(as_value_map(&request.query_params).into()),
    );
    vm.variables.insert(
        "headers".to_string(),
        Value::Map(as_value_map(&request.headers).into()),
    );
    vm.variables.insert(
        "path_params".to_string(),
        Value::Map(as_value_map(path_params).into()),
    );

    // Also as param_<name>, which is how path parameters have always been
    // exposed; programs written against that keep working.
    for (name, value) in path_params {
        vm.variables
            .insert(format!("param_{}", name), Value::String(value.clone()));
    }
}

fn as_value_map(source: &HashMap<String, String>) -> HashMap<String, Value> {
    source
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect()
}

/// Read the response a handler left behind.
///
/// `பதில்` writes `response_status`, `response_body` and `response_headers`
/// into the VM's globals; a handler that sets none of them still answers 200,
/// so a route that only prints is not an error.
pub fn response_from(vm: &VM) -> HttpResponse {
    let status = vm
        .variables
        .get("response_status")
        .and_then(|value| match value {
            Value::Number(n) => rust_decimal::prelude::ToPrimitive::to_u16(n),
            _ => None,
        })
        .unwrap_or(200);

    let body = vm
        .variables
        .get("response_body")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "Handler executed successfully".to_string());

    let mut response = HttpResponse::success(status, body);

    // A handler's own headers override the defaults, which is what lets a
    // route serve HTML or a CSV export instead of the JSON this server
    // otherwise assumes.
    if let Some(Value::Map(fields)) = vm.variables.get("response_headers") {
        for (name, value) in fields {
            response.set_header(name, &value.to_string());
        }
    }

    // பதில்_கோப்பு names a file to send instead of a body. The bytes are read
    // here rather than in the language, because a PDF is not a சரம் and
    // pretending otherwise is how a download arrives corrupted.
    if let Some(path) = vm.variables.get("response_file").map(|v| v.to_string()) {
        match std::fs::read(&path) {
            Ok(raw) => {
                response.bytes = Some(raw);
                // Only a default: a handler that set Content-Type keeps it.
                response
                    .headers
                    .entry("Content-Type".to_string())
                    .and_modify(|existing| {
                        if existing == "application/json" {
                            *existing = "application/octet-stream".to_string();
                        }
                    })
                    .or_insert_with(|| "application/octet-stream".to_string());
            }
            Err(e) => {
                // Saying so beats sending an empty 200 that looks like success.
                response = HttpResponse::internal_error(&format!("cannot read '{}': {}", path, e));
            }
        }
    }

    // Content-Length always describes the body actually being sent, never a
    // value the handler supplied.
    let length = match &response.bytes {
        Some(raw) => raw.len(),
        None => response.body.len(),
    };
    response.set_header("Content-Length", &length.to_string());

    response
}

/// Does a route pattern match a path, treating a `:name` segment as a wildcard?
pub fn path_matches(pattern: &str, path: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|p| !p.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

    if pattern_parts.len() != path_parts.len() {
        return false;
    }

    pattern_parts
        .iter()
        .zip(path_parts.iter())
        .all(|(pattern_part, path_part)| pattern_part.starts_with(':') || pattern_part == path_part)
}

/// The `:name` segments of a pattern, paired with what the path had there.
pub fn extract_path_params(pattern: &str, path: &str) -> HashMap<String, String> {
    let pattern_parts = pattern.split('/').filter(|p| !p.is_empty());
    let path_parts = path.split('/').filter(|p| !p.is_empty());

    pattern_parts
        .zip(path_parts)
        .filter_map(|(pattern_part, path_part)| {
            pattern_part
                .strip_prefix(':')
                .map(|name| (name.to_string(), path_part.to_string()))
        })
        .collect()
}

/// Find the handler for a request and run it.
///
/// Shared by both servers, so the sync and async paths cannot disagree about
/// what a route means. An exact match wins; failing that, a pattern with
/// `:params` is tried, and either way the handler is bound and read back
/// identically.
pub fn dispatch(handlers: &HashMap<String, Bytecode>, request: &HttpRequest) -> HttpResponse {
    let method = request.method.to_uppercase();
    let exact_key = format!("{} {}", method, request.path);

    let matched = match handlers.get(&exact_key) {
        Some(bytecode) => Some((bytecode, HashMap::new())),
        None => handlers.iter().find_map(|(route_key, bytecode)| {
            let (route_method, pattern) = route_key.split_once(' ')?;
            if route_method != method || !path_matches(pattern, &request.path) {
                return None;
            }
            Some((bytecode, extract_path_params(pattern, &request.path)))
        }),
    };

    let (bytecode, path_params) = match matched {
        Some(found) => found,
        None => return HttpResponse::not_found(),
    };

    let mut vm = VM::new();
    bind_request(&mut vm, request, &path_params);

    match vm.execute(bytecode.clone()) {
        Ok(_) => response_from(&vm),
        Err(e) => {
            eprintln!("❌ Handler execution error: {}", e);
            HttpResponse::internal_error(&format!("Handler error: {}", e))
        }
    }
}

/// Run one tick of a scheduled job.
///
/// A fresh VM, like a request — so a job sees no leftover state from its last
/// run, and borrows its database connection from the same pool a handler uses.
///
/// A failure is logged and swallowed. A scheduled job is nobody's request:
/// there is no caller to return an error to, and a nightly reconciliation that
/// throws should not take the server down with it.
pub fn run_scheduled(label: &str, bytecode: &Bytecode) {
    let mut vm = VM::new();
    if let Err(e) = vm.execute(bytecode.clone()) {
        eprintln!("❌ இடைவெளி {} failed: {}", label, e);
    }
}

pub struct RequestHandler;

impl RequestHandler {
    /// Execute a handler for a request
    pub fn execute(request: &HttpRequest, handler_stmts: &[Stmt]) -> Result<HttpResponse, String> {
        let mut vm = VM::new();
        bind_request(&mut vm, request, &HashMap::new());

        // Compile and execute handler
        let bytecode = crate::vm::bytecode::compiler::BytecodeCompiler::compile_statements(
            handler_stmts.to_vec(),
        );

        vm.execute(bytecode)
            .map_err(|e| format!("Handler execution error: {}", e))?;

        Ok(response_from(&vm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, path: &str) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: String::new(),
            body_bytes: Vec::new(),
        }
    }

    fn text_var(vm: &VM, name: &str) -> String {
        vm.variables
            .get(name)
            .unwrap_or_else(|| panic!("'{}' was never bound", name))
            .to_string()
    }

    #[test]
    fn test_handler_execution() {
        let result = RequestHandler::execute(&request("GET", "/api/test"), &[]);
        assert!(result.is_ok());
    }

    // The whole point of the change: a POST body has to be readable from
    // eTamil. It was parsed off the socket and then dropped.
    #[test]
    fn the_body_is_bound_for_the_handler_to_read() {
        let mut req = request("POST", "/api/pativu");
        req.body = r#"{"qokY":1500}"#.to_string();

        let mut vm = VM::new();
        bind_request(&mut vm, &req, &HashMap::new());

        assert_eq!(text_var(&vm, "request_body"), r#"{"qokY":1500}"#);
    }

    // Regression: the path-parameter branch bound neither of these.
    #[test]
    fn path_parameters_do_not_cost_the_query_string_or_headers() {
        let mut req = request("GET", "/api/kaNakku/1000");
        req.query_params
            .insert("from".to_string(), "2026-04-01".to_string());
        req.headers
            .insert("authorization".to_string(), "Bearer t".to_string());

        let mut params = HashMap::new();
        params.insert("id".to_string(), "1000".to_string());

        let mut vm = VM::new();
        bind_request(&mut vm, &req, &params);

        assert_eq!(text_var(&vm, "param_id"), "1000");
        match vm.variables.get("path_params") {
            Some(Value::Map(fields)) => {
                assert_eq!(
                    fields.get("id").map(|v| v.to_string()),
                    Some("1000".to_string())
                );
            }
            other => panic!("path_params should be a record, got {:?}", other),
        }
        match vm.variables.get("query_params") {
            Some(Value::Map(fields)) => {
                assert_eq!(
                    fields.get("from").map(|v| v.to_string()),
                    Some("2026-04-01".to_string())
                );
            }
            other => panic!("query_params should be a record, got {:?}", other),
        }
        match vm.variables.get("headers") {
            Some(Value::Map(fields)) => {
                assert_eq!(
                    fields.get("authorization").map(|v| v.to_string()),
                    Some("Bearer t".to_string())
                );
            }
            other => panic!("headers should be a record, got {:?}", other),
        }
    }

    #[test]
    fn a_handler_that_sets_nothing_still_answers_200() {
        let vm = VM::new();
        assert_eq!(response_from(&vm).status_code, 200);
    }
}
