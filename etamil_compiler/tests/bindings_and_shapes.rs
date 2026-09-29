// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! நிலை bindings, functions as values, record shapes and their methods.
//!
//! The same pipeline as `language_tests.rs` — lex, parse, check, compile, run —
//! because most of what these features promise is a refusal, and a refusal
//! that only one stage makes is one the other stages can be talked out of.

use etamil_compiler::lexer;
use etamil_compiler::parser::Parser;
use etamil_compiler::vm::{BytecodeCompiler, VM};
use rust_decimal::Decimal;

fn run(source: &str) -> Result<VM, String> {
    let tokens = lexer::tokenize(source).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let mut parser = Parser::new(tokens.iter());
    let ast = parser.parse().map_err(|error| error.to_string())?;
    etamil_compiler::check::check(&ast).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let bytecode = BytecodeCompiler::compile_statements(ast);
    let mut vm = VM::new();
    vm.execute(bytecode)?;
    Ok(vm)
}

/// The error a program is refused with, from whichever stage refused it.
fn refused(source: &str) -> String {
    match run(source) {
        Ok(_) => panic!("expected this to be refused:\n{}", source),
        Err(why) => why,
    }
}

fn num(vm: &VM, name: &str) -> Decimal {
    vm.variables
        .get(name)
        .unwrap_or_else(|| panic!("variable '{}' was never assigned", name))
        .to_number()
}

fn text(vm: &VM, name: &str) -> String {
    vm.variables
        .get(name)
        .unwrap_or_else(|| panic!("variable '{}' was never assigned", name))
        .to_string()
}

// --- நிலை: immutable bindings ------------------------------------------------

#[test]
fn a_fixed_binding_reads_like_any_other() {
    let vm = run("நிலை எல்லை = 250000; நிலை எண் வீதம் = 5%; வரி = எல்லை * வீதம்;").unwrap();
    assert_eq!(num(&vm, "வரி"), Decimal::from(12500));
}

#[test]
fn a_fixed_binding_cannot_be_assigned_again() {
    let why = refused("நிலை எல்லை = 250000;\nஎல்லை = 300000;");
    assert!(why.contains("cannot assign twice to 'எல்லை'"), "{}", why);
    // Pointed at the second assignment, not the declaration.
    assert!(why.contains("line 2"), "{}", why);
}

#[test]
fn no_part_of_a_fixed_value_can_change() {
    let why = refused("நிலை அடுக்குகள் = [0, 5%, 20%]; அடுக்குகள்[1] = 10%;");
    assert!(why.contains("cannot change 'அடுக்குகள்[…]'"), "{}", why);

    let why = refused("நிலை கணக்கு = {இருப்பு: 100}; கணக்கு.இருப்பு = 0;");
    assert!(why.contains("cannot change 'கணக்கு.இருப்பு'"), "{}", why);

    // Appending is an assignment too, however the VM carries it out.
    let why = refused("நிலை பட்டியல் = []; பட்டியல் = இணை(பட்டியல், 1);");
    assert!(why.contains("cannot assign twice"), "{}", why);
}

#[test]
fn a_copy_of_a_fixed_value_is_an_ordinary_variable() {
    // Values are copied, so changing the copy cannot reach the original —
    // which is why fixing the name is enough to fix the value.
    let vm = run("நிலை அசல் = [1, 2]; நகல் = அசல்; நகல்[0] = 9;").unwrap();
    assert_eq!(text(&vm, "அசல்"), "[1, 2]");
    assert_eq!(text(&vm, "நகல்"), "[9, 2]");
}

#[test]
fn a_name_is_fixed_once_per_scope() {
    let why = refused("நிலை அ = 1; நிலை அ = 2;");
    assert!(why.contains("already bound with நிலை on line 1"), "{}", why);

    let why = refused("அ = 1; நிலை அ = 2;");
    assert!(
        why.contains("already assigned as an ordinary variable"),
        "{}",
        why
    );
}

#[test]
fn a_fixed_name_cannot_become_a_loop_variable_or_a_query_result() {
    let why = refused("நிலை உருப்படி = 1; ஒவ்வொரு உருப்படி இல் [1, 2] { அச்சு உருப்படி; }");
    assert!(why.contains("cannot assign twice to 'உருப்படி'"), "{}", why);
}

