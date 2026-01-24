// Intent Compiler - FastAPI Route Aggregator
// Collects and includes routers from all controllers

use std::fs;
use std::path::Path;

use crate::ast::IntentFile;
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate FastAPI routes aggregator
pub fn generate_routes(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    let mut content = String::new();

    // Imports
    content.push_str("# Intent Compiler Generated FastAPI Routes Aggregator\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from fastapi import APIRouter\n");
    content.push_str("from controllers import *\n\n");
    
    content.push_str("router = APIRouter()\n\n");

    // Include router from each controller
    for entity in &ast.entities {
        let name_lower = entity.name.to_lowercase();
        content.push_str(&format!("router.include_router({}_router)\n", name_lower));
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
