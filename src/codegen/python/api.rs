// Intent Compiler - FastAPI Route Generator
// Generates FastAPI routes from action definitions

use std::fs;
use std::path::Path;

use crate::ast::{Action, Decorator, HttpMethod, IntentFile};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate FastAPI routes
pub fn generate_routes(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    let mut content = String::new();

    // Imports
    content.push_str("# Intent Compiler Generated FastAPI Routes\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import uuid\n");
    content.push_str("from uuid import UUID\n");
    content.push_str("from typing import List, Optional, Literal\n\n");
    content.push_str("from fastapi import APIRouter, Depends, HTTPException, status\n");
    content.push_str("from sqlalchemy.orm import Session\n");
    content.push_str("from sqlalchemy.exc import IntegrityError\n\n");
    content.push_str("from db.database import get_db\n");
    content.push_str("from db.models import *\n");
    content.push_str("from models import *\n");
    content.push_str("from logic.rules import *\n");
    content.push_str("from logic.policies import *\n");
    content.push_str("from controllers import *\n");
    
    // Collect unique auth dependencies to import
    let mut auth_deps = std::collections::HashSet::new();
    auth_deps.insert("get_password_hash".to_string());
    auth_deps.insert("get_current_user_token".to_string());
    for action in &ast.actions {
        for decorator in &action.decorators {
            if let Decorator::Auth { name: Some(entity_name), .. } = decorator {
                let first_char = entity_name.chars().next().unwrap_or(' ');
                if first_char.is_uppercase() {
                    auth_deps.insert(format!("get_current_{}", entity_name.to_lowercase()));
                }
            }
        }
    }
    let mut auth_deps_vec: Vec<_> = auth_deps.into_iter().collect();
    auth_deps_vec.sort(); // Consistent order
    let auth_imports = auth_deps_vec.join(", ");
    content.push_str(&format!("from core.security import {}, verify_password, create_access_token\n\n\n", auth_imports));
    
    content.push_str("router = APIRouter()\n\n\n");

    // Generate routes for each action
    for action in &ast.actions {
        let has_api = action.decorators.iter().any(|d| matches!(d, Decorator::Api { .. }));
        if has_api {
            content.push_str(&generate_route(action, ast)?);
            content.push_str("\n\n");
        } else {
            // Generate the function only (internal action)
            content.push_str(&generate_action_function(action, ast)?);
            content.push_str("\n\n");
        }
    }

    // Generate CRUD routes for each entity (convenience)
    for entity in &ast.entities {
        content.push_str(&generate_entity_crud_routes(entity, ast)?);
    }

    let lines = content.lines().count();
    let path = output_dir.join("api/routes.py");
    fs::write(&path, &content)?;
    result.add_file("api/routes.py", lines);

    // Generate __init__.py
    let init_content = "# Intent Compiler Generated API\nfrom . import routes\n";
    fs::write(output_dir.join("api/__init__.py"), init_content)?;

    Ok(result)
}

/// Generate a single route from an action
fn generate_route(action: &Action, ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();

    // Get API decorator
    let default_path = "/".to_string();
    let (method, path) = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, path } = d {
            Some((method, path))
        } else {
            None
        }
    }).unwrap_or((&HttpMethod::Get, &default_path));

    // Get return type from output section
    let returns = action.output.as_ref().map(|o| o.entity.clone());

    // Generate decorator
    let method_str = match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Patch => "patch",
        HttpMethod::Delete => "delete",
    };

    let response_type_str = if let Some(output) = &action.output {
        if !output.fields.is_empty() {
             // Use projected model name: EntityActionResponse
             // e.g., UserSignupResponse
             let model_name = format!("{}{}Response", output.entity, crate::codegen::python::models::to_pascal_case(&action.name));
             if matches!(method, HttpMethod::Get) && !path.contains('{') {
                 Some(format!("List[{}]", model_name))
             } else {
                 Some(model_name)
             }
        } else {
             // Full entity return
             if matches!(method, HttpMethod::Get) && !path.contains('{') {
                 returns.as_ref().map(|r| format!("List[{}]", r))
             } else {
                 returns.clone()
             }
        }
    } else {
        None
    };

    let response_model = response_type_str.as_ref()
        .map(|r| format!(", response_model={}", r))
        .unwrap_or_default();

    content.push_str(&format!(
        "@router.{}(\"{}\"{})\n",
        method_str, path, response_model
    ));

    // Generate function
    content.push_str(&generate_action_function(action, ast)?);

    Ok(content)
}