#[test]
fn a_function_cannot_change_an_outer_fixed_name_it_can_only_shadow_it() {
    // Assigning inside a function makes a local, so the global is untouched.
    // That is the VM's rule for every name, and நிலை does not bend it.
    let vm = run("நிலை எல்லை = 10; \
         செயல் மாற்று() { எல்லை = 99; திரும்பு எல்லை; } \
         உள்ளே = மாற்று();")
    .unwrap();
    assert_eq!(num(&vm, "உள்ளே"), Decimal::from(99));
    assert_eq!(num(&vm, "எல்லை"), Decimal::from(10));
}

#[test]
fn a_fixed_parameter_cannot_be_assigned() {
    let why = refused("செயல் வரி(நிலை எண் தொகை) { தொகை = தொகை * 2; திரும்பு தொகை; }");
    assert!(why.contains("cannot assign twice to 'தொகை'"), "{}", why);

    let vm = run("செயல் வரி(நிலை எண் தொகை) எண் { திரும்பு தொகை * 18%; } அ = வரி(1000);").unwrap();
    assert_eq!(num(&vm, "அ"), Decimal::from(180));
}

#[test]
fn nilai_is_still_an_ordinary_name_where_it_is_not_a_binding() {
    // Two examples use நிலை as a variable, and records use it as a field.
    let vm = run(
        "நிலை = 5; நிலை = நிலை + 1; ப = {நிலை: \"திறந்தது\"}; ந = ப.நிலை; \
         செயல் இரு(நிலை) { திரும்பு நிலை * 2; } இ = இரு(4);",
    )
    .unwrap();
    assert_eq!(num(&vm, "நிலை"), Decimal::from(6));
    assert_eq!(text(&vm, "ந"), "திறந்தது");
    assert_eq!(num(&vm, "இ"), Decimal::from(8));
}

#[test]
fn a_fixed_binding_is_written_three_ways() {
    let vm = run("nilY a = 1; _const _b = 2; நிலை இ = 3;").unwrap();
    assert_eq!(
        num(&vm, "a") + num(&vm, "_b") + num(&vm, "இ"),
        Decimal::from(6)
    );
}

#[test]
fn a_fixed_binding_is_held_to_its_declared_type_too() {
    let why = refused("நிலை எண் எல்லை = \"அதிகம்\";");
    assert!(why.contains("is declared எண்"), "{}", why);
}

#[test]
fn every_route_handler_is_its_own_scope() {
    // Two handlers each fixing the same local name are two programs, not one
    // name bound twice. Checked only: the VM cannot run routes.
    let source = "நிலை அடிப்படை = 1; \
         வழி பெறு, \"/அ\" { நிலை விடை = அடிப்படை; பதில் 200, விடை; } \
         வழி பெறு, \"/ஆ\" { நிலை விடை = 2; பதில் 200, விடை; }";
    let tokens = lexer::tokenize(source).unwrap();
    let ast = Parser::new(tokens.iter()).parse().unwrap();
    assert!(etamil_compiler::check::check(&ast).is_ok());
}

// --- functions as values ---------------------------------------------------

#[test]
fn a_named_function_is_a_value() {
    let vm = run("செயல் இரட்டி(x) { திரும்பு x * 2; } ச = இரட்டி; விடை = ச(21);").unwrap();
    assert_eq!(num(&vm, "விடை"), Decimal::from(42));
    assert_eq!(text(&vm, "ச"), "<செயல் இரட்டி>");
}

#[test]
fn a_function_can_be_passed_and_called_by_its_parameter_name() {
    let vm = run("செயல் ஒவ்வொன்றும்(பட்டியல், செயல் மாற்று) { \
             விடை = []; \
             ஒவ்வொரு உருப்படி இல் பட்டியல் { விடை = இணை(விடை, மாற்று(உருப்படி)); } \
             திரும்பு விடை; \
         } \
         செயல் வரியுடன்(தொகை) { திரும்பு தொகை * 118%; } \
         முடிவு = ஒவ்வொன்றும்([100, 250], வரியுடன்);")
    .unwrap();
    assert_eq!(text(&vm, "முடிவு"), "[118, 295]");
}

#[test]
fn a_builtin_is_a_value_too() {
    let vm = run("அளவு = நீளம்; ந = அளவு([1, 2, 3]);").unwrap();
    assert_eq!(num(&vm, "ந"), Decimal::from(3));
}

#[test]
fn a_mistyped_name_is_still_an_undefined_variable() {
    let why = refused("ச = நீளாம்;");
    assert!(why.contains("undefined variable 'நீளாம்'"), "{}", why);
}

