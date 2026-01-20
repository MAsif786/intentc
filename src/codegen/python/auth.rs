// Intent Compiler - Authentication Logic Generator
// Generates JWT-based security utilities

use std::fs;
use std::path::Path;

use crate::ast::IntentFile;
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate authentication and security utilities
pub fn generate_auth_utils(_ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    
    let content = r#"# Intent Compiler Generated Security Utilities
# Generated automatically - do not edit

from datetime import datetime, timedelta
from typing import Any, Union, Optional

import jwt
from passlib.context import CryptContext
from fastapi import Depends, HTTPException, status
from fastapi.security import OAuth2PasswordBearer
from sqlalchemy.orm import Session

from db.database import settings, get_db

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
"#;

    let path = output_dir.join("core/security.py");
    let lines = content.lines().count();
    fs::write(&path, content)?;
    result.add_file("core/security.py", lines);

    // Also generate a simple auth routes file for login/token
    let auth_routes_content = r#"# Intent Compiler Generated Auth Routes
from datetime import timedelta
from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.security import OAuth2PasswordRequestForm
from sqlalchemy.orm import Session

from db.database import get_db, settings
from core.security import create_access_token

router = APIRouter()

@router.post("/login")
async def login_for_access_token(form_data: OAuth2PasswordRequestForm = Depends()):
    # Note: This is a placeholder for login logic. 
    # In a real app, you would verify against your User model.
    # For now, it accepts any login with password 'password' for testing.
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
"#;

    let auth_path = output_dir.join("api/auth.py");
    let auth_lines = auth_routes_content.lines().count();
    fs::write(&auth_path, auth_routes_content)?;
    result.add_file("api/auth.py", auth_lines);

    Ok(result)
}
