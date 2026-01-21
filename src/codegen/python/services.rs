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
    content.push_str("    _instance: Optional['");
    content.push_str(&format!("{}Service'] = None\n\n", name));
    
    // Singleton pattern
    content.push_str("    def __new__(cls):\n");
    content.push_str("        if cls._instance is None:\n");
    content.push_str("            cls._instance = super().__new__(cls)\n");
    content.push_str("            cls._instance._initialized = False\n");
    content.push_str("        return cls._instance\n\n");
    
    // Init
    content.push_str("    def __init__(self):\n");
    content.push_str("        if self._initialized:\n");
    content.push_str("            return\n");
    content.push_str(&format!("        self.repo = {}_repository\n", name_lower));
    content.push_str("        self._initialized = True\n\n");
    
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
    
    // Check if this is a  select-based action (login-style)
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
        content.push_str(&format!("    def {}(self, db: Session, data) -> dict:\n", action_name));
        content.push_str(&format!("        \"\"\"Business logic for {}\"\"\"\n", action_name));
        
        // Process derives
        if let Some(process) = &action.process {
            for derive in &process.derives {
                match &derive.value {
                    DeriveValue::Select { entity, .. } => {
                        // Simplified: just get first record
                        content.push_str(&format!("        {} = db.query({}Model).first()\n", derive.name, entity));
                        content.push_str(&format!("        if not {}:\n", derive.name));
                        content.push_str("            raise HTTPException(status_code=400, detail=\"Not found\")\n");
                    }
                    DeriveValue::Compute { function, args: _ } if function == "verify_hash" => {
                        // Simplified verify hash
                        content.push_str("        if not verify_password(data.password, user.password_hash):\n");
                        content.push_str("            raise HTTPException(status_code=400, detail=\"Invalid credentials\")\n");
                    }
                    DeriveValue::SystemCall { namespace, capability, .. } if namespace == "jwt" && capability == "create" => {
                        content.push_str(&format!("        {} = create_access_token(data={{\"sub\": user.email}})\n", derive.name));
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
                    content.push_str(&format!("            \"{}\": user.{},\n", field, field));
                }
            }
        }
        content.push_str("        }\n\n");
    } else if has_hash {
        // Signup-style method
        content.push_str(&format!("    def {}(self, db: Session, data) -> {}Model:\n", action_name, entity_name));
        content.push_str(&format!("        \"\"\"Business logic for {}\"\"\"\n", action_name));
        content.push_str("        data_dict = data.model_dump()\n");
        
        // Process password hashing
        if let Some(input) = &action.input {
            for param in &input.fields {
                for dec in &param.decorators {
                    if let Decorator::Map { target, transform } = dec {
                        if matches!(transform, MapTransform::Hash) {
                            content.push_str(&format!("        # Hash {} -> {}\n", param.name, target));
                            content.push_str(&format!("        data_dict['{}'] = get_password_hash(data_dict.pop('{}'))\n", target, param.name));
                        }
                    }
                }
            }
        }
        
        content.push_str("        return self.repo.create(db, data_dict)\n\n");
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