#[test]
fn an_anonymous_function_can_be_written_where_a_value_goes() {
    let vm = run("கூட்டு = செயல்(அ, ஆ) { திரும்பு அ + ஆ; }; விடை = கூட்டு(2, 3);").unwrap();
    assert_eq!(num(&vm, "விடை"), Decimal::from(5));
    assert_eq!(text(&vm, "கூட்டு"), "<செயல்>");
}

#[test]
fn an_anonymous_function_captures_the_locals_it_uses_by_value() {
    // வீதம் is a parameter of the enclosing function; the value carries it
    // out after that call has returned.
    let vm = run(
        "செயல் வரி_செய்(வீதம்) { திரும்பு செயல்(தொகை) { திரும்பு தொகை * வீதம்; }; } \
         ஜிஎஸ்டி = வரி_செய்(18%); \
         டிடிஎஸ் = வரி_செய்(10%); \
         அ = ஜிஎஸ்டி(1000); ஆ = டிடிஎஸ்(1000);",
    )
    .unwrap();
    assert_eq!(num(&vm, "அ"), Decimal::from(180));
    assert_eq!(num(&vm, "ஆ"), Decimal::from(100));
}

#[test]
fn a_capture_is_a_copy_taken_when_the_function_value_is_made() {
    let vm = run("செயல் சோதி() { \
             அளவு = 1; \
             பெறு_அளவு = செயல்() { திரும்பு அளவு; }; \
             அளவு = 2; \
             திரும்பு பெறு_அளவு(); \
         } \
         விடை = சோதி();")
    .unwrap();
    assert_eq!(num(&vm, "விடை"), Decimal::from(1));
}

#[test]
fn at_the_top_level_a_function_value_reads_globals_live() {
    // The same rule a named செயல் has always had: a global is read when the
    // function runs, not when it was made.
    let vm = run("அளவு = 1; எடு = செயல்() { திரும்பு அளவு; }; அளவு = 2; விடை = எடு();").unwrap();
    assert_eq!(num(&vm, "விடை"), Decimal::from(2));
}

#[test]
fn nested_functions_capture_through_the_one_between() {
    let vm = run(
        "செயல் முதல்(அ) { திரும்பு செயல்(ஆ) { திரும்பு செயல்(இ) { திரும்பு அ + ஆ + இ; }; }; } \
         விடை = முதல்(1)(10)(100);",
    )
    .unwrap();
    assert_eq!(num(&vm, "விடை"), Decimal::from(111));
}

#[test]
fn a_function_in_an_array_or_a_record_can_be_called() {
    let vm = run(
        "விதிகள் = [செயல்(x) { திரும்பு x + 1; }, செயல்(x) { திரும்பு x * 10; }]; \
         அ = விதிகள்[1](5); \
         அட்டவணை = {கூட்டு: செயல்(x) { திரும்பு x + 100; }}; \
         ஆ = அட்டவணை.கூட்டு(1);",
    )
    .unwrap_or_else(|why| panic!("{}", why));
    assert_eq!(num(&vm, "அ"), Decimal::from(50));
    assert_eq!(num(&vm, "ஆ"), Decimal::from(101));
}

#[test]
fn calling_something_that_is_not_a_function_says_so() {
    let why = refused("அ = [5]; ஆ = அ[0](1);");
    assert!(
        why.contains("this is not a function: it is a number"),
        "{}",
        why
    );
}

#[test]
fn a_function_value_is_held_to_its_arity() {
    let why = refused("ச = செயல்(அ) { திரும்பு அ; }; ச(1, 2);");
    assert!(why.contains("expects 1 argument(s), got 2"), "{}", why);
}

#[test]
fn a_function_parameter_is_held_to_its_declared_type() {
    let why = refused("செயல் செய்(செயல் மாற்று) { திரும்பு மாற்று(1); } விடை = செய்(5);");
    assert!(why.contains("is declared செயல்"), "{}", why);
}

#[test]
fn a_question_mark_inside_a_function_value_returns_from_it() {
    let vm = run(
        "பாதுகாப்பு = செயல்(x) { மதிப்பு_ = x?; திரும்பு சரி(மதிப்பு_ + 1); }; \
         அ = பாதுகாப்பு(சரி(1)); ஆ = பாதுகாப்பு(தவறு(\"இல்லை\"));",
    )
    .unwrap();
    assert_eq!(text(&vm, "அ"), "சரி(2)");
    assert_eq!(text(&vm, "ஆ"), "தவறு(இல்லை)");
}

