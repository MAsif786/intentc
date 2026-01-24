// Intent Compiler - Python Controller Generator
// Generates controller classes for route handling

use crate::ast::{Action, Decorator, IntentFile};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;
use std::fs;
use std::path::Path;

pub fn generate_controllers(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();

    // Create controllers directory
    let controllers_dir = output_dir.join("controllers");
    fs::create_dir_all(&controllers_dir)?;

    // Generate entity-specific controllers
    for entity in &ast.entities {
        let content = generate_entity_controller(entity, ast);
        let filename = format!("{}_controller.py", entity.name.to_lowercase());
        let path = controllers_dir.join(&filename);
        fs::write(&path, &content)?;
        result.add_file(format!("controllers/{}", filename), content.lines().count());
    }

    // Generate __init__.py
    let init_content = generate_controllers_init(ast);
    let init_path = controllers_dir.join("__init__.py");
    fs::write(&init_path, &init_content)?;
    result.add_file("controllers/__init__.py", init_content.lines().count());

    Ok(result)
}

fn generate_entity_controller(entity: &crate::ast::Entity, ast: &IntentFile) -> String {
    let name = &entity.name;
    let name_lower = name.to_lowercase();
    
    let mut content = String::new();
    
    // Header
    content.push_str("# Intent Compiler Generated Controller\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from typing import Optional\n");
    content.push_str("from sqlalchemy.orm import Session\n\n");
    content.push_str(&format!("from db.models import {}Model\n", name));
    content.push_str(&format!("from services.{}_service import {}_service\n\n\n", name_lower, name_lower));
    
    // Controller class
    content.push_str(&format!("class {}Controller:\n", name));
    content.push_str(&format!("    \"\"\"Controller for {} entity routes\"\"\"\n\n", name));
    content.push_str(&format!("    service = {}_service\n\n", name_lower));
    
    // CRUD methods
    content.push_str(&generate_crud_controller_methods(name, &name_lower));
    
    // Action-specific methods
    for action in &ast.actions {
        if let Some(output) = &action.output {
            if output.entity == *name {
                content.push_str(&generate_action_controller_method(action, name, ast));
            }
        }
    }
    
    // Singleton instance
    content.push_str(&format!("\n# Singleton instance\n"));
    content.push_str(&format!("{}_controller = {}Controller()\n", name_lower, name));
    
    content
}

fn generate_crud_controller_methods(name: &str, _name_lower: &str) -> String {
    let mut content = String::new();
    
    // List
    content.push_str(&format!("    async def list(self, db: Session, skip: int = 0, limit: int = 100) -> list[{}Model]:\n", name));
    content.push_str("        \"\"\"List all records\"\"\"\n");
    content.push_str("        return self.service.get_all(db, skip=skip, limit=limit)\n\n");
    
    // Get
    content.push_str(&format!("    async def get(self, db: Session, id: str) -> Optional[{}Model]:\n", name));
    content.push_str("        \"\"\"Get a record by ID\"\"\"\n");
    content.push_str("        return self.service.get_by_id(db, id)\n\n");
    
    // Create
    content.push_str(&format!("    async def create(self, db: Session, data) -> {}Model:\n", name));
    content.push_str("        \"\"\"Create a new record\"\"\"\n");
    content.push_str("        return self.service.create(db, data.model_dump())\n\n");
    
    // Update
    content.push_str(&format!("    async def update(self, db: Session, id: str, data) -> Optional[{}Model]:\n", name));
    content.push_str("        \"\"\"Update a record by ID\"\"\"\n");
    content.push_str("        return self.service.update(db, id, data.model_dump())\n\n");
    
    // Delete
    content.push_str("    async def delete(self, db: Session, id: str) -> dict:\n");
    content.push_str("        \"\"\"Delete a record by ID\"\"\"\n");
    content.push_str("        success = self.service.delete(db, id)\n");
    content.push_str("        return {\"success\": success}\n\n");
    
    content
}

