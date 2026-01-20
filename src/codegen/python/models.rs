// Intent Compiler - Pydantic Model Generator
// Generates Pydantic models from entity definitions

use std::fs;
use std::path::Path;

use crate::ast::{Entity, FieldType, Decorator, IntentFile};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate Pydantic models for all entities
pub fn generate_models(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();

    for entity in &ast.entities {
        let (content, lines) = generate_entity_model(entity)?;
        let filename = format!("{}.py", entity.name.to_lowercase());
        let path = output_dir.join("models").join(&filename);
        fs::write(&path, &content)?;
        result.add_file(format!("models/{}", filename), lines);
    }

    // Generate request models for actions
    let (req_content, req_lines) = generate_action_requests(ast)?;
    if req_lines > 0 {
        let req_path = output_dir.join("models/requests.py");
        fs::write(&req_path, &req_content)?;
        result.add_file("models/requests.py", req_lines);
    }

    // Generate models/__init__.py with all exports
    let init_content = generate_models_init(ast);
    let init_path = output_dir.join("models/__init__.py");
    fs::write(&init_path, &init_content)?;

    Ok(result)
}

/// Generate a single entity model
fn generate_entity_model(entity: &Entity) -> CompileResult<(String, usize)> {
    let mut content = String::new();

    // Imports
    content.push_str("# Intent Compiler Generated Pydantic Model\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from datetime import datetime\n");
    content.push_str("from typing import Optional, Literal\n");
    content.push_str("from pydantic import BaseModel, Field\n\n\n");

    // Generate enum classes for enum fields
    for field in &entity.fields {
        if let FieldType::Enum(values) = &field.field_type {
            content.push_str(&format!(
                "# Enum for {} field\n",
                field.name
            ));
            content.push_str(&format!(
                "{}Type = Literal[{}]\n\n",
                capitalize(&field.name),
                values.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(", ")
            ));
        }
    }

    // Base model (for creation)
    content.push_str(&format!("class {}Base(BaseModel):\n", entity.name));
    content.push_str("    model_config = {\"extra\": \"forbid\"}\n");
    content.push_str("    \"\"\"Base model with common fields\"\"\"\n");
    
    let mut has_fields = false;
    for field in &entity.fields {
        // Skip primary key in base model
        if field.decorators.contains(&Decorator::Primary) {
            continue;
        }
        has_fields = true;
        content.push_str(&generate_field_line(field));
    }
    
    if !has_fields {
        content.push_str("    pass\n");
    }
    content.push_str("\n\n");

    // Create model
    content.push_str(&format!("class {}Create({}Base):\n", entity.name, entity.name));
    content.push_str("    \"\"\"Model for creating new records\"\"\"\n");
    content.push_str("    pass\n\n\n");

    // Update model (all fields optional)
    content.push_str(&format!("class {}Update(BaseModel):\n", entity.name));
    content.push_str("    model_config = {\"extra\": \"forbid\"}\n");
    content.push_str("    \"\"\"Model for updating records (all fields optional)\"\"\"\n");
    
    let mut has_update_fields = false;
    for field in &entity.fields {
        if field.decorators.contains(&Decorator::Primary) {
            continue;
        }
        has_update_fields = true;
        content.push_str(&generate_optional_field_line(field));
    }
    
    if !has_update_fields {
        content.push_str("    pass\n");
    }
    content.push_str("\n\n");

    // Full model (includes ID)
    content.push_str(&format!("class {}({}Base):\n", entity.name, entity.name));
    content.push_str("    \"\"\"Full model with all fields including ID\"\"\"\n");
    
    // Add primary key field
    for field in &entity.fields {
        if field.decorators.contains(&Decorator::Primary) {
            content.push_str(&generate_field_line(field));
        }
    }
    content.push_str("\n");
    content.push_str("    model_config = {\n");
    content.push_str("        \"from_attributes\": True,\n");
    content.push_str("        \"extra\": \"forbid\"\n");
    content.push_str("    }\n");

    let lines = content.lines().count();
    Ok((content, lines))
}