#[test]
fn a_fixed_binding_can_hold_a_function() {
    let why = refused("நிலை செயல் வரி = செயல்(x) { திரும்பு x; }; வரி = 5;");
    assert!(why.contains("cannot assign twice to 'வரி'"), "{}", why);
}

// --- வடிவம்: declared record shapes -------------------------------------------

const LOAN: &str = "வடிவம் கடன் { எண் அசல், எண் வீதம், சொல் பெயர் } ";

#[test]
fn a_shaped_record_is_built_and_read_like_a_record() {
    let vm = run(&format!(
        "{}க = கடன்{{அசல்: 100000, வீதம்: 9%, பெயர்: \"ராஜா\"}}; வட்டி_ = க.அசல் * க.வீதம்; வ = வகை(க);",
        LOAN
    ))
    .unwrap();
    assert_eq!(num(&vm, "வட்டி_"), Decimal::from(9000));
    assert_eq!(text(&vm, "வ"), "கடன்");
    assert_eq!(text(&vm, "க"), "கடன்{அசல்: 100000, பெயர்: ராஜா, வீதம்: 0.09}");
}

#[test]
fn a_shape_can_be_used_above_the_line_that_declares_it() {
    let vm = run("க = கடன்{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}; \
         வடிவம் கடன் { எண் அசல், எண் வீதம், சொல் பெயர் }")
    .unwrap();
    assert_eq!(text(&vm, "க"), "கடன்{அசல்: 1, பெயர்: அ, வீதம்: 0.01}");
}

#[test]
fn a_mistyped_field_is_refused_before_the_program_runs() {
    // The bug declared shapes exist for: this used to fail only when the line
    // ran, and only if it ran.
    let why = refused(&format!(
        "{}கடன் க = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}}; (பொய்) எனில் {{ அச்சு க.அசால்; }}",
        LOAN
    ));
    assert!(why.contains("'கடன்' has no field 'அசால்'"), "{}", why);
    assert!(why.contains("its fields are: அசல், வீதம், பெயர்"), "{}", why);
}

#[test]
fn a_fixed_binding_knows_its_shape_without_being_told() {
    let why = refused(&format!(
        "{}நிலை க = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}}; அ = க.அசால்;",
        LOAN
    ));
    assert!(why.contains("has no field 'அசால்'"), "{}", why);
}

#[test]
fn a_literal_must_give_every_field_and_no_other() {
    let why = refused(&format!("{}க = கடன்{{அசல்: 1, வீதம்: 1%}};", LOAN));
    assert!(why.contains("missing: பெயர்"), "{}", why);

    let why = refused(&format!(
        "{}க = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\", காலம்: 5}};",
        LOAN
    ));
    assert!(why.contains("has no field 'காலம்'"), "{}", why);
}

#[test]
fn a_field_is_held_to_its_declared_type() {
    let why = refused(&format!(
        "{}க = கடன்{{அசல்: \"நிறைய\", வீதம்: 1%, பெயர்: \"அ\"}};",
        LOAN
    ));
    assert!(why.contains("'கடன்.அசல்' is declared எண்"), "{}", why);
}

#[test]
fn a_value_that_only_exists_at_runtime_is_checked_at_runtime() {
    // உள்ளீடு is not known to the checker; the shape still holds.
    let why = refused(&format!(
        "{}செயல் உரை_தா() {{ திரும்பு \"நிறைய\"; }} க = கடன்{{அசல்: உரை_தா(), வீதம்: 1%, பெயர்: \"அ\"}};",
        LOAN
    ));
    assert!(why.contains("'கடன்.அசல்' is declared எண்"), "{}", why);
    assert!(why.contains("but was given a string"), "{}", why);
}

#[test]
fn a_shaped_record_takes_only_its_own_fields_when_changed() {
    let vm = run(&format!(
        "{}கடன் க = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}}; க.அசல் = 500;",
        LOAN
    ))
    .unwrap();
    assert_eq!(text(&vm, "க"), "கடன்{அசல்: 500, பெயர்: அ, வீதம்: 0.01}");

    let why = refused(&format!(
        "{}கடன் க = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}}; க.காலம் = 5;",
        LOAN
    ));
    assert!(why.contains("has no field 'காலம்'"), "{}", why);

    // A key computed at runtime is checked when it is used.
    let why = refused(&format!(
        "{}க = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}}; விசை_ = \"கா\" & \"லம்\"; க[விசை_] = 5;",
        LOAN
    ));
    assert!(why.contains("has no field 'காலம்'"), "{}", why);
}

