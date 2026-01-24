// Intent Compiler - Python Service Generator
// Generates service classes with business logic

use crate::ast::{Action, Decorator, DeriveValue, IntentFile, MapTransform};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;
use std::fs;
use std::path::Path;

pub fn generate_services(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();

    // Create services directory
    let services_dir = output_dir.join("services");
    fs::create_dir_all(&services_dir)?;

    // Generate entity-specific services
    for entity in &ast.entities {
        let content = generate_entity_service(entity, ast);
        let filename = format!("{}_service.py", entity.name.to_lowercase());
        let path = services_dir.join(&filename);
        fs::write(&path, &content)?;
        result.add_file(format!("services/{}", filename), content.lines().count());
    }

    // Generate __init__.py
    let init_content = generate_services_init(ast);
    let init_path = services_dir.join("__init__.py");
    fs::write(&init_path, &init_content)?;
    result.add_file("services/__init__.py", init_content.lines().count());

    Ok(result)
}

fn generate_entity_service(entity: &crate::ast::Entity, ast: &IntentFile) -> String {
    let name = &entity.name;
    let name_lower = name.to_lowercase();
    
    let mut content = String::new();
    
    // Header
    content.push_str("# Intent Compiler Generated Service\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from typing import Optional\n");
    content.push_str("from sqlalchemy.orm import Session\n");
    content.push_str("from fastapi import HTTPException\n\n");
    content.push_str(&format!("from db.models import {}Model\n", name));
    content.push_str(&format!("from repositories.{}_repository import {}_repository\n", name_lower, name_lower));
    content.push_str("from core.security import get_password_hash, verify_password, create_access_token\n\n\n");
    
    // Service class
    content.push_str(&format!("class {}Service:\n", name));
    content.push_str(&format!("    \"\"\"Service for {} entity with business logic\"\"\"\n\n", name));
    content.push_str(&format!("    repo = {}_repository\n\n", name_lower));
    
    // Generate CRUD methods
    content.push_str(&generate_crud_methods(name, &name_lower));
    
    // Generate action-specific methods for this entity
    for action in &ast.actions {
        if let Some(output) = &action.output {
            if output.entity == *name {
                content.push_str(&generate_action_method(action, name, &name_lower));
            }
        }
    }
    
    // Singleton instance
    content.push_str(&format!("\n# Singleton instance\n"));
    content.push_str(&format!("{}_service = {}Service()\n", name_lower, name));
    
    content
}

fn generate_crud_methods(name: &str, _name_lower: &str) -> String {
    let mut content = String::new();
    
    // Get all
    content.push_str(&format!("    def get_all(self, db: Session, skip: int = 0, limit: int = 100) -> list[{}Model]:\n", name));
    content.push_str("        \"\"\"Get all records with pagination\"\"\"\n");
    content.push_str("        return self.repo.get_all(db, skip=skip, limit=limit)\n\n");
    
    // Get by ID
    content.push_str(&format!("    def get_by_id(self, db: Session, id: str) -> Optional[{}Model]:\n", name));
    content.push_str("        \"\"\"Get a record by ID\"\"\"\n");
    content.push_str("        return self.repo.get_by_id(db, id)\n\n");
    
    // Create
    content.push_str(&format!("    def create(self, db: Session, data: dict) -> {}Model:\n", name));
    content.push_str("        \"\"\"Create a new record\"\"\"\n");
    content.push_str("        return self.repo.create(db, data)\n\n");
    
    // Update
    content.push_str(&format!("    def update(self, db: Session, id: str, data: dict) -> Optional[{}Model]:\n", name));
    content.push_str("        \"\"\"Update a record by ID\"\"\"\n");
    content.push_str("        return self.repo.update(db, id, data)\n\n");
    
    // Delete
    content.push_str("    def delete(self, db: Session, id: str) -> bool:\n");
    content.push_str("        \"\"\"Delete a record by ID\"\"\"\n");
    content.push_str("        return self.repo.delete(db, id)\n\n");
    
    content
}