/// Generate the Python function for an action
fn generate_action_function(action: &Action, _ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    let func_name = action.name.clone();

    // Get return type from output section
    let returns = action.output.as_ref().map(|o| o.entity.clone());
    
    // Determine if auth is required
    let requires_auth = action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. }));

    // Get API decorator for path params if any
    let default_path = "/".to_string();
    let (_, path) = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, path } = d {
            Some((method, path))
        } else {
            None
        }
    }).unwrap_or((&HttpMethod::Get, &default_path));

    // Build parameters
    let mut params = Vec::new();
    let mut call_params = Vec::new();
    
    // Add path parameters
    for segment in path.split('/') {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len()-1];
            // Find the type from action params
            let param_type = action.input.as_ref()
                .and_then(|i| i.fields.iter().find(|p| p.name == param_name))
                .map(|p| p.param_type.to_python_type())
                .unwrap_or_else(|| "str".to_string());
            params.push(format!("{}: {}", param_name, param_type));
            call_params.push(param_name.to_string());
        }
    }

    // Add body parameter for POST/PUT/PATCH (if it has @api)
    let has_api = action.decorators.iter().any(|d| matches!(d, Decorator::Api { .. }));
    let method = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, .. } = d { Some(method) } else { None }
    }).unwrap_or(&HttpMethod::Get);

    if has_api && matches!(method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch) {
        // If action has input fields, use generated Request model
        let has_input = action.input.as_ref().map(|i| !i.fields.is_empty()).unwrap_or(false);
        if has_input {
             let request_model = format!("{}Request", crate::codegen::python::models::to_pascal_case(&action.name));
             params.push(format!("data: {}", request_model));
             call_params.push("data".to_string());
        } else if let Some(return_type) = &returns {
            let body_model = if matches!(method, HttpMethod::Post) {
                format!("{}Create", return_type)
            } else {
                format!("{}Update", return_type)
            };
            params.push(format!("data: {}", body_model));
            call_params.push("data".to_string());
        }
    } else if !has_api {
        if let Some(input) = &action.input {
            for field in &input.fields {
                params.push(format!("{}: {}", field.name, field.param_type.to_python_type()));
                call_params.push(field.name.clone());
            }
        }
    }

    // Add database dependency
    params.push("db: Session = Depends(get_db)".to_string());
    call_params.push("db".to_string());

    // Add auth dependency if required
    if requires_auth {
        // Find if a specific entity or action is requested
        let (auth_name, auth_args) = action.decorators.iter().find_map(|d| {
            if let Decorator::Auth { name, args } = d {
                Some((name.as_ref().map(|s| s.clone()), args.clone()))
            } else {
                None
            }
        }).unwrap_or((None, Vec::new()));

        if let Some(name) = auth_name {
            let first_char = name.chars().next().unwrap_or(' ');
            if first_char.is_uppercase() {
                params.push(format!(
                    "current_user: {}Model = Depends(get_current_{})", 
                    name, name.to_lowercase()
                ));
            } else {
                if auth_args.is_empty() {
                    params.push(format!("current_user = Depends({})", name));
                } else {
                    let lambda_args = auth_args.join(", ");
                    let call_args = auth_args.iter().map(|a| format!("{0}={0}", a)).collect::<Vec<_>>().join(", ");
                    params.push(format!(
                        "current_user = Depends(lambda {}: {}({}))", 
                        lambda_args, name, call_args
                    ));
                }
            }
        } else {
            params.push("current_user: dict = Depends(get_current_user_token)".to_string());
        }
        call_params.push("current_user".to_string());
    }

    let params_str = params.join(", ");
    let call_params_str = call_params.join(", ");
    
    // Return type annotation
    let response_type_str = if let Some(output) = &action.output {
        if !output.fields.is_empty() {
             let model_name = format!("{}{}Response", output.entity, crate::codegen::python::models::to_pascal_case(&action.name));
             if matches!(method, HttpMethod::Get) && !path.contains('{') && has_api {
                 Some(format!("List[{}]", model_name))
             } else {
                 Some(model_name)
             }
        } else {
             if matches!(method, HttpMethod::Get) && !path.contains('{') && has_api {
                 returns.as_ref().map(|r| format!("List[{}]", r))
             } else {
                 returns.clone()
             }
        }
    } else {
        None
    };

    let return_annotation = response_type_str.as_ref()
        .map(|r| format!(" -> {}", r))
        .unwrap_or_else(|| " -> dict".to_string());

    content.push_str(&format!(
        "async def {}({}){}:\n",
        func_name, params_str, return_annotation
    ));

    // Generate function body
    content.push_str(&format!("    \"\"\"{} action\"\"\"\n", action.name));
    
    // Find appropriate controller
    let controller_var = if let Some(output) = &action.output {
        format!("{}_controller", output.entity.to_lowercase())
    } else {
        // Default to a generic controller or the first one?
        // Actually, if it has no output entity, it might be a global action.
        // For now, let's assume it's a global controller or handle it.
        "global_controller".to_string()
    };

    content.push_str(&format!(
        "    return await {}.{}({})\n",
        controller_var, func_name, call_params_str
    ));

    Ok(content)
}