#[test]
fn the_rest_of_the_fields_can_come_from_another_record_of_the_shape() {
    let vm = run(&format!(
        "{}அ = கடன்{{அசல்: 1000, வீதம்: 9%, பெயர்: \"ராஜா\"}}; ஆ = கடன்{{அசல்: 2000, ..அ}};",
        LOAN
    ))
    .unwrap();
    assert_eq!(text(&vm, "ஆ"), "கடன்{அசல்: 2000, பெயர்: ராஜா, வீதம்: 0.09}");
    assert_eq!(text(&vm, "அ"), "கடன்{அசல்: 1000, பெயர்: ராஜா, வீதம்: 0.09}");

    let why = refused(&format!("{}அ = {{அசல்: 1}}; ஆ = கடன்{{அசல்: 2, ..அ}};", LOAN));
    assert!(why.contains("'..' needs a கடன்"), "{}", why);
}

#[test]
fn a_plain_record_becomes_a_shape_by_calling_the_shape() {
    let vm = run(&format!(
        "{}பதிவு_ = {{அசல்: 5000, வீதம்: 7%, பெயர்: \"அ\"}}; க = கடன்(பதிவு_); ம = மதிப்பு(க).அசல்; \
         தவறானது = கடன்({{அசல்: 1}}); இ = தவறா(தவறானது);",
        LOAN
    ))
    .unwrap();
    assert_eq!(num(&vm, "ம"), Decimal::from(5000));
    assert_eq!(
        vm.variables.get("இ"),
        Some(&etamil_compiler::vm::Value::Boolean(true))
    );
    assert!(text(&vm, "தவறானது").contains("missing: வீதம், பெயர்"));
}

#[test]
fn a_shaped_record_equals_only_a_record_of_the_same_shape() {
    let vm = run(&format!(
        "{}அ = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}}; ஆ = கடன்{{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}}; \
         ஒன்று = அ == ஆ; வேறு = அ == {{அசல்: 1, வீதம்: 1%, பெயர்: \"அ\"}};",
        LOAN
    ))
    .unwrap();
    assert_eq!(text(&vm, "ஒன்று"), "true");
    assert_eq!(text(&vm, "வேறு"), "false");
}

#[test]
fn a_shape_can_hold_a_shape() {
    let vm = run("வடிவம் நபர் { சொல் பெயர் } \
         வடிவம் கணக்கு_ { நபர் உரிமையாளர், எண் இருப்பு_ } \
         க = கணக்கு_{உரிமையாளர்: நபர்{பெயர்: \"மீனா\"}, இருப்பு_: 10}; ப = க.உரிமையாளர்.பெயர்;")
    .unwrap();
    assert_eq!(text(&vm, "ப"), "மீனா");

    let why = refused(
        "வடிவம் நபர் { சொல் பெயர் } \
         வடிவம் கணக்கு_ { நபர் உரிமையாளர் } \
         க = கணக்கு_{உரிமையாளர்: {பெயர்: \"மீனா\"}};",
    );
    assert!(why.contains("'கணக்கு_.உரிமையாளர்' is declared நபர்"), "{}", why);
}

#[test]
fn a_shape_is_declared_once_at_the_top_level_under_a_name_of_its_own() {
    let why = refused("வடிவம் அ { எண் x } வடிவம் அ { எண் y }");
    assert!(why.contains("'அ' is already a shape"), "{}", why);

    let why = refused("செயல் அ() { திரும்பு 1; } வடிவம் அ { எண் x }");
    assert!(
        why.contains("cannot be both a shape and a function"),
        "{}",
        why
    );

    let why = refused("(மெய்) எனில் { வடிவம் அ { எண் x } }");
    assert!(why.contains("declared at the top level"), "{}", why);

    let why = refused("வடிவம் அ { எண் x, சொல் x }");
    assert!(why.contains("declared twice"), "{}", why);

    let why = refused("எண்_ப x = 5;");
    assert!(why.contains("there is no shape named 'எண்_ப'"), "{}", why);
}

