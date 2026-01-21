// Intent Compiler - FastAPI Route Generator
// Generates FastAPI routes from action definitions

use std::fs;
use std::path::Path;

use crate::ast::{Action, Decorator, HttpMethod, IntentFile, MapTransform, DeriveValue, LiteralValue};
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
        } else if let Some(return_type) = &returns {
            let body_model = if matches!(method, HttpMethod::Post) {
                format!("{}Create", return_type)
            } else {
                format!("{}Update", return_type)
            };
            params.push(format!("data: {}", body_model));
        }
    } else if !has_api {
        // Internal actions take input as keyword arguments
        // We include = Depends() for optional params if they match some patterns?
        // Actually, for Depends() support, we can just let FastAPI handle it.
        if let Some(input) = &action.input {
            for field in &input.fields {
                params.push(format!("{}: {}", field.name, field.param_type.to_python_type()));
            }
        }
    }

    // Add database dependency
    params.push("db: Session = Depends(get_db)".to_string());

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
                // Entity-based auth
                params.push(format!(
                    "current_user: {}Model = Depends(get_current_{})", 
                    name, name.to_lowercase()
                ));
            } else {
                // Action/Rule-based auth
                // If the user specified arguments @auth(validate_user(id)), 
                // we'll use a lambda wrapper to explicitly map the values.
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
            // Default token-based auth
            params.push("current_user: dict = Depends(get_current_user_token)".to_string());
        }
    }

    let params_str = params.join(", ");
    
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
    
    // Generate appropriate body based on HTTP method or as default for internal
    if !has_api {
        // Default body for internal actions: just return Success or the output type
        if let Some(return_type) = &returns {
             content.push_str(&format!("    # Internal logic for {}\n", action.name));
             content.push_str(&format!("    return db.query({}Model).first()\n", return_type));
        } else {
             content.push_str("    return {\"status\": \"ok\"}\n");
        }
    } else {
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
                    
                    // Policy Check
                    content.push_str(&generate_policy_enforcement(action, _ast, "result")?);

                    content.push_str("    return result\n");
                } else {
                    content.push_str("    return {\"message\": \"Success\"}\n");
                }
            } else {
                // GET list
                if let Some(return_type) = &returns {
                    let model_name = format!("{}Model", return_type);
                    
                    // Policy Check (Global/Pre-check)
                    content.push_str(&generate_policy_enforcement(action, _ast, "None")?);

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
                
                // Check if process section has a select - indicates query action not create
                let has_find = action.process.as_ref().map(|p| {
                    p.derives.iter().any(|d| {
                        matches!(&d.value, DeriveValue::Select { .. })
                    })
                }).unwrap_or(false);
                
                if has_find {
                    // Query-based action (e.g., login)
                    content.push_str("    # Query-based action\n");
                    
                    // Process derives in order - select returns the entity, compute checks it, etc.
                    if let Some(process) = &action.process {
                        for derive in &process.derives {
                            match &derive.value {
                                DeriveValue::Select { entity, predicate } => {
                                    let py_code = select_to_python(entity, predicate);
                                    content.push_str(&format!("    {} = {}\n", derive.name, py_code));
                                    content.push_str(&format!("    if not {}:\n", derive.name));
                                    content.push_str("        raise HTTPException(status_code=400, detail=\"Not found\")\n");
                                }
                                DeriveValue::Compute { function, args } if function == "verify_hash" => {
                                    let py_code = compute_to_python(function, args);
                                    content.push_str(&format!("    if not {}:\n", py_code));
                                    content.push_str("        raise HTTPException(status_code=400, detail=\"Invalid credentials\")\n");
                                }
                                DeriveValue::SystemCall { namespace, capability, args } => {
                                    let py_code = system_call_to_python(namespace, capability, args);
                                    content.push_str(&format!("    {} = {}\n", derive.name, py_code));
                                }
                                DeriveValue::Compute { function, args } => {
                                    let py_code = compute_to_python(function, args);
                                    content.push_str(&format!("    {} = {}\n", derive.name, py_code));
                                }
                                _ => {
                                    let val_expr = derive_value_to_python(&derive.value);
                                    content.push_str(&format!("    {} = {}\n", derive.name, val_expr));
                                }
                            }
                        }
                    }
                    
                    // Return the found entity with derived fields
                    content.push_str("    return {\n");
                    if let Some(output) = &action.output {
                        for field in &output.fields {
                            // Check if field is a derive name (like token) or entity field
                            let is_derived = action.process.as_ref().map(|p| {
                                p.derives.iter().any(|d| d.name == *field)
                            }).unwrap_or(false);
                            
                            if is_derived {
                                content.push_str(&format!("        \"{}\": {},\n", field, field));
                            } else {
                                // Assume it's from the found entity (first derive with select)
                                let found_var = action.process.as_ref()
                                    .and_then(|p| p.derives.iter().find(|d| {
                                        matches!(&d.value, DeriveValue::Select { .. })
                                    }))
                                    .map(|d| d.name.clone())
                                    .unwrap_or_else(|| "user".to_string());
                                content.push_str(&format!("        \"{}\": {}.{},\n", field, found_var, field));
                            }
                        }
                    }
                    content.push_str("    }\n");
                } else {
                    // Standard create action
                    content.push_str("    # Generate UUID for new record\n");
                    content.push_str("    data_dict = data.model_dump()\n");
                    content.push_str("    data_dict['id'] = str(uuid.uuid4())\n");
                    
                    // Process parameter logic (mapping and hashing)
                    let has_input = action.input.as_ref().map(|i| !i.fields.is_empty()).unwrap_or(false);
                    if has_input {
                        if let Some(input) = &action.input {
                            for param in &input.fields {
                                let mut target_name = param.name.clone();
                                let mut needs_hash = false;

                                for dec in &param.decorators {
                                    if let Decorator::Map { target, transform } = dec {
                                        target_name = target.clone();
                                        if matches!(transform, MapTransform::Hash) {
                                            needs_hash = true;
                                        }
                                    }
                                }

                                // If we need to map or hash
                                if target_name != param.name || needs_hash {
                                    content.push_str(&format!("    # Transform {}\n", param.name));
                                    
                                    // Get value
                                    let val_expr = format!("data_dict.pop('{}')", param.name);
                                    let val_expr = if needs_hash {
                                        format!("get_password_hash({})", val_expr)
                                    } else {
                                        val_expr
                                    };

                                    content.push_str(&format!("    data_dict['{}'] = {}\n", target_name, val_expr));
                                }
                            }
                        }
                    }
                    
                    // Process section derivations - for create actions
                    if let Some(process) = &action.process {
                        for derive in &process.derives {
                            let val_expr = derive_value_to_python(&derive.value);
                            content.push_str(&format!("    data_dict['{}'] = {}\n", derive.name, val_expr));
                        }
                    }
                    
                    content.push_str(&format!(
                        "    db_obj = {}(**data_dict)\n",
                        model_name
                    ));
                    
                    // Policy Check (on the new object)
                    content.push_str(&generate_policy_enforcement(action, _ast, "db_obj")?);

                    content.push_str("    db.add(db_obj)\n");
                    content.push_str("    try:\n");
                    content.push_str("        db.commit()\n");
                    content.push_str("        db.refresh(db_obj)\n");
                    content.push_str("        return db_obj\n");
                    content.push_str("    except IntegrityError as e:\n");
                    content.push_str("        db.rollback()\n");
                    content.push_str("        raise HTTPException(status_code=400, detail=str(e.orig))\n");
                }
            } else {
                content.push_str("    return {\"message\": \"Created\"}\n");
            }
        },
        _ => {
            // Policy Check
            content.push_str(&generate_policy_enforcement(action, _ast, "None")?);
            content.push_str("    return {\"status\": \"ok\"}\n");
        }
    }
    }

    Ok(content)
}