/// Generate a field line for Pydantic model
fn generate_field_line(field: &crate::ast::Field) -> String {
    let python_type = field_type_to_python(&field.field_type);
    let is_optional = field.decorators.contains(&Decorator::Optional);
    
    // Get default value if any
    let default = field.decorators.iter().find_map(|d| {
        if let Decorator::Default(val) = d {
            Some(val.clone())
        } else {
            None
        }
    });

    let type_str = if is_optional {
        format!("Optional[{}]", python_type)
    } else {
        python_type
    };

    match default {
        Some(val) => {
            let default_val = format_default_value(&val, &field.field_type);
            format!("    {}: {} = {}\n", field.name, type_str, default_val)
        }
        None if is_optional => {
            format!("    {}: {} = None\n", field.name, type_str)
        }
        None => {
            format!("    {}: {}\n", field.name, type_str)
        }
    }
}

/// Generate an optional field line for update model
fn generate_optional_field_line(field: &crate::ast::Field) -> String {
    let python_type = field_type_to_python(&field.field_type);
    format!("    {}: Optional[{}] = None\n", field.name, python_type)
}

/// Convert IDL field type to Python type string
fn field_type_to_python(field_type: &FieldType) -> String {
    match field_type {
        FieldType::String => "str".to_string(),
        FieldType::Number => "float".to_string(),
        FieldType::Boolean => "bool".to_string(),
        FieldType::DateTime => "datetime".to_string(),
        FieldType::Enum(values) => {
            format!("Literal[{}]", values.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(", "))
        }
        FieldType::Reference(name) => format!("\"{}\"", name),
        FieldType::Array(inner) => format!("list[{}]", field_type_to_python(inner)),
        FieldType::Optional(inner) => format!("Optional[{}]", field_type_to_python(inner)),
    }
}

/// Format a default value for Python
fn format_default_value(value: &str, field_type: &FieldType) -> String {
    match field_type {
        FieldType::String => {
            if value == "now" {
                "Field(default_factory=datetime.now)".to_string()
            } else {
                format!("\"{}\"", value)
            }
        }
        FieldType::Number => value.to_string(),
        FieldType::Boolean => {
            if value == "true" { "True" } else { "False" }.to_string()
        }
        FieldType::DateTime => {
            if value == "now" {
                "Field(default_factory=datetime.now)".to_string()
            } else {
                format!("\"{}\"", value)
            }
        }
        _ => format!("\"{}\"", value),
    }
}

/// Generate models/__init__.py
fn generate_models_init(ast: &IntentFile) -> String {
    let mut content = String::new();
    content.push_str("# Intent Compiler Generated Models\n");
    content.push_str("# Generated automatically - do not edit\n\n");

    for entity in &ast.entities {
        let module = entity.name.to_lowercase();
        content.push_str(&format!(
            "from .{} import {}, {}Create, {}Update\n",
            module, entity.name, entity.name, entity.name
        ));
    }

    // Import request models
    let requests_exist = ast.actions.iter().any(|a| !a.params.is_empty());
    if requests_exist {
        content.push_str("from .requests import *\n");
    }

    content.push_str("\n__all__ = [\n");
    for entity in &ast.entities {
        content.push_str(&format!("    \"{}\",\n", entity.name));
        content.push_str(&format!("    \"{}Create\",\n", entity.name));
        content.push_str(&format!("    \"{}Update\",\n", entity.name));
    }
    content.push_str("]\n");

    content
}

/// Generate request models for actions
fn generate_action_requests(ast: &IntentFile) -> CompileResult<(String, usize)> {
    let mut content = String::new();
    let mut count = 0;

    content.push_str("# Intent Compiler Generated Request Models\n");
    content.push_str("from typing import Optional, List, Literal\n");
    content.push_str("from datetime import datetime\n");
    content.push_str("from pydantic import BaseModel, Field\n\n\n");

    for action in &ast.actions {
        if action.params.is_empty() {
            continue;
        }

        count += 1;
        let model_name = format!("{}Request", capitalize(&action.name));
        content.push_str(&format!("class {}(BaseModel):\n", model_name));
        content.push_str("    model_config = {\"extra\": \"forbid\"}\n");
        
        for param in &action.params {
             let python_type = field_type_to_python(&param.param_type);
             content.push_str(&format!("    {}: {}\n", param.name, python_type));
        }
        content.push_str("\n\n");
    }

    if count == 0 {
        return Ok((String::new(), 0));
    }

    let lines = content.lines().count();
    Ok((content, lines))
}

/// Capitalize first letter
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
