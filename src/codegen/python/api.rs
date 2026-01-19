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
    content.push_str("from typing import List, Optional\n\n");
    content.push_str("from fastapi import APIRouter, Depends, HTTPException, status\n");
    content.push_str("from sqlalchemy.orm import Session\n\n");
    content.push_str("from db.database import get_db\n");
    content.push_str("from db.models import *\n");
    content.push_str("from models import *\n");
    content.push_str("from logic.rules import *\n\n\n");
    
    content.push_str("router = APIRouter()\n\n\n");

    // Generate routes for each action
    for action in &ast.actions {
        content.push_str(&generate_route(action, ast)?);
        content.push_str("\n\n");
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
fn generate_route(action: &Action, _ast: &IntentFile) -> CompileResult<String> {
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

    // Get return type
    let returns = action.decorators.iter().find_map(|d| {
        if let Decorator::Returns(type_name) = d {
            Some(type_name.clone())
        } else {
            None
        }
    });

    // Determine if auth is required
    let _requires_auth = action.decorators.contains(&Decorator::Auth);

    // Generate decorator
    let method_str = match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Patch => "patch",
        HttpMethod::Delete => "delete",
    };

    let response_model = returns.as_ref()
        .map(|r| format!(", response_model={}", r))
        .unwrap_or_default();

    content.push_str(&format!(
        "@router.{}(\"{}\"{})\n",
        method_str, path, response_model
    ));

    // Generate function signature
    let func_name = action.name.clone();
    
    // Build parameters
    let mut params = Vec::new();
    
    // Add path parameters
    for segment in path.split('/') {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len()-1];
            // Find the type from action params
            let param_type = action.params.iter()
                .find(|p| p.name == param_name)
                .map(|p| p.param_type.to_python_type())
                .unwrap_or_else(|| "str".to_string());
            params.push(format!("{}: {}", param_name, param_type));
        }
    }

    // Add body parameter for POST/PUT/PATCH
    if matches!(method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch) {
        if let Some(return_type) = &returns {
            // Use Create model for POST, Update for PATCH/PUT
            let body_model = if matches!(method, HttpMethod::Post) {
                format!("{}Create", return_type)
            } else {
                format!("{}Update", return_type)
            };
            params.push(format!("data: {}", body_model));
        }
    }

    // Add database dependency
    params.push("db: Session = Depends(get_db)".to_string());

    let params_str = params.join(", ");
    
    // Return type annotation
    let return_annotation = returns.as_ref()
        .map(|r| format!(" -> {}", r))
        .unwrap_or_else(|| " -> dict".to_string());

    content.push_str(&format!(
        "async def {}({}){}:\n",
        func_name, params_str, return_annotation
    ));

    // Generate function body
    content.push_str(&format!("    \"\"\"{} endpoint\"\"\"\n", action.name));
    
    // Generate appropriate body based on HTTP method
    match method {
        HttpMethod::Get => {
            if path.contains('{') {
                // GET single item
                if let Some(return_type) = &returns {
                    let model_name = format!("{}Model", return_type);
                    let id_param = path.split('/').find(|s| s.starts_with('{')).map(|s| &s[1..s.len()-1]).unwrap_or("id");
                    content.push_str(&format!(
                        "    result = db.query({}).filter({}.id == {}).first()\n",
                        model_name, model_name, id_param
                    ));
                    content.push_str("    if not result:\n");
                    content.push_str("        raise HTTPException(status_code=404, detail=\"Not found\")\n");
                    content.push_str("    return result\n");
                } else {
                    content.push_str("    return {\"message\": \"Success\"}\n");
                }
            } else {
                // GET list
                if let Some(return_type) = &returns {
                    let model_name = format!("{}Model", return_type);
                    content.push_str(&format!(
                        "    return db.query({}).all()\n",
                        model_name
                    ));
                } else {
                    content.push_str("    return {\"message\": \"Success\"}\n");
                }
            }
        }
        HttpMethod::Post => {
            if let Some(return_type) = &returns {
                let model_name = format!("{}Model", return_type);
                content.push_str("    # Generate UUID for new record\n");
                content.push_str("    data_dict = data.model_dump()\n");
                content.push_str("    data_dict['id'] = str(uuid.uuid4())\n");
                content.push_str(&format!(
                    "    db_obj = {}(**data_dict)\n",
                    model_name
                ));
                content.push_str("    db.add(db_obj)\n");
                content.push_str("    db.commit()\n");
                content.push_str("    db.refresh(db_obj)\n");
                content.push_str("    return db_obj\n");
            } else {
                content.push_str("    return {\"message\": \"Created\"}\n");
            }
        }
        HttpMethod::Put | HttpMethod::Patch => {
            if let Some(return_type) = &returns {
                let model_name = format!("{}Model", return_type);
                let id_param = path.split('/').find(|s| s.starts_with('{')).map(|s| &s[1..s.len()-1]).unwrap_or("id");
                content.push_str(&format!(
                    "    db_obj = db.query({}).filter({}.id == {}).first()\n",
                    model_name, model_name, id_param
                ));
                content.push_str("    if not db_obj:\n");
                content.push_str("        raise HTTPException(status_code=404, detail=\"Not found\")\n");
                content.push_str("    update_data = data.model_dump(exclude_unset=True)\n");
                content.push_str("    for field, value in update_data.items():\n");
                content.push_str("        setattr(db_obj, field, value)\n");
                content.push_str("    db.commit()\n");
                content.push_str("    db.refresh(db_obj)\n");
                content.push_str("    return db_obj\n");
            } else {
                content.push_str("    return {\"message\": \"Updated\"}\n");
            }
        }
        HttpMethod::Delete => {
            if let Some(return_type) = &returns {
                let model_name = format!("{}Model", return_type);
                let id_param = path.split('/').find(|s| s.starts_with('{')).map(|s| &s[1..s.len()-1]).unwrap_or("id");
                content.push_str(&format!(
                    "    db_obj = db.query({}).filter({}.id == {}).first()\n",
                    model_name, model_name, id_param
                ));
                content.push_str("    if not db_obj:\n");
                content.push_str("        raise HTTPException(status_code=404, detail=\"Not found\")\n");
                content.push_str("    db.delete(db_obj)\n");
                content.push_str("    db.commit()\n");
                content.push_str("    return {\"message\": \"Deleted\"}\n");
            } else {
                content.push_str("    return {\"message\": \"Deleted\"}\n");
            }
        }
    }

    Ok(content)
}

/// Generate CRUD routes for an entity
fn generate_entity_crud_routes(entity: &crate::ast::Entity, _ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    let entity_lower = entity.name.to_lowercase();
    let model_name = format!("{}Model", entity.name);

    // List all
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
        "    return db.query({0}).offset(skip).limit(limit).all()\n\n\n",
        model_name
    ));

    // Get by ID
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
        "    result = db.query({0}).filter({0}.id == id).first()\n",
        model_name
    ));
    content.push_str("    if not result:\n");
    content.push_str(&format!(
        "        raise HTTPException(status_code=404, detail=\"{} not found\")\n",
        entity.name
    ));
    content.push_str("    return result\n\n\n");

    Ok(content)
}
