// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! An expression written back as eTamil, for the one place a board has to say
//! where something happened: a report that a number was rounded or overflowed.
//!
//! The AST carries no position for an expression, so a report names the
//! operation by its text — `அளவீடு / 8` — together with the nearest line the
//! AST does know (an assignment's, or the function's). Read on a serial
//! monitor, the text is what finds the place in the source.

use crate::parser::Expr;

/// The expression as an author would have written it. Nested operations are
/// bracketed, so precedence never has to be inferred from the report.
pub fn text(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.normalize().to_string(),
        Expr::Boolean(true) => "மெய்".to_string(),
        Expr::Boolean(false) => "பொய்".to_string(),
        Expr::Null => "இன்மை".to_string(),
        Expr::String(s) => format!("\"{}\"", s),
        Expr::Variable(name) => name.clone(),
        Expr::BinaryOp { op, left, right } => {
            if op == "-" && matches!(left.as_ref(), Expr::Number(n) if n.is_zero()) {
                return format!("-{}", operand(right));
            }
            format!("{} {} {}", operand(left), op, operand(right))
        }
        Expr::Comparison { op, left, right } => format!("{} {} {}", operand(left), op, operand(right)),
        Expr::Logical { op, left, right } => {
            let word = if op == "&&" { "மற்றும்" } else { "அல்லது" };
            format!("{} {} {}", operand(left), word, operand(right))
        }
        Expr::Not(inner) => format!("இல்லை {}", operand(inner)),
        Expr::Concat { left, right } => format!("{} & {}", text(left), text(right)),
        Expr::Call { name, args } => {
            let args: Vec<String> = args.iter().map(text).collect();
            format!("{}({})", name, args.join(", "))
        }
        Expr::Index { base, index } => format!("{}[{}]", operand(base), text(index)),
        Expr::ArrayLiteral(items) => {
            let items: Vec<String> = items.iter().map(text).collect();
            format!("[{}]", items.join(", "))
        }
        _ => "…".to_string(),
    }
}

fn operand(expr: &Expr) -> String {
    match expr {
        Expr::BinaryOp { .. } | Expr::Comparison { .. } | Expr::Logical { .. } | Expr::Concat { .. } => {
            format!("({})", text(expr))
        }
        _ => text(expr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr(source: &str) -> Expr {
        let program = crate::module::load_source(&format!("அ = {};", source), std::path::Path::new("."))
            .expect("parses");
        match program.into_iter().next() {
            Some(crate::parser::Stmt::Assign { value, .. }) => value,
            other => panic!("not an assignment: {:?}", other),
        }
    }

    #[test]
    fn operations_read_back_as_written() {
        assert_eq!(text(&expr("அளவீடு / 8")), "அளவீடு / 8");
        assert_eq!(text(&expr("(அ + 1) * 2.5")), "(அ + 1) * 2.5");
        assert_eq!(text(&expr("-அ")), "-அ");
        assert_eq!(text(&expr("f(அ, 3) > 2 மற்றும் இல்லை ஆ")), "(f(அ, 3) > 2) மற்றும் இல்லை ஆ");
    }
}
