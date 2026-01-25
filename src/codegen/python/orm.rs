// Intent Compiler - SQLAlchemy ORM Generator
// Generates SQLAlchemy models from entity definitions

use std::fs;
use std::path::Path;

use crate::ast::{Entity, FieldType, Decorator, IntentFile};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate SQLAlchemy ORM models
pub fn generate_orm_models(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    let mut content = String::new();

    // Imports
    content.push_str("# Intent Compiler Generated SQLAlchemy Models\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import uuid\n");
    content.push_str("from datetime import datetime\n");
    content.push_str("from typing import Optional\n\n");
    content.push_str("from sqlalchemy import Column, String, Float, Boolean, DateTime, Enum, ForeignKey, Integer\n");
    content.push_str("from sqlalchemy.orm import DeclarativeBase, relationship\n\n\n");
    
    // Base class
    content.push_str("class Base(DeclarativeBase):\n");
    content.push_str("    \"\"\"Base class for all ORM models\"\"\"\n");
    content.push_str("    pass\n\n\n");

    // Generate each entity as a SQLAlchemy model
    for entity in &ast.entities {
        content.push_str(&generate_orm_model(entity, ast)?);
        content.push_str("\n\n");
    }

    let lines = content.lines().count();
    let path = output_dir.join("db/models.py");
    fs::write(&path, &content)?;
    result.add_file("db/models.py", lines);

    Ok(result)
}

/// Generate a single ORM model
fn generate_orm_model(entity: &Entity, _ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    let table_name = entity.name.to_lowercase() + "s"; // Simple pluralization

    content.push_str(&format!("class {}Model(Base):\n", entity.name));
    content.push_str(&format!("    \"\"\"SQLAlchemy model for {}\"\"\"\n", entity.name));
    content.push_str(&format!("    __tablename__ = \"{}\"\n\n", table_name));

    // Generate columns
    for field in &entity.fields {
        content.push_str(&generate_column(field)?);
    }

    // Generate relationships for foreign keys
    for field in &entity.fields {
        if let FieldType::Reference(ref_entity) = &field.field_type {
            let relationship_name = ref_entity.to_lowercase();
            content.push_str(&format!(
                "    {} = relationship(\"{}Model\", back_populates=\"{}s\")\n",
                relationship_name, ref_entity, entity.name.to_lowercase()
            ));
        }
    }

    // Add repr method
    content.push_str("\n");
    content.push_str("    def __repr__(self):\n");
    
    // Find the primary key field
    let pk_field = entity.fields.iter()
        .find(|f| f.decorators.contains(&Decorator::Primary))
        .map(|f| &f.name)
        .unwrap_or(&entity.fields[0].name);
    
    content.push_str(&format!(
        "        return f\"<{}({}={{self.{}}})\"\n",
        entity.name, pk_field, pk_field
    ));

    Ok(content)
}

/// Generate a SQLAlchemy column definition
fn generate_column(field: &crate::ast::Field) -> CompileResult<String> {
    let is_primary = field.decorators.contains(&Decorator::Primary);
    let is_unique = field.decorators.contains(&Decorator::Unique);
    let is_optional = field.decorators.contains(&Decorator::Optional);
    let is_index = field.decorators.contains(&Decorator::Index);
    
    let default = field.decorators.iter().find_map(|d| {
        if let Decorator::Default(val) = d {
            Some(val.clone())
        } else {
            None
        }
    });

    let column_type = field_type_to_sqlalchemy(&field.field_type);
    
    let mut options = Vec::new();
    
    if is_primary {
        options.push("primary_key=True".to_string());
    }
    if is_unique {
        options.push("unique=True".to_string());
    }
    if is_optional || !is_primary {
        let nullable_val = if is_optional { "True" } else { "False" };
        options.push(format!("nullable={}", nullable_val));
    }
    if is_index {
        options.push("index=True".to_string());
    }
    if let Some(val) = default {
        let default_value = format_default(&val, &field.field_type);
        options.push(format!("default={}", default_value));
    }

    let options_str = if options.is_empty() {
        String::new()
    } else {
        format!(", {}", options.join(", "))
    };

    Ok(format!(
        "    {} = Column({}{})\n",
        field.name, column_type, options_str
    ))
}

/// Convert IDL field type to SQLAlchemy column type
fn field_type_to_sqlalchemy(field_type: &FieldType) -> String {
    match field_type {
        FieldType::String => "String(255)".to_string(),
        FieldType::Number => "Float".to_string(),
        FieldType::Boolean => "Boolean".to_string(),
        FieldType::DateTime => "DateTime".to_string(),
        FieldType::Uuid => "String(36)".to_string(),  // UUID stored as 36-char string
        FieldType::Email => "String(255)".to_string(), // Email as string
        FieldType::Enum(values) => {
            let enum_values = values.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(", ");
            format!("Enum({})", enum_values)
        }
        FieldType::Reference(name) => {
            format!("ForeignKey(\"{}.id\")", name.to_lowercase() + "s")
        }
        FieldType::Ref(name) => {
            format!("ForeignKey(\"{}.id\")", name.to_lowercase() + "s")
        }
        FieldType::Array(_) => "String".to_string(), // Store as JSON string
        FieldType::List(_) => "String".to_string(),  // Store as JSON string
        FieldType::Optional(inner) => field_type_to_sqlalchemy(inner),
    }
}

/// Format default value for SQLAlchemy
fn format_default(value: &str, field_type: &FieldType) -> String {
    match field_type {
        FieldType::String => format!("\"{}\"", value),
        FieldType::Number => value.to_string(),
        FieldType::Boolean => {
            if value == "true" { "True" } else { "False" }.to_string()
        }
        FieldType::DateTime => {
            if value == "now" {
                "datetime.now".to_string()
            } else {
                format!("\"{}\"", value)
            }
        }
        FieldType::Uuid => {
            if value == "uuid" {
                "lambda: str(uuid.uuid4())".to_string()
            } else {
                format!("\"{}\"", value)
            }
        }
        _ => format!("\"{}\"", value),
    }
}