#[test]
fn a_shape_typed_parameter_takes_only_that_shape() {
    let why = refused(&format!(
        "{}செயல் வட்டி_கணி(கடன் க) {{ திரும்பு க.அசல் * க.வீதம்; }} அ = வட்டி_கணி({{அசல்: 1}});",
        LOAN
    ));
    assert!(why.contains("'க' is declared கடன்"), "{}", why);

    let why = refused(&format!(
        "{}செயல் வட்டி_கணி(கடன் க) {{ திரும்பு க.அசால்; }}",
        LOAN
    ));
    assert!(why.contains("has no field 'அசால்'"), "{}", why);
}

#[test]
fn a_block_after_a_loop_head_is_still_a_block() {
    // `ஒவ்வொரு x இல் பட்டியல் {` must not read `பட்டியல் {` as a shaped literal.
    let vm = run("பட்டியல் = [1, 2]; மொத்தம் = 0; ஒவ்வொரு x இல் பட்டியல் { மொத்தம் = மொத்தம் + x; }").unwrap();
    assert_eq!(num(&vm, "மொத்தம்"), Decimal::from(3));
}

#[test]
fn vativam_is_still_an_ordinary_name_where_it_is_not_a_shape() {
    // nUlakam/vativam/AvaNam.qmz has a parameter called வடிவம்.
    let vm = run("செயல் எடு(வடிவம்) { திரும்பு வடிவம்[\"அ\"]; } வ = எடு({அ: 5});").unwrap();
    assert_eq!(num(&vm, "வ"), Decimal::from(5));
}

// --- methods: functions attached to a shape --------------------------------

const ACCOUNT: &str = "வடிவம் கணக்கு_ப { \
       சொல் பெயர், எண் இருப்பு_, \
       செயல் புதிது(பெயர்) கணக்கு_ப { திரும்பு கணக்கு_ப{பெயர்: பெயர், இருப்பு_: 0}; } \
       செயல் வை(இது, எண் தொகை) கணக்கு_ப { திரும்பு கணக்கு_ப{இருப்பு_: இது.இருப்பு_ + தொகை, ..இது}; } \
       செயல் போதுமா(இது, எண் தொகை) ஈர்ம { திரும்பு இது.இருப்பு_ >= தொகை; } \
     } ";

#[test]
fn a_method_is_called_on_a_record_of_its_shape() {
    let vm = run(&format!(
        "{}க = கணக்கு_ப.புதிது(\"மீனா\"); க = க.வை(500); க = க.வை(250); ஆம் = க.போதுமா(700); இல்லை_ = க.போதுமா(800);",
        ACCOUNT
    ))
    .unwrap();
    assert_eq!(text(&vm, "க"), "கணக்கு_ப{இருப்பு_: 750, பெயர்: மீனா}");
    assert_eq!(text(&vm, "ஆம்"), "true");
    assert_eq!(text(&vm, "இல்லை_"), "false");
}

#[test]
fn a_method_can_be_called_on_the_shape_with_the_record_first() {
    // Rust's `Account::deposit(&account, 500)`.
    let vm = run(&format!(
        "{}க = கணக்கு_ப.புதிது(\"அ\"); ஆ = கணக்கு_ப.வை(க, 10);",
        ACCOUNT
    ))
    .unwrap();
    assert_eq!(text(&vm, "ஆ"), "கணக்கு_ப{இருப்பு_: 10, பெயர்: அ}");
}

#[test]
fn a_method_cannot_change_the_record_it_was_called_on() {
    // இது is Rust's `&self`: a method that means to change a record returns
    // a new one.
    let why = refused("வடிவம் அ { எண் x, செயல் கூட்டு(இது) { இது.x = இது.x + 1; } }");
    assert!(why.contains("cannot change 'இது.x'"), "{}", why);

    let why = refused("வடிவம் அ { எண் x, செயல் மாற்று(இது) { இது = 5; } }");
    assert!(why.contains("cannot assign twice to 'இது'"), "{}", why);
}

#[test]
fn a_method_sees_only_its_own_shape_s_fields() {
    let why = refused("வடிவம் அ { எண் x, செயல் எடு(இது) { திரும்பு இது.y; } }");
    assert!(why.contains("'அ' has no field 'y'"), "{}", why);
}