fn generate_action_method(action: &Action, entity_name: &str, _entity_lower: &str) -> String {
    let mut content = String::new();
    let action_name = &action.name;
    
    // Build parameters (match controllers.rs)
    let mut params = Vec::new();

    // Get API decorator for path params
    let default_path = "/".to_string();
    let (_, path) = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, path } = d {
            Some((method, path))
        } else {
            None
        }
    }).unwrap_or((&crate::ast::HttpMethod::Get, &default_path));

    for segment in path.split('/') {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len()-1];
            params.push(param_name.to_string());
        }
    }

    // Add data if applicable
    let has_api = action.decorators.iter().any(|d| matches!(d, Decorator::Api { .. }));
    let method = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, .. } = d { Some(method) } else { None }
    }).unwrap_or(&crate::ast::HttpMethod::Get);

    if has_api && matches!(method, crate::ast::HttpMethod::Post | crate::ast::HttpMethod::Put | crate::ast::HttpMethod::Patch) {
        params.push("data".to_string());
    } else if !has_api {
        if let Some(input) = &action.input {
            for field in &input.fields {
                params.push(field.name.clone());
            }
        }
    }

    params.push("db: Session".to_string());

    let requires_auth = action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. }));
    if requires_auth {
        params.push("current_user".to_string());
    }

    let params_str = params.join(", ");

    // Check if this is a select-based action (login-style)
    let has_find = action.process.as_ref().map(|p| {
        p.derives.iter().any(|d| {
            matches!(&d.value, DeriveValue::Select { .. })
        })
    }).unwrap_or(false);
    
    // Check for password hashing (signup-style)
    let has_hash = action.input.as_ref().map(|i| {
        i.fields.iter().any(|f| {
            f.decorators.iter().any(|d| matches!(d, Decorator::Map { transform: MapTransform::Hash, .. }))
        })
    }).unwrap_or(false);
    
    if has_find {
        // Login-style method
        content.push_str(&format!("    def {}(self, {}) -> dict:\n", action_name, params_str));
        content.push_str(&format!("        \"\"\"Business logic for {}\"\"\"\n", action_name));
        
        // Process derives in order
        if let Some(process) = &action.process {
            for derive in &process.derives {
                match &derive.value {
                    DeriveValue::Select { entity, predicate } => {
                        let py_code = select_to_python(entity, predicate);
                        content.push_str(&format!("        {} = {}\n", derive.name, py_code));
                        content.push_str(&format!("        if not {}:\n", derive.name));
                        content.push_str("            raise HTTPException(status_code=400, detail=\"Not found\")\n");
                    }
                    DeriveValue::Compute { function, args } if function == "verify_hash" => {
                        let py_code = compute_to_python(function, args);
                        content.push_str(&format!("        if not {}:\n", py_code));
                        content.push_str("            raise HTTPException(status_code=400, detail=\"Invalid credentials\")\n");
                    }
                    DeriveValue::SystemCall { namespace, capability, args } => {
                        let py_code = system_call_to_python(namespace, capability, args);
                        content.push_str(&format!("        {} = {}\n", derive.name, py_code));
                    }
                    _ => {}
                }
            }
        }
        
        // Return output
        content.push_str("        return {\n");
        if let Some(output) = &action.output {
            for field in &output.fields {
                let is_derived = action.process.as_ref().map(|p| {
                    p.derives.iter().any(|d| d.name == *field)
                }).unwrap_or(false);
                
                if is_derived {
                    content.push_str(&format!("            \"{}\": {},\n", field, field));
                } else {
                    // Assume it's from the found entity (first derive with select)
                    let found_var = action.process.as_ref()
                        .and_then(|p| p.derives.iter().find(|d| {
                            matches!(&d.value, DeriveValue::Select { .. })
                        }))
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "user".to_string());
                    content.push_str(&format!("            \"{}\": {}.{},\n", field, found_var, field));
                }
            }
        }
        content.push_str("        }\n\n");
    } else if has_hash {
        // Signup-style method
        content.push_str(&format!("    def {}(self, {}) -> {}Model:\n", action_name, params_str, entity_name));
        content.push_str(&format!("        \"\"\"Business logic for {}\"\"\"\n", action_name));
        content.push_str("        data_dict = data.model_dump()\n");
        
        if let Some(input) = &action.input {
            for param in &input.fields {
                for dec in &param.decorators {
                    if let Decorator::Map { target, transform } = dec {
                        if matches!(transform, MapTransform::Hash) {
                            content.push_str(&format!("        data_dict['{}'] = get_password_hash(data_dict.pop('{}'))\n", target, param.name));
                        }
                    }
                }
            }
        }
        
        content.push_str("        return self.repo.create(db, data_dict)\n\n");
    } else {
        // Generic action (like create_product or list_products)
        // Determine return type
        let returns_list = if matches!(method, crate::ast::HttpMethod::Get) && !path.contains('{') {
            true
        } else {
            false
        };

        if returns_list {
            content.push_str(&format!("    def {}(self, {}) -> list[{}Model]:\n", action_name, params_str, entity_name));
        } else {
            content.push_str(&format!("    def {}(self, {}) -> {}Model:\n", action_name, params_str, entity_name));
        }
        content.push_str(&format!("        \"\"\"Business logic for {}\"\"\"\n", action_name));

        if matches!(method, crate::ast::HttpMethod::Post) {
             content.push_str("        return self.repo.create(db, data.model_dump())\n\n");
        } else if matches!(method, crate::ast::HttpMethod::Get) && path.contains('{') {
             content.push_str("        result = self.repo.get_by_id(db, id)\n");
             content.push_str("        if not result:\n");
             content.push_str("            raise HTTPException(status_code=404, detail=\"Not found\")\n");
             content.push_str("        return result\n\n");
        } else {
             content.push_str("        return self.repo.get_all(db)\n\n");
        }
    }
    
    content
}

