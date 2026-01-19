// Intent Compiler - Business Rules Generator
// Generates Python functions from rule definitions

use std::fs;
use std::path::Path;

use crate::ast::{Expression, Consequence, LiteralValue, BinaryOperator, LogicalOperator, IntentFile, Rule};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate business rules
pub fn generate_rules(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    let mut content = String::new();

    // Imports
    content.push_str("# Intent Compiler Generated Business Rules\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from typing import Any, Dict\n");
    content.push_str("from fastapi import HTTPException\n");
    content.push_str("import logging\n\n");
    content.push_str("logger = logging.getLogger(__name__)\n\n\n");

    // Generate ValidationError class
    content.push_str("class ValidationError(Exception):\n");
    content.push_str("    \"\"\"Raised when a business rule validation fails\"\"\"\n");
    content.push_str("    def __init__(self, message: str):\n");
    content.push_str("        self.message = message\n");
    content.push_str("        super().__init__(message)\n\n\n");

    // Generate each rule
    for rule in &ast.rules {
        content.push_str(&generate_rule(rule)?);
        content.push_str("\n\n");
    }

    // Generate a function to run all rules
    content.push_str("def validate_all(entity_name: str, data: Dict[str, Any]) -> None:\n");
    content.push_str("    \"\"\"Run all validation rules for an entity\"\"\"\n");
    content.push_str("    rules = {\n");
    for rule in &ast.rules {
        // Extract entity name from the condition
        let entity = extract_entity_from_expression(&rule.condition);
        if let Some(entity_name) = entity {
            content.push_str(&format!(
                "        \"{}\": [{}],\n",
                entity_name, rule.name
            ));
        }
    }
    content.push_str("    }\n");
    content.push_str("    for rule_func in rules.get(entity_name, []):\n");
    content.push_str("        rule_func(data)\n");

    let lines = content.lines().count();
    let path = output_dir.join("logic/rules.py");
    fs::write(&path, &content)?;
    result.add_file("logic/rules.py", lines);

    // Generate __init__.py
    let init_content = "# Intent Compiler Generated Logic\nfrom .rules import *\n";
    fs::write(output_dir.join("logic/__init__.py"), init_content)?;

    Ok(result)
}

/// Generate a single rule function
fn generate_rule(rule: &Rule) -> CompileResult<String> {
    let mut content = String::new();

    content.push_str(&format!("def {}(data: Dict[str, Any]) -> None:\n", rule.name));
    content.push_str(&format!("    \"\"\"Business rule: {}\"\"\"\n", rule.name));
    
    // Generate condition check
    let condition = generate_expression(&rule.condition);
    content.push_str(&format!("    if {}:\n", condition));
    
    // Generate consequence
    content.push_str(&generate_consequence(&rule.consequence));

    Ok(content)
}

/// Generate Python expression from AST Expression
fn generate_expression(expr: &Expression) -> String {
    match expr {
        Expression::Binary { left, operator, right } => {
            let left_str = generate_expression(left);
            let right_str = generate_expression(right);
            let op_str = match operator {
                BinaryOperator::Equal => "==",
                BinaryOperator::NotEqual => "!=",
                BinaryOperator::GreaterThan => ">",
                BinaryOperator::LessThan => "<",
                BinaryOperator::GreaterEqual => ">=",
                BinaryOperator::LessEqual => "<=",
            };
            format!("({} {} {})", left_str, op_str, right_str)
        }
        Expression::Logical { left, operator, right } => {
            let left_str = generate_expression(left);
            let right_str = generate_expression(right);
            let op_str = match operator {
                LogicalOperator::And => "and",
                LogicalOperator::Or => "or",
            };
            format!("({} {} {})", left_str, op_str, right_str)
        }
        Expression::Not(inner) => {
            format!("not ({})", generate_expression(inner))
        }
        Expression::FieldAccess { entity: _, field } => {
            format!("data.get(\"{}\")", field)
        }
        Expression::Literal(value) => {
            match value {
                LiteralValue::String(s) => format!("\"{}\"", s),
                LiteralValue::Number(n) => n.to_string(),
                LiteralValue::Boolean(b) => if *b { "True" } else { "False" }.to_string(),
            }
        }
        Expression::Identifier(name) => {
            // Could be an enum value or variable
            format!("\"{}\"", name)
        }
    }
}

/// Generate Python code for a consequence
fn generate_consequence(consequence: &Consequence) -> String {
    match consequence {
        Consequence::Reject(message) => {
            format!(
                "        raise HTTPException(status_code=400, detail=\"{}\")\n",
                message
            )
        }
        Consequence::Log(message) => {
            format!(
                "        logger.info(\"{}\")\n",
                message
            )
        }
        Consequence::ActionCall { action, args } => {
            let args_str: Vec<String> = args.iter().map(generate_expression).collect();
            format!(
                "        {}({})\n",
                action,
                args_str.join(", ")
            )
        }
    }
}

/// Extract entity name from an expression (for grouping rules)
fn extract_entity_from_expression(expr: &Expression) -> Option<String> {
    match expr {
        Expression::FieldAccess { entity, .. } => Some(entity.clone()),
        Expression::Binary { left, .. } => extract_entity_from_expression(left),
        Expression::Logical { left, .. } => extract_entity_from_expression(left),
        Expression::Not(inner) => extract_entity_from_expression(inner),
        _ => None,
    }
}