/// Generate CRUD routes for an entity
fn generate_entity_crud_routes(entity: &crate::ast::Entity, ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    let entity_lower = entity.name.to_lowercase();
    let model_name = format!("{}Model", entity.name);

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
            "    return db.query({0}).offset(skip).limit(limit).all()\n\n\n",
            model_name
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
            "    result = db.query({0}).filter({0}.id == id).first()\n",
            model_name
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

fn derive_value_to_python(val: &DeriveValue) -> String {
    match val {
        DeriveValue::Literal(lit) => match lit {
            LiteralValue::String(s) => format!("\"{}\"", s),
            LiteralValue::Number(n) => n.to_string(),
            LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
        DeriveValue::Identifier(s) => s.clone(),
        DeriveValue::FieldAccess { path } => {
            if path.first().map(|s| s.as_str()) == Some("auth") {
                if let Some(field) = path.last() {
                    return format!("current_user.{}", field);
                }
            }
            path.join(".")
        }
        DeriveValue::Select { entity, predicate } => select_to_python(entity, predicate),
        DeriveValue::Compute { function, args } => compute_to_python(function, args),
        DeriveValue::SystemCall { namespace, capability, args } => system_call_to_python(namespace, capability, args),
    }
}


/// Generate policy enforcement code
fn generate_policy_enforcement(action: &Action, ast: &IntentFile, target_var: &str) -> CompileResult<String> {
    let mut content = String::new();
    
    for decorator in &action.decorators {
        if let Decorator::Policy(name) = decorator {
            // Find the policy
            let policy = if name.contains('.') {
                // Entity-scoped policy
                let parts: Vec<&str> = name.split('.').collect();
                let entity_name = parts[0];
                let policy_name = parts[1];
                
                ast.find_entity(entity_name)
                    .and_then(|e| e.policies.iter().find(|p| p.name == policy_name))
            } else {
                // Global policy
                ast.policies.iter().find(|p| p.name == *name)
            };


            if let Some(_p) = policy {
                 let func_name = if name.contains('.') {
                    let parts: Vec<&str> = name.split('.').collect();
                    format!("check_{}_{}", parts[0], parts[1])
                } else {
                    format!("check_{}", name)
                };

                // Determine resource argument
                let resource_arg = if target_var == "None" {
                    ""
                } else {
                    &format!(", resource={}", target_var)
                };

                content.push_str(&format!("    # Enforce policy: {}\n", name));
                content.push_str(&format!("    {}(user=current_user{})\n", func_name, resource_arg));
            } else {
                return Err(crate::error::CompileError::validation(
                    format!("Policy not found: {}", name),
                    action.location.clone()
                ));
            }
        }
    }
    
    Ok(content)
}

fn select_to_python(entity: &str, predicate: &crate::ast::Predicate) -> String {
    use crate::ast::{FieldReference, CompareOp};
    
    let right_str = match &predicate.value {
        FieldReference::InputField(name) => format!("data.{}", name),
        FieldReference::DerivedField { name, field } => format!("{}.{}", name, field),
        FieldReference::Literal(lit) => match lit {
            LiteralValue::String(s) => format!("\"{}\"", s),
            LiteralValue::Number(n) => n.to_string(),
            LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
    };
    
    let op_str = match predicate.operator {
        CompareOp::Equal => "==",
        CompareOp::NotEqual => "!=",
        CompareOp::Less => "<",
        CompareOp::Greater => ">",
    };
    
    // Determine filter field name (left side of predicate)
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
            LiteralValue::String(s) => format!("\"{}\"", s),
            LiteralValue::Number(n) => n.to_string(),
            LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
    }).collect();
    
    match function {
        "verify_hash" => {
            // verify_hash(input, target) -> verify_password(input, target)
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
            // Generic compute function
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
            LiteralValue::String(s) => format!("\"{}\"", s),
            LiteralValue::Number(n) => n.to_string(),
            LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
    }).collect();
    
    match (namespace, capability) {
        ("jwt", "create") => {
            // jwt.create(subject) -> create_access_token(data={"sub": subject})
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
            // Generic system call
            format!("{}_{}({})", namespace, capability, args_str.join(", "))
        }
    }
}
