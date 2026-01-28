use core::panic;
use serde::Deserialize;
use solang_parser::{
    parse,
    pt::{ContractPart, Expression, SourceUnitPart, Statement},
};
use std::{env, fs, path::Path};
use toml;
mod compact;

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

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
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

    // visits the AST and generates Compact code
    let mut top_level = true;
    for part in &ast.0 {
        match part {
            SourceUnitPart::ContractDefinition(def) => {
                println!("found contract {:?}", def.name);
                // import Compact standard library
                compact_output.push(String::from("import CompactStandardLibrary;\n"));
                // visit each part of the contract
                for part in &def.parts {
                    visit(part, top_level, &mut compact_output);
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
    Ok(())
}

fn visit(part: &ContractPart, top_level: bool, compact_output: &mut Vec<String>) {
    println!("Visiting contract part: {:#?}", part);
    let mut formatter = Formatter::new();

    match part {
        ContractPart::VariableDefinition(var_def) => {
            match &var_def.name {
                None => panic!("VariableDefinition without a name found!"),
                Some(var_name) => {
                    let var_type = match &var_def.ty {
                        solang_parser::pt::Expression::Variable(id) => id.name.clone(),
                        _ => panic!("Unsupported variable type expression!"),
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
                    // if at top level, the variable is a ledger variable
                    if top_level {
                        compact_output.push(format!(
                            "{}ledger {}: {};\n",
                            if is_exported { "export " } else { "" },
                            var_name,
                            var_type
                        ));
                    } else {
                        compact_output.push(format!("let {}: {};\n", var_name, var_type));
                    }
                }
            }
        }
        ContractPart::FunctionDefinition(func_def) => {
            let func_name = match &func_def.name {
                Some(name) => name.name.clone(),
                None => panic!("FunctionDefinition without a name found!"),
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
                                            panic!(
                                                "Parameter of type '{}' is missing a name!",
                                                sol_type
                                            )
                                        }
                                        Some(param_name) => {
                                            params.push(format!("{}: {}", param_name, compact_type))
                                        }
                                    },
                                    None => {
                                        panic!("Unsupported parameter type '{}' found!", sol_type)
                                    }
                                }
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
                                    Some(compact_type) => compact_type,
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
            // function body
            let func_body: String = match &func_def.body {
                Some(body) => visit_statement(body, &mut formatter),
                None => String::from(""),
            };
            // function signature
            let func_signature = format!(
                "{}circuit {}({}): {} {{",
                if is_exported { "export " } else { "" },
                func_name,
                params.join(", "),
                return_type
            );
            compact_output.push(func_signature);
            // function body (TODO: transpile statements)
            compact_output.push(func_body);
            compact_output.push(String::from("}\n"));
        }
        _ => (),
    }
}

fn visit_statement(part: &Statement, formatter: &mut Formatter) -> String {
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
                    .map(|stmt| visit_statement(stmt, formatter))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .trim_end()
                    .to_string();
            }
        }
        Statement::Expression(_, expr) => {
            let result = visit_expression(expr, formatter);
            formatter.line(&format!("{};", result));
            return formatter.print();
        }
        _ => String::from(""),
    }
}

fn visit_expression(expr: &Expression, formatter: &mut Formatter) -> String {
    match expr {
        Expression::Variable(id) => id.name.clone(),
        Expression::FunctionCall(_, name, params) => {
            // parses the function name
            let name = visit_expression(name, formatter);
            // parses the function params
            let param_strs: Vec<String> = params
                .iter()
                .map(|param| visit_expression(param, formatter))
                .collect();
            if !param_strs.is_empty() {
                return format!("{}({})", name, param_strs.join(", "));
            } else {
                return format!("{}()", name);
            }
        }
        Expression::MemberAccess(_, expr, identifier) => {
            let object = visit_expression(expr, formatter);
            format!("{}.{}", object, identifier.name)
        }
        Expression::NumberLiteral(_, number, _, _) => number.to_string(),
        _ => String::from(""),
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
