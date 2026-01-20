// Intent Compiler - Python Repository Generator
// Generates repository classes with CRUD operations

use crate::ast::IntentFile;
use crate::codegen::GenerationResult;
use crate::error::CompileResult;
use std::fs;
use std::path::Path;

pub fn generate_repositories(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();

    // Create repositories directory
    let repos_dir = output_dir.join("repositories");
    fs::create_dir_all(&repos_dir)?;

    // Generate base repository
    let base_content = generate_base_repository();
    let base_path = repos_dir.join("base.py");
    fs::write(&base_path, &base_content)?;
    result.add_file("repositories/base.py", base_content.lines().count());

    // Generate entity-specific repositories
    for entity in &ast.entities {
        let content = generate_entity_repository(entity);
        let filename = format!("{}_repository.py", entity.name.to_lowercase());
        let path = repos_dir.join(&filename);
        fs::write(&path, &content)?;
        result.add_file(format!("repositories/{}", filename), content.lines().count());
    }

    // Generate __init__.py
    let init_content = generate_repositories_init(ast);
    let init_path = repos_dir.join("__init__.py");
    fs::write(&init_path, &init_content)?;
    result.add_file("repositories/__init__.py", init_content.lines().count());

    Ok(result)
}

fn generate_base_repository() -> String {
    r#"# Intent Compiler Generated Base Repository
# Generated automatically - do not edit

from typing import TypeVar, Generic, Optional, Type
from sqlalchemy.orm import Session
import uuid

T = TypeVar('T')


class BaseRepository(Generic[T]):
    """Base repository with CRUD operations"""
    
    def __init__(self, model: Type[T]):
        self.model = model
    
    def create(self, db: Session, data: dict) -> T:
        """Create a new record"""
        if 'id' not in data:
            data['id'] = str(uuid.uuid4())
        db_obj = self.model(**data)
        db.add(db_obj)
        db.commit()
        db.refresh(db_obj)
        return db_obj
    
    def get_by_id(self, db: Session, id: str) -> Optional[T]:
        """Get a record by ID"""
        return db.query(self.model).filter(self.model.id == id).first()
    
    def get_all(self, db: Session, skip: int = 0, limit: int = 100) -> list[T]:
        """Get all records with pagination"""
        return db.query(self.model).offset(skip).limit(limit).all()
    
    def update(self, db: Session, id: str, data: dict) -> Optional[T]:
        """Update a record by ID"""
        db_obj = self.get_by_id(db, id)
        if db_obj:
            for key, value in data.items():
                if hasattr(db_obj, key):
                    setattr(db_obj, key, value)
            db.commit()
            db.refresh(db_obj)
        return db_obj
    
    def delete(self, db: Session, id: str) -> bool:
        """Delete a record by ID"""
        db_obj = self.get_by_id(db, id)
        if db_obj:
            db.delete(db_obj)
            db.commit()
            return True
        return False
    
    def find_by(self, db: Session, **filters) -> Optional[T]:
        """Find a single record by filters"""
        query = db.query(self.model)
        for key, value in filters.items():
            if hasattr(self.model, key):
                query = query.filter(getattr(self.model, key) == value)
        return query.first()
    
    def find_all_by(self, db: Session, **filters) -> list[T]:
        """Find all records matching filters"""
        query = db.query(self.model)
        for key, value in filters.items():
            if hasattr(self.model, key):
                query = query.filter(getattr(self.model, key) == value)
        return query.all()
    
    def exists(self, db: Session, **filters) -> bool:
        """Check if a record exists"""
        return self.find_by(db, **filters) is not None
    
    def count(self, db: Session) -> int:
        """Count all records"""
        return db.query(self.model).count()
"#.to_string()
}

fn generate_entity_repository(entity: &crate::ast::Entity) -> String {
    let name = &entity.name;
    let name_lower = name.to_lowercase();
    
    format!(r#"# Intent Compiler Generated Repository
# Generated automatically - do not edit

from typing import Optional
from sqlalchemy.orm import Session

from db.models import {name}Model
from repositories.base import BaseRepository


class {name}Repository(BaseRepository[{name}Model]):
    """Repository for {name} entity"""
    
    _instance: Optional['{name}Repository'] = None
    
    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._initialized = False
        return cls._instance
    
    def __init__(self):
        if self._initialized:
            return
        super().__init__({name}Model)
        self._initialized = True


# Singleton instance
{name_lower}_repository = {name}Repository()
"#, name = name, name_lower = name_lower)
}

fn generate_repositories_init(ast: &IntentFile) -> String {
    let mut content = String::new();
    content.push_str("# Intent Compiler Generated Repositories\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("from repositories.base import BaseRepository\n");
    
    for entity in &ast.entities {
        let name_lower = entity.name.to_lowercase();
        content.push_str(&format!(
            "from repositories.{}_repository import {}Repository, {}_repository\n",
            name_lower, entity.name, name_lower
        ));
    }
    
    content.push_str("\n__all__ = [\n");
    content.push_str("    \"BaseRepository\",\n");
    for entity in &ast.entities {
        content.push_str(&format!("    \"{}Repository\",\n", entity.name));
        content.push_str(&format!("    \"{}_repository\",\n", entity.name.to_lowercase()));
    }
    content.push_str("]\n");
    
    content
}