fn generate_services_init(ast: &IntentFile) -> String {
    let mut content = String::new();
    content.push_str("# Intent Compiler Generated Services\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    
    for entity in &ast.entities {
        let name_lower = entity.name.to_lowercase();
        content.push_str(&format!(
            "from services.{}_service import {}Service, {}_service\n",
            name_lower, entity.name, name_lower
        ));
    }
    
    content.push_str("\n__all__ = [\n");
    for entity in &ast.entities {
        content.push_str(&format!("    \"{}Service\",\n", entity.name));
        content.push_str(&format!("    \"{}_service\",\n", entity.name.to_lowercase()));
    }
    content.push_str("]\n");
    
    content
}

fn select_to_python(entity: &str, predicate: &crate::ast::Predicate) -> String {
    use crate::ast::{FieldReference, CompareOp};
    
    let right_str = match &predicate.value {
        FieldReference::InputField(name) => format!("data.{}", name),
        FieldReference::DerivedField { name, field } => format!("{}.{}", name, field),
        FieldReference::Literal(lit) => match lit {
            crate::ast::LiteralValue::String(s) => format!("\"{}\"", s),
            crate::ast::LiteralValue::Number(n) => n.to_string(),
            crate::ast::LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
    };
    
    let op_str = match predicate.operator {
        CompareOp::Equal => "==",
        CompareOp::NotEqual => "!=",
        CompareOp::Less => "<",
        CompareOp::Greater => ">",
    };
    
    let filter_field = match &predicate.field {
        FieldReference::InputField(name) | FieldReference::DerivedField { field: name, .. } => name.clone(),
        _ => "id".to_string(),
    };
    
    format!("db.query({}Model).filter({}Model.{} {} {}).first()", entity, entity, filter_field, op_str, right_str)
}

fn compute_to_python(function: &str, args: &[crate::ast::FunctionArg]) -> String {
    use crate::ast::FunctionArg;
    
    let args_str: Vec<String> = args.iter().map(|arg| match arg {
        FunctionArg::TypeName(s) => s.clone(),
        FunctionArg::Identifier(s) => format!("data.{}", s),
        FunctionArg::FieldAccess { path } => path.join("."),
        FunctionArg::Literal(lit) => match lit {
            crate::ast::LiteralValue::String(s) => format!("\"{}\"", s),
            crate::ast::LiteralValue::Number(n) => n.to_string(),
            crate::ast::LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
    }).collect();
    
    match function {
        "verify_hash" => {
            if args.len() >= 2 {
                format!("verify_password({}, {})", args_str[0], args_str[1])
            } else {
                "False".to_string()
            }
        }
        "slugify" => {
            if !args.is_empty() {
                format!("\"{}\".lower().replace(' ', '-')", args_str[0])
            } else {
                "\"\"".to_string()
            }
        }
        _ => {
            format!("{}({})", function, args_str.join(", "))
        }
    }
}

fn system_call_to_python(namespace: &str, capability: &str, args: &[crate::ast::FunctionArg]) -> String {
    use crate::ast::FunctionArg;
    
    let args_str: Vec<String> = args.iter().map(|arg| match arg {
        FunctionArg::TypeName(s) => s.clone(),
        FunctionArg::Identifier(s) => format!("data.{}", s),
        FunctionArg::FieldAccess { path } => path.join("."),
        FunctionArg::Literal(lit) => match lit {
            crate::ast::LiteralValue::String(s) => format!("\"{}\"", s),
            crate::ast::LiteralValue::Number(n) => n.to_string(),
            crate::ast::LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
    }).collect();
    
    match (namespace, capability) {
        ("jwt", "create") => {
            if !args.is_empty() {
                format!("create_access_token(data={{\"sub\": {}}})", args_str[0])
            } else {
                "create_access_token(data={})".to_string()
            }
        }
        ("jwt", "verify") => {
            if !args.is_empty() {
                format!("verify_token({})", args_str[0])
            } else {
                "verify_token()".to_string()
            }
        }
        _ => {
            format!("{}_{}({})", namespace, capability, args_str.join(", "))
        }
    }
}