#[test]
fn an_unknown_method_is_refused_before_the_program_runs() {
    let why = refused(&format!(
        "{}கணக்கு_ப க = கணக்கு_ப.புதிது(\"அ\"); க = க.எடு_(5);",
        ACCOUNT
    ));
    assert!(why.contains("has no method or field 'எடு_'"), "{}", why);

    let why = refused(&format!("{}க = கணக்கு_ப.இல்லாதது();", ACCOUNT));
    assert!(
        why.contains("'கணக்கு_ப' has no function 'இல்லாதது'"),
        "{}",
        why
    );
}

#[test]
fn a_method_without_its_record_is_called_on_the_shape_not_on_a_record() {
    let why = refused(&format!(
        "{}கணக்கு_ப க = கணக்கு_ப.புதிது(\"அ\"); ஆ = க.புதிது(\"ஆ\");",
        ACCOUNT
    ));
    assert!(why.contains("takes no இது"), "{}", why);
}

#[test]
fn method_arguments_are_held_to_their_declared_types() {
    let why = refused(&format!(
        "{}கணக்கு_ப க = கணக்கு_ப.புதிது(\"அ\"); க = க.வை(\"நூறு\");",
        ACCOUNT
    ));
    assert!(why.contains("'தொகை' is declared எண்"), "{}", why);
}

#[test]
fn a_method_result_carries_its_declared_type() {
    let why = refused(&format!(
        "{}கணக்கு_ப க = கணக்கு_ப.புதிது(\"அ\"); எண் ந = க.போதுமா(1);",
        ACCOUNT
    ));
    assert!(why.contains("'ந' is declared எண்"), "{}", why);
}

#[test]
fn a_method_is_found_at_runtime_when_the_checker_cannot_see_the_shape() {
    // பெறு_கணக்கு declares no return type, so the checker cannot know what
    // it gives back; the VM finds the method on the record it was given.
    let vm = run(&format!(
        "{}செயல் பெறு_கணக்கு() {{ திரும்பு கணக்கு_ப.புதிது(\"அ\"); }} ஆ = பெறு_கணக்கு().வை(3);",
        ACCOUNT
    ))
    .unwrap();
    assert_eq!(text(&vm, "ஆ"), "கணக்கு_ப{இருப்பு_: 3, பெயர்: அ}");

    let why = refused(&format!(
        "{}செயல் பெறு_கணக்கு() {{ திரும்பு கணக்கு_ப.புதிது(\"அ\"); }} ஆ = பெறு_கணக்கு().எடு_(3);",
        ACCOUNT
    ));
    assert!(
        why.contains("'கணக்கு_ப' has no method or field 'எடு_'"),
        "{}",
        why
    );
}

#[test]
fn a_method_can_call_another_method_and_be_passed_as_a_value() {
    let vm = run("வடிவம் தொகை_ப { எண் மதிப்பு_, \
             செயல் இரட்டி(இது) தொகை_ப { திரும்பு தொகை_ப{மதிப்பு_: இது.மதிப்பு_ * 2}; } \
             செயல் நான்மடங்கு(இது) தொகை_ப { திரும்பு இது.இரட்டி().இரட்டி(); } \
         } \
         அ = தொகை_ப{மதிப்பு_: 5}.நான்மடங்கு(); \
         மாற்றி = செயல்(x) { திரும்பு x.இரட்டி(); }; ஆ = மாற்றி(அ);")
    .unwrap();
    assert_eq!(text(&vm, "அ"), "தொகை_ப{மதிப்பு_: 20}");
    assert_eq!(text(&vm, "ஆ"), "தொகை_ப{மதிப்பு_: 40}");
}

#[test]
fn a_variable_cannot_take_a_shape_s_name() {
    let why = refused("வடிவம் அ { எண் x } அ = 5;");
    assert!(why.contains("is the name of a shape"), "{}", why);
}

#[test]
fn a_method_and_a_field_cannot_share_a_name() {
    let why = refused("வடிவம் அ { எண் x, செயல் x(இது) { திரும்பு 1; } }");
    assert!(
        why.contains("cannot be both a field and a method"),
        "{}",
        why
    );
}

#[test]
fn a_shape_can_hold_a_function_in_a_field() {
    let vm = run("வடிவம் விதி_ப { சொல் பெயர், செயல் கணி }          வ = விதி_ப{பெயர்: \"ஜிஎஸ்டி\", கணி: செயல்(x) { திரும்பு x * 18%; }}; அ = வ.கணி(100);")
    .unwrap();
    assert_eq!(num(&vm, "அ"), Decimal::from(18));
}
