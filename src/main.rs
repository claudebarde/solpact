use serde::Deserialize;
use solang_parser::{
    parse,
    pt::{ContractPart, Expression, SourceUnitPart, Statement},
};
use std::collections::HashMap;
use std::{env, fs, path::Path};
use toml;
mod compact;
use compact::CompactType;
use std::io::Write;

use crate::compact::sol_to_compact_type;

const WITNESSES_CONTRACT: &str = "Witnesses";

#[derive(Debug, Deserialize)]
enum ValueInScope {
    Variable(CompactType),
    Function(FuncSignature),
}

#[derive(Debug, Deserialize)]

struct Scope {
    values: HashMap<String, ValueInScope>,
    user_defined_types: HashMap<String, CompactType>,
}
impl Scope {
    fn new() -> Self {
        return Scope {
            values: HashMap::new(),
            user_defined_types: HashMap::new(),
        };
    }
}

#[derive(Debug, Deserialize)]
struct SolpactConfig {
    compact: CompactSection,
}

#[derive(Debug, Deserialize)]
struct CompactSection {
    default_language_version: String,
}

struct Formatter {
    out: String,
    indent: usize,
}

impl Formatter {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
        }
    }
    fn indent(&mut self) {
        self.indent += 1;
    }
    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn reset_line(&mut self) {
        // reinitialize output line
        self.out = String::new()
    }

    fn line(&mut self, (s, do_not_print): (String, bool)) {
        // println!("formatter line: {:#?}", s);
        self.reset_line();

        if do_not_print {
            return;
        }

        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(&s);
        self.out.push_str(";");
        //FIXME: doesn't print line return if it is a ledger definition
        // there should be a better way to do it
        if !&s.contains("ledger ") {
            self.out.push('\n');
        }
    }

    fn print(&mut self) -> String {
        self.out.clone()
    }
}

