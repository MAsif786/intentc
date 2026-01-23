use crate::ast::IntentFile;
use crate::codegen::GenerationResult;
use crate::error::CompileResult;
use std::fs;
use std::path::Path;

pub fn generate_policies(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    let mut content = String::new();

    content.push_str("# Intent Compiler Generated Policies\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from fastapi import HTTPException, status\n");
    content.push_str("from typing import Any, Optional\n\n");

    for policy in &ast.policies {
        content.push_str(&generate_policy_function(policy, ast, None));
        content.push_str("\n\n");
    }

    for entity in &ast.entities {
        for policy in &entity.policies {
            content.push_str(&generate_policy_function(policy, ast, Some(&entity.name)));
            content.push_str("\n\n");
        }
    }

    let path = output_dir.join("logic/policies.py");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, &content)?;
    result.add_file("logic/policies.py", content.lines().count());

    Ok(result)
}

fn generate_policy_function(policy: &crate::ast::Policy, _ast: &IntentFile, entity_context: Option<&str>) -> String {
    let mut content = String::new();
    let func_name = if let Some(entity) = entity_context {
        format!("check_{}_{}", entity, policy.name)
    } else {
        format!("check_{}", policy.name)
    };

    content.push_str(&format!("def {}(user: Any, resource: Any = None) -> None:\n", func_name));
    content.push_str(&format!("    \"\"\"Policy: {}\"\"\"\n", policy.name));

    // We assume 'user' maps to policy.subject
    // And 'resource' maps to the entity if it's an entity-scoped policy

    let target_var = if entity_context.is_some() { "resource" } else { "None" };
    
    // We reuse expression_to_python logic but need to adapt it. 
    // Since we don't have access to api.rs's internal helper, we might need to duplicate or move `expression_to_python`.
    // For now, I'll duplicate the simple version or ask to move it to a shared place.
    // Let's implement a local version for now that uses `user` instead of `current_user`.
    
    let check_expr = expression_to_python(&policy.require, &policy.subject, target_var);
    
    content.push_str(&format!("    if not ({}):\n", check_expr));
    content.push_str(&format!("        raise HTTPException(status_code=403, detail=\"Access denied by policy {}\")\n", policy.name));

    content
}

fn expression_to_python(expr: &crate::ast::Expression, subject: &str, target_var: &str) -> String {
    use crate::ast::Expression;
    match expr {
        Expression::Binary { left, operator, right } => {
            let l = expression_to_python(left, subject, target_var);
            let r = expression_to_python(right, subject, target_var);
            let op = match operator {
                crate::ast::BinaryOperator::Equal => "==",
                crate::ast::BinaryOperator::NotEqual => "!=",
                crate::ast::BinaryOperator::GreaterThan => ">",
                crate::ast::BinaryOperator::LessThan => "<",
                crate::ast::BinaryOperator::GreaterEqual => ">=",
                crate::ast::BinaryOperator::LessEqual => "<=",
            };
            format!("{} {} {}", l, op, r)
        }
        Expression::Logical { left, operator, right } => {
            let l = expression_to_python(left, subject, target_var);
            let r = expression_to_python(right, subject, target_var);
            let op = match operator {
                crate::ast::LogicalOperator::And => "and",
                crate::ast::LogicalOperator::Or => "or",
            };
            format!("({} {} {})", l, op, r)
        }
        Expression::Not(inner) => {
            format!("not ({})", expression_to_python(inner, subject, target_var))
        }
        Expression::FieldAccess { entity, field } => {
            if entity == subject || entity == "subject" {
                format!("user.{}", field)
            } else {
                format!("{}.{}", target_var, field)
            }
        }
        Expression::Literal(lit) => match lit {
            crate::ast::LiteralValue::String(s) => format!("\"{}\"", s),
            crate::ast::LiteralValue::Number(n) => n.to_string(),
            crate::ast::LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
        Expression::Identifier(s) => s.clone(),
    }
}
