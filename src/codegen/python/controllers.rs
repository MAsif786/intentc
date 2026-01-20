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
    content.push_str("    _instance: Optional['");
    content.push_str(&format!("{}Controller'] = None\n\n", name));
    
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
    content.push_str(&format!("        self.service = {}_service\n", name_lower));
    content.push_str("        self._initialized = True\n\n");
    
    // CRUD methods
    content.push_str(&generate_crud_controller_methods(name, &name_lower));
    
    // Action-specific methods
    for action in &ast.actions {
        if let Some(output) = &action.output {
            if output.entity == *name {
                content.push_str(&generate_action_controller_method(action, name));
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

fn generate_action_controller_method(action: &Action, entity_name: &str) -> String {
    let mut content = String::new();
    let action_name = &action.name;
    
    // Get HTTP method and check if auth required
    let mut _has_auth = false;
    for decorator in &action.decorators {
        if matches!(decorator, Decorator::Auth { .. }) {
            _has_auth = true;
        }
    }
    
    // Check return type based on action
    let has_find = action.process.as_ref().map(|p| {
        p.derives.iter().any(|d| {
            matches!(&d.value, crate::ast::DeriveValue::FunctionCall { name, .. } if name == "find")
        })
    }).unwrap_or(false);
    
    if has_find {
        // Login-style action returns dict
        content.push_str(&format!("    async def {}(self, db: Session, data) -> dict:\n", action_name));
        content.push_str(&format!("        \"\"\"Handle {} action\"\"\"\n", action_name));
        content.push_str(&format!("        return self.service.{}(db, data)\n\n", action_name));
    } else {
        // Create-style action returns model
        content.push_str(&format!("    async def {}(self, db: Session, data) -> {}Model:\n", action_name, entity_name));
        content.push_str(&format!("        \"\"\"Handle {} action\"\"\"\n", action_name));
        content.push_str(&format!("        return self.service.{}(db, data)\n\n", action_name));
    }
    
    content
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