fn read_project_config() -> Option<SolpactConfig> {
    let path = Path::new("solpact.toml");

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

#[derive(Debug, Deserialize)]
struct FuncSignature {
    name: String,
    is_exported: bool,
    params: Vec<String>,
    return_type: String,
}
fn parse_func_signature(
    func_def: &solang_parser::pt::FunctionDefinition,
    scope: &mut Scope,
) -> FuncSignature {
    let func_name = match &func_def.name {
        Some(name) => name.name.clone(),
        None if &func_def.ty == &solang_parser::pt::FunctionTy::Constructor => {
            "constructor".to_string()
        }
        None => panic!("FunctionDefinition without a name found: {:#?}", func_def),
    };
    // looks for visibility attributes in attrs vector to define if the function is exported
    let mut is_exported = false;
    for attr in &func_def.attributes {
        match attr {
            solang_parser::pt::FunctionAttribute::Visibility(vis) => {
                if vis.as_str() == "public" {
                    is_exported = true;
                }
            }
            _ => (),
        }
    }
    // function params
    let mut params: Vec<String> = Vec::new();
    if !&func_def.params.is_empty() {
        for param in &func_def.params {
            match &param.1 {
                None => panic!("Unnamed parameter found!"),
                Some(param) => match &param.ty {
                    solang_parser::pt::Expression::Type(_, param_type) => {
                        let sol_type = param_type.to_string();
                        match compact::sol_to_compact_type(&sol_type) {
                            Some(compact_type) => match &param.name {
                                None => {
                                    panic!("Parameter of type '{}' is missing a name!", sol_type)
                                }
                                Some(param_name) => {
                                    scope.values.insert(
                                        param_name.to_string(),
                                        ValueInScope::Variable(compact_type.clone()),
                                    );
                                    params.push(format!(
                                        "{}: {}",
                                        param_name,
                                        compact_type.to_string()
                                    ))
                                }
                            },
                            None => {
                                panic!("Unsupported parameter type '{}' found!", sol_type)
                            }
                        };
                        // match compact::sol_to_compact_type(&sol_type) {
                        //     Some(compact_type) => match &param.name {
                        //         None => {
                        //             panic!("Parameter of type '{}' is missing a name!", sol_type)
                        //         }
                        //         Some(param_name) => {
                        //             scope.insert(
                        //                 param_name.to_string(),
                        //                 ValueInScope::Variable(
                        //                     CompactType::from_string(&compact_type).unwrap(),
                        //                 ),
                        //             );
                        //             params.push(format!("{}: {}", param_name, compact_type))
                        //         }
                        //     },
                        //     None => {
                        //         panic!("Unsupported parameter type '{}' found!", sol_type)
                        //     }
                        // }
                    }
                    _ => panic!("Unsupported parameter type expression!"),
                },
            }
        }
    }
    // function return type
    let return_type = if func_def.returns.is_empty() {
        "[]".to_string()
    } else {
        // maps over the return types and collects their string representations in a tuple for Compact
        let types: Vec<String> = func_def
            .returns
            .iter()
            .map(|ret| match &ret.1 {
                None => todo!("Handle unnamed return types"),
                Some(param) => match &param.ty {
                    solang_parser::pt::Expression::Type(_, ret_type) => {
                        let sol_type = ret_type.to_string();
                        match compact::sol_to_compact_type(&sol_type) {
                            Some(compact_type) => compact_type.to_string(),
                            None => panic!("Unsupported return type '{}' found!", sol_type),
                        }
                    }
                    _ => panic!("Unsupported return type expression!"),
                },
            })
            .collect();
        if types.len() == 1 {
            types[0].clone()
        } else {
            format!("[{}]", types.join(", "))
        }
    };

    return FuncSignature {
        name: func_name,
        is_exported,
        params,
        return_type,
    };
}

fn get_sol_type(expr_to_type: &Expression, scope: &mut Scope) -> Result<CompactType, String> {
    match expr_to_type {
        Expression::Variable(id) => {
            // looks for the variable in the scope
            match scope.values.get(&id.name) {
                Some(ty) => match ty {
                    ValueInScope::Variable(var) => Ok(var.clone()),
                    ValueInScope::Function(_) => Err(format!(
                        "Expected variable type for '{}', found function instead!",
                        id.name
                    )),
                },
                None => Err(format!("Variable '{}' not found in scope!", id.name)),
            }
        }
        Expression::Type(_, sol_type) => {
            let sol_type_str = sol_type.to_string();
            match compact::sol_to_compact_type(&sol_type_str) {
                Some(compact_type) => Ok(compact_type),
                None => Err(format!("Unsupported type '{}' found!", sol_type_str)),
            }
        }
        Expression::FunctionCall(_, expr, params) => match get_sol_type(expr, scope) {
            Ok(sol_type) => Ok(sol_type),
            Err(e) => {
                if e == "disclose".to_string() {
                    if params.len() != 1 {
                        return Err(format!(
                            "Compact.disclose expects exactly one parameter, found {}",
                            params.len()
                        ));
                    }
                    return get_sol_type(&params[0], scope);
                } else {
                    Err(e)
                }
            }
        },
        Expression::MemberAccess(_, expr, id) => {
            // TODO: supports other member accesses outside of Compact.disclose
            if expr.to_string() == "Compact" && id.name == "disclose" {
                // FIXME: this is a hacky way to handle Compact.disclose, maybe there's a better way
                return Err("disclose".to_string());
            } else {
                Err(format!(
                    "MemberAccess '{:#?}' cannot be converted to a type",
                    expr_to_type
                ))
            }
        }
        _ => Err(format!(
            "Expression '{:#?}' cannot be converted to a type",
            expr_to_type
        )),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Expect: cargo run -- <input.sol> <output.compact>
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "solpact".to_string());

    let input_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: {program} <input.sol> <output.compact>");
            std::process::exit(2);
        }
    };

    let output_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: {program} <input.sol> <output.compact>");
            std::process::exit(2);
        }
    };

    // Read Solidity source
    let source = fs::read_to_string(&input_path)
        .map_err(|e| format!("Failed to read input file '{}': {e}", input_path))?;

    // Parse Solidity into AST
    // The 2nd arg is a file number (use 0 for single-file POC)
    let (ast, comments) = match parse(&source, 0) {
        Ok(ok) => ok,
        Err(errors) => {
            eprintln!("Parse failed with {} error(s):", errors.len());
            for e in errors {
                eprintln!("  - {:#?}", e);
            }
            std::process::exit(1);
        }
    };

    // transpiles Solidity AST to Compact code
    let mut compact_output: Vec<String> =
        vec![String::from("// auto-generated code from Solidity source")];

    // checks if one of the comments is a Compact language version
    let mut has_language_version = false;
    for comment in comments {
        // remove leading slashes and whitespace
        let trimmed_value = comment.value().trim_start_matches('/').trim();
        if trimmed_value.starts_with("language_version") {
            // TODO: verify that the format for the language version is correct
            compact_output.push(format!("pragma {};\n", trimmed_value));
            has_language_version = true;
            break;
        }
    }
    // if no language version is found in comments, check project config
    if !has_language_version {
        if let Some(config) = read_project_config() {
            compact_output.push(format!(
                "pragma language_version {};\n",
                config.compact.default_language_version
            ));
        }
    }

    let mut formatter = Formatter::new();

    // visits the AST and generates Compact code
    let mut top_level = true;
    let mut global_scope = Scope::new();
    for part in &ast.0 {
        match part {
            SourceUnitPart::ContractDefinition(def) => {
                match &def.name {
                    Some(def_name) => {
                        if def_name.to_string() == WITNESSES_CONTRACT {
                            if def.parts.is_empty() {
                                compact_output
                                    .push(String::from("// No witness found in the contract\n"));
                                break;
                            }
                            for part in &def.parts {
                                // only the function definitions are relevant here
                                match part {
                                    ContractPart::FunctionDefinition(func_def) => {
                                        let func_signature =
                                            parse_func_signature(func_def, &mut global_scope);
                                        compact_output.push(format!(
                                            "witness {}(): {};\n",
                                            func_signature.name, func_signature.return_type
                                        ));
                                    }
                                    _ => (),
                                }
                            }
                        } else {
                            // import Compact standard library
                            compact_output.push(String::from("import CompactStandardLibrary;\n"));
                            // visit each part of the contract
                            for part in &def.parts {
                                visit(
                                    part,
                                    top_level,
                                    &mut compact_output,
                                    &mut formatter,
                                    &mut global_scope,
                                );
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }

    // Ensure output directory exists if needed
    if let Some(parent) = Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create output dir '{:?}': {e}", parent))?;
        }
    }

    // Write Compact file
    let output = compact_output.join("\n");
    fs::write(&output_path, output)
        .map_err(|e| format!("Failed to write output file '{}': {e}", output_path))?;

    println!("Wrote Compact output to {output_path}");

    // Compile the contract using the Compact compiler
    use std::process::Command;

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "echo hello"])
            .output()
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("compact --version")
            .output()
            .expect("failed to execute process")
    };

    match String::from_utf8(output.stdout) {
        Ok(compact_version) => {
            println!("{}", compact_version);
            Ok(())
        }
        Err(_) => {
            println!("The Compact compiler is not installed!");
            Ok(())
        }
    }
}