fn generate_action_controller_method(action: &Action, entity_name: &str, ast: &IntentFile) -> String {
    let mut content = String::new();
    let action_name = &action.name;
    
    // Build parameters (match api.rs)
    let mut params = Vec::new();
    let mut call_params = Vec::new();

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
            call_params.push(param_name.to_string());
        }
    }

    // Add data if applicable
    let has_api = action.decorators.iter().any(|d| matches!(d, Decorator::Api { .. }));
    let method = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, .. } = d { Some(method) } else { None }
    }).unwrap_or(&crate::ast::HttpMethod::Get);

    if has_api && matches!(method, crate::ast::HttpMethod::Post | crate::ast::HttpMethod::Put | crate::ast::HttpMethod::Patch) {
        params.push("data".to_string());
        call_params.push("data".to_string());
    } else if !has_api {
        if let Some(input) = &action.input {
            for field in &input.fields {
                params.push(field.name.clone());
                call_params.push(field.name.clone());
            }
        }
    }

    params.push("db: Session".to_string());
    call_params.push("db".to_string());

    let requires_auth = action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. }));
    if requires_auth {
        params.push("current_user".to_string());
        call_params.push("current_user".to_string());
    }

    let params_str = params.join(", ");
    let call_params_str = call_params.join(", ");

    // Check return type based on action
    let has_find = action.process.as_ref().map(|p| {
        p.derives.iter().any(|d| {
            matches!(&d.value, crate::ast::DeriveValue::Select { .. })
        })
    }).unwrap_or(false);
    
    // Determine return type
    let returns_list = if matches!(method, crate::ast::HttpMethod::Get) && !path.contains('{') {
        true
    } else {
        false
    };

    if has_find {
        content.push_str(&format!("    async def {}(self, {}) -> dict:\n", action_name, params_str));
    } else if returns_list {
        content.push_str(&format!("    async def {}(self, {}) -> list[{}Model]:\n", action_name, params_str, entity_name));
    } else {
        content.push_str(&format!("    async def {}(self, {}) -> {}Model:\n", action_name, params_str, entity_name));
    }
    
    content.push_str(&format!("        \"\"\"Handle {} action\"\"\"\n", action_name));

    // Policy Check (Prefix)
    // For GET/DELETE we check before. For POST we check after creating but before committing?
    // Actually, in the new flow, the Service handles creation.
    // So the Controller should probably check PRE-conditions.
    let policy_check = generate_policy_enforcement(action, ast, "None").unwrap_or_default();
    content.push_str(&policy_check);

    content.push_str(&format!("        return self.service.{}({})\n\n", action_name, call_params_str));
    
    content
}

fn generate_policy_enforcement(action: &Action, ast: &IntentFile, target_var: &str) -> CompileResult<String> {
    let mut content = String::new();
    
    for decorator in &action.decorators {
        if let Decorator::Policy(name) = decorator {
            let policy = if name.contains('.') {
                let parts: Vec<&str> = name.split('.').collect();
                let entity_name = parts[0];
                let policy_name = parts[1];
                
                ast.find_entity(entity_name)
                    .and_then(|e| e.policies.iter().find(|p| p.name == policy_name))
            } else {
                ast.policies.iter().find(|p| p.name == *name)
            };

            if let Some(_p) = policy {
                 let func_name = if name.contains('.') {
                    let parts: Vec<&str> = name.split('.').collect();
                    format!("check_{}_{}", parts[0], parts[1])
                } else {
                    format!("check_{}", name)
                };

                let resource_arg = if target_var == "None" {
                    ""
                } else {
                    &format!(", resource={}", target_var)
                };

                content.push_str(&format!("        # Enforce policy: {}\n", name));
                content.push_str(&format!("        {}(user=current_user{})\n", func_name, resource_arg));
            }
        }
    }
    
    Ok(content)
}

fn generate_controllers_init(ast: &IntentFile) -> String {
    let mut content = String::new();
    content.push_str("# Intent Compiler Generated Controllers\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    
    for entity in &ast.entities {
        let name_lower = entity.name.to_lowercase();
        content.push_str(&format!(
            "from controllers.{}_controller import {}Controller, {}_controller\n",
            name_lower, entity.name, name_lower
        ));
    }
    
    content.push_str("\n__all__ = [\n");
    for entity in &ast.entities {
        content.push_str(&format!("    \"{}Controller\",\n", entity.name));
        content.push_str(&format!("    \"{}_controller\",\n", entity.name.to_lowercase()));
    }
    content.push_str("]\n");
    
    content
}
