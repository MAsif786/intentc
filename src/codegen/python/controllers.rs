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
    content.push_str("# Intent Compiler Generated Controller with Routes\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from typing import Optional, List\n");
    content.push_str("from fastapi import APIRouter, Depends, HTTPException, status\n");
    content.push_str("from sqlalchemy.orm import Session\n\n");
    content.push_str("from db.database import get_db\n");
    content.push_str(&format!("from db.models import {}Model\n", name));
    content.push_str("from models import *\n");
    content.push_str(&format!("from services.{}_service import {}_service\n", name_lower, name_lower));
    content.push_str("from core.security import get_current_user_token, get_password_hash\n");
    content.push_str("from logic.policies import *\n\n");
    
    // Router definition
    content.push_str(&format!("router = APIRouter(prefix=\"/{}s\", tags=[\"{}\"])\n\n\n", name_lower, name));

    // CRUD methods as Routes
    content.push_str(&generate_crud_routes(name, &name_lower, ast));
    
    // Action methods as Routes
    for action in &ast.actions {
        if let Some(output) = &action.output {
            if output.entity == *name {
                content.push_str(&generate_action_route(action, name, ast));
            }
        }
    }
    
    content
}

fn generate_crud_routes(name: &str, name_lower: &str, ast: &IntentFile) -> String {
    let mut content = String::new();
    
    // Helper to check if an action already defines this route
    let route_exists = |method: crate::ast::HttpMethod, path_to_check: &str| -> bool {
        let name_plural = format!("{}s", name_lower);
        ast.actions.iter().any(|a| {
            a.decorators.iter().any(|d| {
                if let Decorator::Api { method: m, path: p } = d {
                    if *m != method { return false; }
                    let p_clean = p.trim_matches('/');
                    let check_clean = path_to_check.trim_matches('/');
                    p_clean == check_clean || 
                    p_clean == format!("{}/{}", name_plural, check_clean).trim_matches('/') ||
                    p_clean == format!("{}/{}", name_lower, check_clean).trim_matches('/')
                } else {
                    false
                }
            })
        })
    };

    // List Route
    if !route_exists(crate::ast::HttpMethod::Get, "/") {
        content.push_str(&format!("@router.get(\"/\", response_model=List[{0}])\n", name));
        content.push_str(&format!("async def list_{0}s(skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):\n", name_lower));
        content.push_str(&format!("    \"\"\"List all {0}s\"\"\"\n", name_lower));
        content.push_str(&format!("    return {0}_service.get_all(db, skip=skip, limit=limit)\n\n", name_lower));
    }
    
    // Get Route
    if !route_exists(crate::ast::HttpMethod::Get, "/{id}") {
        content.push_str(&format!("@router.get(\"/{{id}}\", response_model={0})\n", name));
        content.push_str(&format!("async def get_{0}(id: str, db: Session = Depends(get_db)):\n", name_lower));
        content.push_str(&format!("    \"\"\"Get {0} by ID\"\"\"\n", name));
        content.push_str(&format!("    result = {0}_service.get_by_id(db, id)\n", name_lower));
        content.push_str("    if not result:\n");
        content.push_str(&format!("        raise HTTPException(status_code=404, detail=\"{} not found\")\n", name));
        content.push_str("    return result\n\n");
    }
    
    content
}

fn generate_action_route(action: &Action, entity_name: &str, ast: &IntentFile) -> String {
    let mut content = String::new();
    let action_name = &action.name;
    let entity_lower = entity_name.to_lowercase();
    
    // Get API info
    let default_path = "/".to_string();
    let (method, path) = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, path } = d {
            Some((method, path))
        } else {
            None
        }
    }).unwrap_or((&crate::ast::HttpMethod::Get, &default_path));

    let method_str = format!("{:?}", method).to_lowercase();
    
    // Strip entity prefix from path if present (e.g. /users/signup -> /signup because router has /users prefix)
    let entity_prefix = format!("/{}s", entity_lower);
    let entity_prefix_single = format!("/{}", entity_lower);
    let mut relative_path = path.clone();
    if relative_path.starts_with(&entity_prefix) {
        relative_path = relative_path[entity_prefix.len()..].to_string();
    } else if relative_path.starts_with(&entity_prefix_single) {
        relative_path = relative_path[entity_prefix_single.len()..].to_string();
    }
    
    if relative_path.is_empty() {
        relative_path = "/".to_string();
    }
    if !relative_path.starts_with('/') {
        relative_path = format!("/{}", relative_path);
    }

    // Response model
    let mut response_model = if let Some(output) = &action.output {
        if !output.fields.is_empty() {
            format!("{}{}Response", entity_name, crate::codegen::python::models::to_pascal_case(action_name))
        } else {
            entity_name.to_string()
        }
    } else {
        "dict".to_string()
    };

    // Determine if it should be a list
    let returns_list = matches!(method, crate::ast::HttpMethod::Get) && !path.contains('{');
    if returns_list && response_model != "dict" {
        response_model = format!("List[{}]", response_model);
    }

    content.push_str(&format!("@router.{}(\"{}\", response_model={})\n", method_str, relative_path, response_model));
    
    // Build parameters
    let mut params = Vec::new();
    let mut call_params = Vec::new();

    for segment in path.split('/') {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len()-1];
            params.push(format!("{}: str", param_name));
            call_params.push(param_name.to_string());
        }
    }

    if matches!(method, crate::ast::HttpMethod::Post | crate::ast::HttpMethod::Put | crate::ast::HttpMethod::Patch) {
        let has_input = action.input.as_ref().map(|i| !i.fields.is_empty()).unwrap_or(false);
        if has_input {
             let request_model = format!("{}Request", crate::codegen::python::models::to_pascal_case(action_name));
             params.push(format!("data: {}", request_model));
        } else {
             params.push(format!("data: {}Create", entity_name));
        }
        call_params.push("data".to_string());
    }

    params.push("db: Session = Depends(get_db)".to_string());
    call_params.push("db".to_string());

    let requires_auth = action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. }));
    if requires_auth {
        params.push("current_user: dict = Depends(get_current_user_token)".to_string());
        call_params.push("current_user".to_string());
    }

    content.push_str(&format!("async def {}({}):\n", action_name, params.join(", ")));
    content.push_str(&format!("    \"\"\"Handle {} action\"\"\"\n", action_name));

    // Policy Check
    let policy_check = generate_policy_enforcement(action, ast, "None").unwrap_or_default();
    content.push_str(&policy_check);

    content.push_str(&format!("    return {0}_service.{1}({2})\n\n", entity_lower, action_name, call_params.join(", ")));
    
    content
}


fn generate_policy_enforcement(action: &Action, ast: &IntentFile, target_var: &str) -> CompileResult<String> {
    let mut content = String::new();
    
    for decorator in &action.decorators {
        if let Decorator::Policy(name) = decorator {
            let policy = ast.policies.iter().find(|p| p.name == *name);

            if let Some(_p) = policy {
                 let func_name = format!("check_{}", name);

                let resource_arg = if target_var == "None" {
                    ""
                } else {
                    &format!(", resource={}", target_var)
                };

                content.push_str(&format!("    # Enforce policy: {}\n", name));
                content.push_str(&format!("    {}(user=current_user{})\n", func_name, resource_arg));
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
            "from controllers.{}_controller import router as {}_router\n",
            name_lower, name_lower
        ));
    }
    
    content.push_str("\n__all__ = [\n");
    for entity in &ast.entities {
        let name_lower = entity.name.to_lowercase();
        content.push_str(&format!("    \"{}_router\",\n", name_lower));
    }
    content.push_str("]\n");
    
    content
}