fn visit(
    part: &ContractPart,
    top_level: bool,
    compact_output: &mut Vec<String>,
    formatter: &mut Formatter,
    scope: &mut Scope,
) {
    println!("Visiting contract part: {:#?}", part);

    match part {
        ContractPart::VariableDefinition(var_def) => {
            match &var_def.name {
                None => panic!("VariableDefinition without a name found!"),
                Some(var_name) => {
                    let var_type = match &var_def.ty {
                        Expression::MemberAccess(_, expr, id) => {
                            match compact::format_sol_to_compact_type(vec![
                                expr.to_string(),
                                id.name.clone(),
                            ]) {
                                Some(compact_type) => compact_type,
                                None => panic!(
                                    "Unsupported variable type '{}.{}' found!",
                                    expr.to_string(),
                                    id.name
                                ),
                            }
                        }
                        Expression::Variable(id) => {
                            if id.name == WITNESSES_CONTRACT {
                                // Witness type declaration are only for Solidity compatibility
                                CompactType::Void
                            } else {
                                match sol_to_compact_type(&id.name) {
                                    Some(compact_type) => compact_type,
                                    None => {
                                        // checks if the type is not user defined
                                        match scope.user_defined_types.get(&id.name) {
                                            Some(ty) => ty.clone(),
                                            None => {
                                                // otherwise panics
                                                panic!(
                                                    "Unsupported variable type '{}' found!",
                                                    id.name
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Expression::Type(_, sol_type) => {
                            let sol_type_str = sol_type.to_string();
                            match compact::sol_to_compact_type(&sol_type_str) {
                                Some(compact_type) => compact_type,
                                None => {
                                    panic!("Unsupported variable type '{}' found!", sol_type_str)
                                }
                            }
                        }
                        _ => panic!("Unsupported variable type expression: {:#?}", var_def.ty),
                    };
                    // looks for visibility attributes in attrs vector to define if the variable is exported
                    let mut is_exported = false;
                    for attr in &var_def.attrs {
                        match attr {
                            solang_parser::pt::VariableAttribute::Visibility(vis) => {
                                if vis.as_str() == "public" {
                                    is_exported = true;
                                }
                            }
                            _ => (),
                        }
                    }
                    if var_type != CompactType::Void {
                        // if at top level, the variable is a ledger variable
                        if top_level {
                            formatter.line((
                                format!(
                                    "{}ledger {}: {}",
                                    if is_exported { "export " } else { "" },
                                    var_name,
                                    var_type.to_string()
                                ),
                                false,
                            ));
                            compact_output.push(formatter.print())
                        } else {
                            formatter.line((
                                format!("let {}: {};\n", var_name, var_type.to_string()),
                                false,
                            ));
                            compact_output.push(formatter.print())
                        }
                    }
                }
            }
        }
        ContractPart::FunctionDefinition(func_def) => {
            let mut scope = Scope::new();
            let func_signature = parse_func_signature(func_def, &mut scope);
            // function body
            let func_body: String = match &func_def.body {
                Some(body) => visit_statement(body, formatter, &mut scope),
                None => String::from(""),
            };
            // function signature
            let func_signature = if func_signature.name == "constructor" {
                format!("\nconstructor({}) {{", func_signature.params.join(", "))
            } else {
                format!(
                    "{}circuit {}({}): {} {{",
                    if func_signature.is_exported {
                        "export "
                    } else {
                        ""
                    },
                    func_signature.name,
                    func_signature.params.join(", "),
                    func_signature.return_type
                )
            };
            compact_output.push(func_signature);
            // function body (TODO: transpile statements)
            compact_output.push(func_body);
            compact_output.push(String::from("}\n"));
            formatter.dedent();
        }
        ContractPart::EnumDefinition(enum_def) => match &enum_def.name {
            None => panic!("EnumDefinition without a name found!"),
            Some(enum_name) => {
                let mut variants: Vec<String> = Vec::new();
                for variant in &enum_def.values {
                    match variant {
                        None => panic!("Enum variant without a name found!"),
                        Some(variant) => variants.push(variant.name.clone()),
                    }
                }
                // saves enum definition in scope
                let _ = scope.user_defined_types.insert(
                    enum_name.to_string(),
                    CompactType::Enum((enum_name.to_string(), variants.clone())),
                );
                // enum definitions are exported by default
                compact_output.push(format!(
                    "export enum {} {{ {} }}\n",
                    enum_name,
                    variants.join(", ")
                ));
            }
        },
        ContractPart::Using(_) => (),
        _ => panic!("Unsupported contract part found: {:#?}", part),
    }
}

fn visit_statement(part: &Statement, formatter: &mut Formatter, scope: &mut Scope) -> String {
    match part {
        Statement::Block { statements, .. } => {
            // update the formatter
            formatter.indent();
            // checks if statements vector is empty
            if statements.is_empty() {
                return String::from("");
            } else {
                return statements
                    .iter()
                    .map(|stmt| visit_statement(stmt, formatter, scope))
                    .collect::<Vec<String>>()
                    .join("")
                    .trim_end()
                    .to_string();
            }
        }
        Statement::Expression(_, expr) => {
            let result = visit_expression(expr, formatter, scope);
            formatter.line(result);
            return formatter.print();
        }
        Statement::Return(_, expr_opt) => {
            if let Some(expr) = expr_opt {
                let result = visit_expression(expr, formatter, scope);
                formatter.line((format!("return {}", result.0), result.1));
                return formatter.print();
            } else {
                formatter.line((String::from("return"), false));
                return formatter.print();
            }
        }
        _ => panic!("Unsupported statement found: {:#?}", part),
    }
}

fn find_some_none_param_type(param_expr: &Expression, scope: &mut Scope) -> String {
    println!("Handling 'some'/'none' with param: {:#?}", param_expr);
    match param_expr {
        Expression::Variable(id) => match scope.values.get(&id.name) {
            Some(val_in_scope) => match val_in_scope {
                ValueInScope::Variable(compact_type) => compact_type.to_string(),
                _ => panic!(
                    "Expected variable type for '{}', found function instead!",
                    &id.name
                ),
            },
            None => String::from("__unknown__"),
        },
        Expression::FunctionCall(_, expr, params) => {
            if expr.to_string() == "Compact.disclose" {
                find_some_none_param_type(&params[0], scope)
            } else {
                // TODO: supports other function calls
                String::from("__unknown__")
            }
        }
        _ => String::from("__unknown__"),
    }
}

fn visit_expression(
    expr: &Expression,
    formatter: &mut Formatter,
    scope: &mut Scope,
) -> (String, bool) {
    match expr {
        Expression::ArrayLiteral(_, elements) => {
            let element_strs: Vec<(String, bool)> = elements
                .iter()
                .map(|element| visit_expression(element, formatter, scope))
                .collect();
            return (
                format!(
                    "[{}]",
                    element_strs
                        .iter()
                        .map(|(s, _)| s.clone())
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
                element_strs.iter().any(|(_, b)| *b),
            );
        }
        Expression::Assign(_, left_expr, right_expr) => {
            let left = visit_expression(left_expr, formatter, scope);
            let right = visit_expression(right_expr, formatter, scope);
            return (format!("{} = {}", left.0, right.0), left.1 || right.1);
        }
        Expression::Equal(_, left_expr, right_expr) => {
            let left = visit_expression(left_expr, formatter, scope);
            let right = visit_expression(right_expr, formatter, scope);
            return (format!("{} == {}", left.0, right.0), left.1 || right.1);
        }
        Expression::FunctionCall(_, name, params) => {
            // parses the function name
            let name = visit_expression(name, formatter, scope);
            // parses the function params
            let param_strs: Vec<(String, bool)> = params
                .iter()
                .map(|param| visit_expression(param, formatter, scope))
                .collect();

            // pad function special handling
            if name.0 == "pad32" && param_strs.len() == 1 {
                // special handling for pad32 function
                return (
                    compact::compact_pad(&name.0, &param_strs[0].0),
                    param_strs[0].1,
                );
            }

            // translates "require" to "assert"
            if name.0 == "require" && param_strs.len() == 2 {
                return (
                    format!(
                        "assert({})",
                        param_strs
                            .iter()
                            .map(|(s, _)| s.clone())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                    param_strs.iter().any(|(_, b)| *b),
                );
            }

            // "some" and "none" require typing for the paramater
            if (name.0 == "some" || name.0 == "none") && param_strs.len() == 1 {
                let param_name = &param_strs[0].0;
                // finds the type of the parameter from the scope
                let param_type = find_some_none_param_type(&params[0], scope);
                return (
                    format!("{}<{}>({})", name.0, param_type, param_name),
                    param_strs[0].1,
                );
            }

            if !param_strs.is_empty() {
                if name.0 == "ownPublicKey" {
                    return (format!("{}()", name.0), name.1);
                }

                return (
                    format!(
                        "{}({})",
                        name.0,
                        param_strs
                            .iter()
                            .map(|(s, _)| s.clone())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                    name.1,
                );
            } else {
                return (format!("{}()", name.0), name.1);
            }
        }
        Expression::MemberAccess(_, expr, identifier) => {
            let (expr_object, do_not_print) = visit_expression(expr, formatter, scope);
            if expr_object == String::from("CSL") {
                match compact::csl_member_access(&identifier.name) {
                    Some(access_str) => (access_str, do_not_print),
                    None => panic!(
                        "Unsupported CSL member access found: CSL.{}",
                        identifier.name
                    ),
                }
            } else if expr_object == String::from("Compact") {
                match compact::compact_member_access(&identifier.name) {
                    Some(access_str) => (access_str, do_not_print),
                    None => panic!(
                        "Unsupported Compact member access found: Compact.{}",
                        identifier.name
                    ),
                }
            } else if expr_object == String::from("Utils") {
                (format!("{}", identifier.name), do_not_print)
            } else if expr_object == String::from("witnesses") {
                (format!("{}", identifier.name), do_not_print)
            } else {
                (format!("{}.{}", expr_object, identifier.name), do_not_print)
            }
        }
        Expression::New(_, expr) => match expr.as_ref() {
            Expression::FunctionCall(_, name, _) => {
                if name.to_string() == WITNESSES_CONTRACT {
                    return (String::from("no-new-keyword"), true);
                } else {
                    panic!("Unsupported 'new' expression found: {:#?}", expr);
                }
            }
            _ => panic!("Unsupported 'new' expression found: {:#?}", expr),
        },

        Expression::NumberLiteral(_, number, _, _) => (number.to_string(), false),
        Expression::StringLiteral(strings) => {
            let combined_string: String = strings.iter().map(|s| s.string.clone()).collect();
            (format!("\"{}\"", combined_string), false)
        }
        Expression::Type(_, ty) => match &ty {
            solang_parser::pt::Type::Bytes(size) => (format!("bytes{}", size), true),
            solang_parser::pt::Type::String => (String::from("Opaque<\"string\">"), true),
            _ => panic!("Unsupported type expression found: {:#?}", ty),
        },
        Expression::Variable(id) => (id.name.clone(), false),
        _ => panic!("Unsupported expression found: {:#?}", expr),
    }
}

// testing the transpiler
#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ws(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn test_parse() {
        let source = r#"
            contract Test {
                function foo(uint x) public returns (uint) {
                    return x + 1;
                }
            }
        "#;
        let result = parse(source, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_counter() {
        let source = r#"
            // language_version >= 0.16 && <= 0.18
            pragma solidity ^0.8.22;

            import "./counter-lib.sol";

            contract CounterContract {
                using CounterLib for Counter;

                Counter public round;

                function increment() public {
                    round.increment(1);
                }
            }
        "#;
        let result = parse(source, 0);
        let expected_output = r#"
            pragma language_version >= 0.16 && <= 0.18;

            import CompactStandardLibrary;

            export ledger round: Counter;

            export circuit increment(): [] {
                round.increment(1);
            }
        "#;
        assert!(result.is_ok());
        println!("Parsed AST: {:#?}", result);
        assert!(strip_ws(expected_output) == strip_ws(result.unwrap().0.to_string().as_str()));
    }
}
