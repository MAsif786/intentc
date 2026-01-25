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
    for entity in &ast.entities {
        if entity.name != *name {
             // Check if this entity is used in any process step
             let is_used = ast.actions.iter()
                .filter(|a| a.output.as_ref().map(|o| o.entity == *name).unwrap_or(false))
                .any(|a| a.process.as_ref().map(|p| p.steps.iter().any(|s| match s {
                    crate::ast::ProcessStep::Mutate(m) => m.entity == entity.name,
                    crate::ast::ProcessStep::Delete(d) => d.entity == entity.name,
                    crate::ast::ProcessStep::Derive(d) => matches!(&d.value, DeriveValue::Select { entity: e, .. } if e == &entity.name),
                })).unwrap_or(false));
             
             if is_used {
                 content.push_str(&format!("from db.models import {}Model\n", entity.name));
             }
        }
    }
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
                content.push_str(&generate_action_method(action, name, &name_lower, ast));
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
    
    
    content
}

fn generate_action_method(action: &Action, entity_name: &str, _entity_lower: &str, ast: &IntentFile) -> String {
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

    // Check for implicit current_user usage in process
    let uses_current_user = action.process.as_ref().map(|p| {
        p.steps.iter().any(|s| match s {
             crate::ast::ProcessStep::Derive(d) => match &d.value {
                 DeriveValue::FieldAccess { path } => path[0] == "current_user",
                 DeriveValue::Identifier(id) => id.starts_with("current_user."),
                 _ => false
             },
             _ => false
        })
    }).unwrap_or(false);

    let requires_auth = action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. }));
    // Add current_user if explicit auth or implicit usage
    if requires_auth || uses_current_user {
        params.push("current_user".to_string());
    }

    let params_str = params.join(", ");
    let has_data = params.contains(&"data".to_string());
    let mut derived_vars = std::collections::HashSet::new();

    // Check if this is a process-based action
    let has_process = action.process.is_some();    
    // Check if this is a simple select-based action (no mutations/deletions)
    let has_find = action.process.as_ref().map(|p| {
        let has_select = p.steps.iter().any(|step| {
            if let crate::ast::ProcessStep::Derive(d) = step {
                matches!(&d.value, DeriveValue::Select { .. })
            } else {
                false
            }
        });
        let has_mutations = p.steps.iter().any(|step| {
            !matches!(step, crate::ast::ProcessStep::Derive(_))
        });
        has_select && !has_mutations
    }).unwrap_or(false);
    
    // Check for password hashing (signup-style)
    let has_hash = action.input.as_ref().map(|i| {
        i.fields.iter().any(|f| {
            f.decorators.iter().any(|d| matches!(d, Decorator::Map { transform: MapTransform::Hash, .. }))
        })
    }).unwrap_or(false);
    
    let returns_list = matches!(method, crate::ast::HttpMethod::Get) && !path.contains('{') && !action_name.starts_with("get_");
    
    if has_find {
        // Find/Select-style method (e.g. Login or List by filter)
        let return_type = if returns_list { "list[dict]" } else { "dict" };
        content.push_str(&format!("    def {}(self, {}) -> {}:\n", action_name, params_str, return_type));
        content.push_str(&format!("        \"\"\"Business logic for {}\"\"\"\n", action_name));
        
        // Process derives in order
        if let Some(process) = &action.process {
            for step in &process.steps {
                if let crate::ast::ProcessStep::Derive(derive) = step {
                match &derive.value {
                    DeriveValue::Select { entity, predicate } => {
                        let py_code = if returns_list {
                             format!("{}.all()", select_query_to_python(entity, predicate, has_data, &derived_vars))
                        } else {
                             format!("{}.first()", select_query_to_python(entity, predicate, has_data, &derived_vars))
                        };
                        content.push_str(&format!("        {} = {}\n", derive.name, py_code));
                        
                        if !returns_list {
                            content.push_str(&format!("        if not {}:\n", derive.name));
                            content.push_str("            raise HTTPException(status_code=400, detail=\"Not found\")\n");
                        }
                        derived_vars.insert(derive.name.clone());
                    }
                    DeriveValue::Compute { function, args } if function == "verify_hash" => {
                        let py_code = compute_to_python(function, args, has_data, &derived_vars);
                        content.push_str(&format!("        if not {}:\n", py_code));
                        content.push_str("            raise HTTPException(status_code=400, detail=\"Invalid credentials\")\n");
                    }
                    DeriveValue::SystemCall { namespace, capability, args } => {
                        let py_code = system_call_to_python(namespace, capability, args, has_data, &derived_vars);
                        content.push_str(&format!("        {} = {}\n", derive.name, py_code));
                        derived_vars.insert(derive.name.clone());
                    }
                    _ => {}
                }
                }
            }
        }
        
        // Return output
        if returns_list {
            // Find the plural variable from the select step
            let found_var = action.process.as_ref()
                .and_then(|p| p.steps.iter().find_map(|step| {
                    if let crate::ast::ProcessStep::Derive(d) = step {
                        if matches!(&d.value, DeriveValue::Select { .. }) {
                            Some(d.name.clone())
                        } else { None }
                    } else { None }
                }))
                .unwrap_or_else(|| "results".to_string());

            content.push_str(&format!("        return [\n            {{\n"));
            if let Some(output) = &action.output {
                for field in &output.fields {
                    // For lists, we assume fields belong to the items in found_var
                    content.push_str(&format!("                \"{}\": item.{},\n", field, field));
                }
            }
            content.push_str(&format!("            }} for item in {}\n        ]\n\n", found_var));
        } else {
            content.push_str("        return {\n");
            if let Some(output) = &action.output {
                for field in &output.fields {
                    let is_derived = action.process.as_ref().map(|p| {
                        p.steps.iter().any(|step| {
                            match step {
                                crate::ast::ProcessStep::Derive(d) => d.name == *field,
                                _ => false
                            }
                        })
                    }).unwrap_or(false);
                    
                    if is_derived {
                        content.push_str(&format!("            \"{}\": {},\n", field, field));
                    } else {
                        // Assume it's from the found entity (first derive with select)
                        let found_var = action.process.as_ref()
                            .and_then(|p| p.steps.iter().find_map(|step| {
                                if let crate::ast::ProcessStep::Derive(d) = step {
                                    if matches!(&d.value, DeriveValue::Select { .. }) {
                                        Some(d.name.clone())
                                    } else { None }
                                } else { None }
                            }))
                            .unwrap_or_else(|| "user".to_string());
                        content.push_str(&format!("            \"{}\": {}.{},\n", field, found_var, field));
                    }
                }
            }
            content.push_str("        }\n\n");
        }
    } else if has_process {
        // Generic Process-based method
        content.push_str(&format!("    def {}(self, {}) -> dict:\n", action_name, params_str));
        content.push_str(&format!("        \"\"\"Process execution for {}\"\"\"\n", action_name));

        if let Some(process) = &action.process {
            for step in &process.steps {
                match step {
                    crate::ast::ProcessStep::Derive(derive) => {
                         match &derive.value {
                            DeriveValue::Select { entity, predicate } => {
                                let py_code = select_to_python(entity, predicate, has_data, &derived_vars);
                                content.push_str(&format!("        {} = {}\n", derive.name, py_code));
                            }
                            DeriveValue::Compute { function, args } => {
                                let py_code = compute_to_python(function, args, has_data, &derived_vars);
                                content.push_str(&format!("        {} = {}\n", derive.name, py_code));
                            }
                            DeriveValue::SystemCall { namespace, capability, args } => {
                                let py_code = system_call_to_python(namespace, capability, args, has_data, &derived_vars);
                                content.push_str(&format!("        {} = {}\n", derive.name, py_code));
                            }
                            DeriveValue::Identifier(id) => {
                                 let val = resolve_identifier_python(id, has_data, &derived_vars);
                                 content.push_str(&format!("        {} = {}\n", derive.name, val));
                            }
                            DeriveValue::FieldAccess { path } => {
                                 let val = resolve_field_access_python(path, has_data, &derived_vars);
                                 content.push_str(&format!("        {} = {}\n", derive.name, val));
                            }
                            DeriveValue::Literal(lit) => {
                                 let val = match lit {
                                     crate::ast::LiteralValue::String(s) => format!("\"{}\"", s),
                                     crate::ast::LiteralValue::Number(n) => n.to_string(),
                                     crate::ast::LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
                                 };
                                 content.push_str(&format!("        {} = {}\n", derive.name, val));
                            }
                         }
                         derived_vars.insert(derive.name.clone());
                    }
                    crate::ast::ProcessStep::Mutate(mutate) => {
                        if let Some(predicate) = &mutate.predicate {
                            // Update mode: mutate Entity where <predicate>:
                            let query = select_query_to_python(&mutate.entity, predicate, has_data, &derived_vars);
                            content.push_str("        update_dict = {\n");
                            for setter in &mutate.setters {
                                let value_expr = derive_value_to_python(&setter.value, has_data, &derived_vars); 
                                content.push_str(&format!("            \"{}\": {},\n", setter.field, value_expr));
                            }
                            content.push_str("        }\n");
                            content.push_str("        update_dict = {k: v for k, v in update_dict.items() if v is not None}\n");
                            content.push_str(&format!("        {}.update(update_dict, synchronize_session=False)\n", query));
                        } else {
                            // Create mode: mutate Entity:
                            content.push_str(&format!("        new_{} = {}Model(\n", mutate.entity.to_lowercase(), mutate.entity));
                            for setter in &mutate.setters {
                                let value_expr = derive_value_to_python(&setter.value, has_data, &derived_vars); 
                                content.push_str(&format!("            {}={},\n", setter.field, value_expr));
                            }
                            content.push_str("        )\n");
                            content.push_str(&format!("        db.add(new_{})\n", mutate.entity.to_lowercase()));
                        }
                    }
                    crate::ast::ProcessStep::Delete(del) => {
                        let query = select_query_to_python(&del.entity, &del.predicate, has_data, &derived_vars);
                        content.push_str(&format!("        {}.delete()\n", query));
                    }
                }
            }
            content.push_str("        db.commit()\n");
        }

        // Return output
        let mut target_var = None;
        if let Some(_output) = &action.output {
             target_var = action.process.as_ref()
                .and_then(|p| p.steps.iter().filter_map(|step| {
                    match step {
                        crate::ast::ProcessStep::Derive(d) => Some(d.name.clone()),
                        crate::ast::ProcessStep::Mutate(m) => {
                             if m.predicate.is_none() { Some(format!("new_{}", m.entity.to_lowercase())) }
                             else { Some("resource".to_string()) }
                        },
                        _ => None
                    }
                }).last());

             if target_var == Some("resource".to_string()) {
                 content.push_str(&format!("        resource = self.repo.get_by_id(db, {})\n", if params.contains(&"id".to_string()) { "id" } else { "data.id" }));
             }
        }

        content.push_str("        return {\n");
        if let Some(output) = &action.output {
             if let Some(var) = target_var {
                 for field in &output.fields {
                     if derived_vars.contains(field) {
                         content.push_str(&format!("            \"{}\": {},\n", field, field));
                     } else {
                         content.push_str(&format!("            \"{}\": {}.{},\n", field, var, field));
                     }
                 }
             } else if action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. })) && output.entity == "User" {
                 // Fallback for self-updates like /profile
                 for field in &output.fields {
                     if derived_vars.contains(field) {
                         content.push_str(&format!("            \"{}\": {},\n", field, field));
                     } else {
                         content.push_str(&format!("            \"{}\": current_user.{},\n", field, field));
                     }
                 }
             } else {
                 content.push_str("            \"id\": \"done\",\n");
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
             content.push_str("        data_dict = data.model_dump()\n");
             
             // Check if entity has user_id or similar and set it from current_user
             let target_entity = ast.find_entity(entity_name);
             if let Some(entity) = target_entity {
                 for field in &entity.fields {
                     if (field.name == "user_id" || field.name == "owner_id" || field.name == format!("{}_id", entity.name.to_lowercase()))
                        && !action.input.as_ref().map(|i| i.fields.iter().any(|f| f.name == field.name)).unwrap_or(false) {
                         content.push_str(&format!("        if \"{}\" not in data_dict and \"current_user\" in locals():\n", field.name));
                         content.push_str(&format!("            data_dict[\"{}\"] = current_user.id\n", field.name));
                     }
                 }
             }

             content.push_str("        return self.repo.create(db, data_dict)\n\n");
        } else if matches!(method, crate::ast::HttpMethod::Get) {
             if path.contains('{') {
                 content.push_str("        result = self.repo.get_by_id(db, id)\n");
                 content.push_str("        if not result:\n");
                 content.push_str("            raise HTTPException(status_code=404, detail=\"Not found\")\n");
                 content.push_str("        return result\n\n");
             } else if action_name.starts_with("get_") && (requires_auth || uses_current_user) {
                 content.push_str("        return current_user\n\n");
             } else {
                 content.push_str("        return self.repo.get_all(db)\n\n");
             }
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

fn select_query_to_python(entity: &str, predicate: &crate::ast::Predicate, has_data: bool, derived_vars: &std::collections::HashSet<String>) -> String {
    use crate::ast::{FieldReference, CompareOp};
    
    let right_str = match &predicate.value {
        FieldReference::InputField(name) => resolve_identifier_python(name, has_data, derived_vars),
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
    
    format!("db.query({}Model).filter({}Model.{} {} {})", entity, entity, filter_field, op_str, right_str)
}

fn select_to_python(entity: &str, predicate: &crate::ast::Predicate, has_data: bool, derived_vars: &std::collections::HashSet<String>) -> String {
    format!("{}.first()", select_query_to_python(entity, predicate, has_data, derived_vars))
}

fn resolve_identifier_python(id: &str, has_data: bool, derived_vars: &std::collections::HashSet<String>) -> String {
     if derived_vars.contains(id) {
         id.to_string()
     } else if id.contains('.') {
        // Handle dotted access if passed as string
        let parts: Vec<&str> = id.split('.').collect();
        resolve_field_access_python(&parts.iter().map(|s| s.to_string()).collect::<Vec<_>>(), has_data, derived_vars)
    } else {
        if has_data {
            format!("data.{}", id)
        } else {
            id.to_string()
        }
    }
}

fn resolve_field_access_python(path: &[String], has_data: bool, derived_vars: &std::collections::HashSet<String>) -> String {
     if path[0] == "input" {
          if has_data {
              format!("data.{}", path[1])
          } else {
              path[1].clone()
          }
     } else if path[0] == "current_user" {
          format!("current_user.{}", path[1])
     } else if derived_vars.contains(&path[0]) {
          format!("{}.{}", path[0], path[1])
     } else {
          // Fallback
          format!("{}.{}", path[0], path[1])
     }
}

fn derive_value_to_python(value: &DeriveValue, has_data: bool, derived_vars: &std::collections::HashSet<String>) -> String {
    match value {
        DeriveValue::Literal(lit) => match lit {
            crate::ast::LiteralValue::String(s) => format!("\"{}\"", s),
            crate::ast::LiteralValue::Number(n) => n.to_string(),
            crate::ast::LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
        DeriveValue::Identifier(s) => resolve_identifier_python(s, has_data, derived_vars),
        DeriveValue::FieldAccess { path } => resolve_field_access_python(path, has_data, derived_vars),
        DeriveValue::Compute { function, args } => compute_to_python(function, args, has_data, derived_vars),
        _ => "None".to_string()
    }
}

fn compute_to_python(function: &str, args: &[crate::ast::FunctionArg], has_data: bool, derived_vars: &std::collections::HashSet<String>) -> String {
    use crate::ast::FunctionArg;
    
    let args_str: Vec<String> = args.iter().map(|arg| match arg {
        FunctionArg::TypeName(s) => s.clone(),
        FunctionArg::Identifier(s) => resolve_identifier_python(s, has_data, derived_vars),
        FunctionArg::FieldAccess { path } => resolve_field_access_python(path, has_data, derived_vars),
        FunctionArg::Literal(lit) => match lit {
            crate::ast::LiteralValue::String(s) => format!("\"{}\"", s),
            crate::ast::LiteralValue::Number(n) => n.to_string(),
            crate::ast::LiteralValue::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
        },
    }).collect();
    
    match function {
        "hash" => {
            if !args_str.is_empty() {
                format!("get_password_hash({})", args_str[0])
            } else {
                "\"\"".to_string()
            }
        }
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

fn system_call_to_python(namespace: &str, capability: &str, args: &[crate::ast::FunctionArg], has_data: bool, derived_vars: &std::collections::HashSet<String>) -> String {
    use crate::ast::FunctionArg;
    
    let args_str: Vec<String> = args.iter().map(|arg| match arg {
        FunctionArg::TypeName(s) => s.clone(),
        FunctionArg::Identifier(s) => resolve_identifier_python(s, has_data, derived_vars),
        FunctionArg::FieldAccess { path } => resolve_field_access_python(path, has_data, derived_vars),
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