/// Generate CRUD routes for an entity
fn generate_entity_crud_routes(entity: &crate::ast::Entity, ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    let entity_lower = entity.name.to_lowercase();
    let controller_var = format!("{}_controller", entity_lower);

    // Helper to check if a route already exists
    let route_exists = |method: HttpMethod, path_suffix: &str| -> bool {
        let expected_path = format!("/{}{}", entity_lower, path_suffix); // e.g. /users or /users/{id}
        ast.actions.iter().any(|a| {
            a.decorators.iter().any(|d| {
                if let Decorator::Api { method: m, path: p } = d {
                    m == &method && (p == &expected_path || p == &format!("/{}s{}", entity_lower, path_suffix))
                } else {
                    false
                }
            })
        })
    };

    // List all
    if !route_exists(HttpMethod::Get, "s") {
        content.push_str(&format!(
            "# Auto-generated CRUD for {}\n",
            entity.name
        ));
        
        content.push_str(&format!(
            "@router.get(\"/{0}s\", response_model=List[{1}])\n",
            entity_lower, entity.name
        ));
        content.push_str(&format!(
            "async def list_{0}s(skip: int = 0, limit: int = 100, db: Session = Depends(get_db)) -> List[{1}]:\n",
            entity_lower, entity.name
        ));
        content.push_str(&format!(
            "    \"\"\"List all {0}s\"\"\"\n",
            entity_lower
        ));
        content.push_str(&format!(
            "    return await {0}.list(db, skip=skip, limit=limit)\n\n\n",
            controller_var
        ));
    }

    // Get by ID
    if !route_exists(HttpMethod::Get, "s/{id}") {
        content.push_str(&format!(
            "@router.get(\"/{0}s/{{id}}\", response_model={1})\n",
            entity_lower, entity.name
        ));
        content.push_str(&format!(
            "async def get_{0}(id: str, db: Session = Depends(get_db)) -> {1}:\n",
            entity_lower, entity.name
        ));
        content.push_str(&format!(
            "    \"\"\"Get {0} by ID\"\"\"\n",
            entity_lower
        ));
        content.push_str(&format!(
            "    result = await {0}.get(db, id)\n",
            controller_var
        ));
        content.push_str("    if not result:\n");
        content.push_str(&format!(
            "        raise HTTPException(status_code=404, detail=\"{} not found\")\n",
            entity.name
        ));
        content.push_str("    return result\n\n\n");
    }

    Ok(content)
}
