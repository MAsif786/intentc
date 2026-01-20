// Intent Compiler - Python Code Generator
// Generates FastAPI + SQLAlchemy + Pydantic Python code

mod models;
mod orm;
mod api;
mod rules;
mod migrations;
mod tests;
mod auth;

use std::fs;
use std::path::Path;

use crate::ast::IntentFile;
use crate::codegen::{CodeGenerator, GenerationResult, TargetLanguage};
use crate::error::CompileResult;

/// Python code generator
pub struct PythonGenerator {
    /// Whether to generate tests
    pub generate_tests: bool,
}

impl PythonGenerator {
    pub fn new() -> Self {
        Self {
            generate_tests: true,
        }
    }

    /// Create output directory structure
    fn create_directories(&self, output_dir: &Path) -> CompileResult<()> {
        let dirs = [
            "models",
            "db",
            "db/migrations",
            "db/migrations/versions",
            "api",
            "core",
            "logic",
            "tests",
        ];

        for dir in dirs {
            fs::create_dir_all(output_dir.join(dir))?;
        }

        Ok(())
    }

    /// Generate requirements.txt
    fn generate_requirements(&self, output_dir: &Path) -> CompileResult<usize> {
        let content = r#"# Intent Compiler Generated Requirements
# Generated automatically - do not edit

# Web framework
fastapi>=0.104.0
uvicorn[standard]>=0.24.0
python-multipart>=0.0.6

# Database
sqlalchemy>=2.0.0
alembic>=1.12.0

# Validation
pydantic>=2.5.0
pydantic-settings>=2.1.0

# Utilities
python-dotenv>=1.0.0
pyjwt>=2.8.0
passlib[bcrypt]>=1.7.4

# Testing
pytest>=7.4.0
pytest-asyncio>=0.21.0
httpx>=0.25.0
"#;

        let path = output_dir.join("requirements.txt");
        fs::write(&path, content)?;
        Ok(content.lines().count())
    }

    /// Generate main.py entry point
    fn generate_main(&self, _ast: &IntentFile, output_dir: &Path) -> CompileResult<usize> {
        let mut content = String::new();
        
        content.push_str(r#"# Intent Compiler Generated Entry Point
# Generated automatically - do not edit

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from db.models import Base
from db.database import engine
from api import routes, auth

# Create database tables
Base.metadata.create_all(bind=engine)

# Initialize FastAPI app
app = FastAPI(
    title="Intent Compiler Generated API",
    description="API generated from Intent Definition Language",
    version="0.1.0",
)

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routes
app.include_router(routes.router)
app.include_router(auth.router, tags=["auth"])


@app.get("/")
async def root():
    return {"message": "Intent Compiler Generated API", "status": "running"}


@app.get("/health")
async def health():
    return {"status": "healthy"}


if __name__ == "__main__":
    import uvicorn
    import argparse
    import os
    from dotenv import load_dotenv

    load_dotenv()

    parser = argparse.ArgumentParser(description="Run the Intent Compiler Generated API")
    parser.add_argument("--host", default=os.getenv("HOST", "0.0.0.0"), help="Host to bind to")
    parser.add_argument("--port", type=int, default=int(os.getenv("PORT", 8000)), help="Port to bind to")
    args = parser.parse_args()

    uvicorn.run(app, host=args.host, port=args.port)
"#);

        let path = output_dir.join("main.py");
        let lines = content.lines().count();
        fs::write(&path, content)?;
        Ok(lines)
    }

    /// Generate database configuration
    fn generate_database_config(&self, output_dir: &Path) -> CompileResult<usize> {
        let content = r#"# Intent Compiler Generated Database Configuration
# Generated automatically - do not edit

from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    database_url: str = "sqlite:///./app.db"
    secret_key: str = "09d25e094faa6ca2556c818166b7a9563b93f7099f6f0f4caa6cf63b88e8d3e7"
    algorithm: str = "HS256"
    access_token_expire_minutes: int = 30
    
    class Config:
        env_file = ".env"


settings = Settings()

engine = create_engine(
    settings.database_url,
    connect_args={"check_same_thread": False} if "sqlite" in settings.database_url else {}
)

SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


def get_db():
    """Dependency to get database session"""
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
"#;

        let path = output_dir.join("db/database.py");
        let lines = content.lines().count();
        fs::write(&path, content)?;
        Ok(lines)
    }

    /// Generate __init__.py files
    fn generate_init_files(&self, output_dir: &Path) -> CompileResult<()> {
        let dirs = ["models", "db", "api", "logic", "tests"];
        
        for dir in dirs {
            let path = output_dir.join(dir).join("__init__.py");
            fs::write(&path, "# Generated by Intent Compiler\n")?;
        }
        
        Ok(())
    }

    /// Generate .env.example
    fn generate_env_example(&self, output_dir: &Path) -> CompileResult<()> {
        let content = r#"# Intent Compiler Generated Environment Variables
# Copy this to .env and customize

DATABASE_URL=sqlite:///./app.db
# DATABASE_URL=postgresql://user:password@localhost/dbname
"#;

        fs::write(output_dir.join(".env.example"), content)?;
        Ok(())
    }
}

impl Default for PythonGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for PythonGenerator {
    fn generate(&self, ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
        let mut result = GenerationResult::new();

        // Create directory structure
        self.create_directories(output_dir)?;

        // Generate __init__.py files
        self.generate_init_files(output_dir)?;

        // Generate requirements.txt
        let lines = self.generate_requirements(output_dir)?;
        result.add_file("requirements.txt", lines);

        // Generate main.py
        let lines = self.generate_main(ast, output_dir)?;
        result.add_file("main.py", lines);

        // Generate database config
        let lines = self.generate_database_config(output_dir)?;
        result.add_file("db/database.py", lines);

        // Generate .env.example
        self.generate_env_example(output_dir)?;
        result.add_file(".env.example", 4);

        // Generate Pydantic models
        let models_result = models::generate_models(ast, output_dir)?;
        result.merge(models_result);

        // Generate SQLAlchemy ORM models
        let orm_result = orm::generate_orm_models(ast, output_dir)?;
        result.merge(orm_result);

        // Generate FastAPI routes
        let api_result = api::generate_routes(ast, output_dir)?;
        result.merge(api_result);

        // Generate authentication utils
        let auth_result = auth::generate_auth_utils(ast, output_dir)?;
        result.merge(auth_result);

        // Generate business rules
        let rules_result = rules::generate_rules(ast, output_dir)?;
        result.merge(rules_result);

        // Generate migrations
        let migrations_result = migrations::generate_migrations(ast, output_dir)?;
        result.merge(migrations_result);

        // Generate tests
        if self.generate_tests {
            let tests_result = tests::generate_tests(ast, output_dir)?;
            result.merge(tests_result);
        }

        Ok(result)
    }

    fn language(&self) -> TargetLanguage {
        TargetLanguage::Python
    }

    fn file_extension(&self) -> &str {
        "py"
    }
}
