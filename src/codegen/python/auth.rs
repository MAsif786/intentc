// Intent Compiler - Authentication Logic Generator
// Generates JWT-based security utilities

use std::fs;
use std::path::Path;

use crate::ast::IntentFile;
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate authentication and security utilities
pub fn generate_auth_utils(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    
    // Collect unique entity names used in @auth decorators
    let mut auth_entities = std::collections::HashSet::new();
    for action in &ast.actions {
        for decorator in &action.decorators {
            if let crate::ast::Decorator::Auth { name: Some(entity_name), .. } = decorator {
                auth_entities.insert(entity_name.clone());
            }
        }
    }

    let mut content = String::new();
    content.push_str(r#"# Intent Compiler Generated Security Utilities
# Generated automatically - do not edit

from datetime import datetime, timedelta
from typing import Any, Union, Optional

import jwt
from passlib.context import CryptContext
from fastapi import Depends, HTTPException, status
from fastapi.security import OAuth2PasswordBearer
from sqlalchemy.orm import Session

from db.database import settings, get_db
from db.models import *

pwd_context = CryptContext(schemes=["bcrypt"], deprecated="auto")
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="login")


def verify_password(plain_password: str, hashed_password: str) -> bool:
    """Verify a plain password against a hashed one"""
    return pwd_context.verify(plain_password, hashed_password)


def get_password_hash(password: str) -> str:
    """Generate a hash from a plain password"""
    return pwd_context.hash(password)


def create_access_token(data: dict, expires_delta: Optional[timedelta] = None) -> str:
    """Create a new JWT access token"""
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.utcnow() + expires_delta
    else:
        expire = datetime.utcnow() + timedelta(minutes=settings.access_token_expire_minutes)
    
    to_encode.update({"exp": expire})
    encoded_jwt = jwt.encode(to_encode, settings.secret_key, algorithm=settings.algorithm)
    return encoded_jwt


async def get_current_user_token(token: str = Depends(oauth2_scheme)) -> dict:
    """Validate token and return payload"""
    credentials_exception = HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    try:
        payload = jwt.decode(token, settings.secret_key, algorithms=[settings.algorithm])
        username: str = payload.get("sub")
        if username is None:
            raise credentials_exception
        return payload
    except jwt.PyJWTError:
        raise credentials_exception
"#);

    // Generate specific dependencies for each auth entity
    for entity_name in auth_entities {
        let model_name = format!("{}Model", entity_name);
        let func_name = format!("get_current_{}", entity_name.to_lowercase());
        
        content.push_str(&format!(r#"

async def {func_name}(
    db: Session = Depends(get_db),
    token_data: dict = Depends(get_current_user_token)
) -> {model_name}:
    """Fetch {entity_name} from DB using token data"""
    username = token_data.get("sub")
    if not username:
        raise HTTPException(status_code=401, detail="Invalid token payload")
    
    # In v0.1 we assume 'sub' is the primary key (id)
    result = db.query({model_name}).filter({model_name}.id == username).first()
    
    # Fallback: check if there's a username field if id lookup fails
    if not result:
        # We can dynamically check if the model has a username field in v0.1 mapping
        result = db.query({model_name}).filter(getattr({model_name}, 'username', None) == username).first()
    
    if not result:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="{entity_name} not found",
            headers={{"WWW-Authenticate": "Bearer"}},
        )
    return result
"#, 
        func_name = func_name,
        entity_name = entity_name,
        model_name = model_name
        ));
    }

    let path = output_dir.join("core/security.py");
    let lines = content.lines().count();
    fs::write(&path, &content)?;
    result.add_file("core/security.py", lines);

    // Also generate a simple auth routes file for login/token
    let mut auth_routes_content = String::new();
    auth_routes_content.push_str(r#"# Intent Compiler Generated Auth Routes
from datetime import timedelta
from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.security import OAuth2PasswordRequestForm
from sqlalchemy.orm import Session

from db.database import get_db, settings
from db.models import *
from core.security import create_access_token, verify_password

router = APIRouter()

@router.post("/login")
async def login_for_access_token(
    form_data: OAuth2PasswordRequestForm = Depends(),
    db: Session = Depends(get_db)
):
"#);

    // Try to find a User entity for login logic
    let auth_entity = if ast.find_entity("User").is_some() {
        Some("User")
    } else {
        // Fallback to first auth entity found
        let mut found = None;
        for action in &ast.actions {
            for decorator in &action.decorators {
                if let crate::ast::Decorator::Auth { name: Some(name), .. } = decorator {
                    found = Some(name.as_str());
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        found
    };

    if let Some(entity_name) = auth_entity {
        let model_name = format!("{}Model", entity_name);
        auth_routes_content.push_str(&format!(r#"    # Fetch user from database
    user = db.query({model_name}).filter({model_name}.email == form_data.username).first()
    if not user:
        # Fallback to username if email doesn't match or exist
        user = db.query({model_name}).filter(getattr({model_name}, 'username', None) == form_data.username).first()

    if not user or not verify_password(form_data.password, getattr(user, 'hashed_password', '')):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incorrect username or password",
            headers={{"WWW-Authenticate": "Bearer"}},
        )
    
    access_token_expires = timedelta(minutes=settings.access_token_expire_minutes)
    # Use user.id for sub as requested
    access_token = create_access_token(
        data={{"sub": str(user.id)}}, expires_delta=access_token_expires
    )
    return {{"access_token": access_token, "token_type": "bearer"}}
"#, model_name = model_name));
    } else {
        auth_routes_content.push_str(r#"    # Note: This is a placeholder for login logic (no User entity found)
    if form_data.password != "password":
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incorrect username or password",
            headers={"WWW-Authenticate": "Bearer"},
        )
    
    access_token_expires = timedelta(minutes=settings.access_token_expire_minutes)
    access_token = create_access_token(
        data={"sub": form_data.username}, expires_delta=access_token_expires
    )
    return {"access_token": access_token, "token_type": "bearer"}
"#);
    }

    let auth_path = output_dir.join("api/auth.py");
    let auth_lines = auth_routes_content.lines().count();
    fs::write(&auth_path, &auth_routes_content)?;
    result.add_file("api/auth.py", auth_lines);

    Ok(result)
}
