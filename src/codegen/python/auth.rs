// Intent Compiler - Security & Auth Generator
// Generates core/security.py with JWT and password hashing logic

use std::fs;
use std::path::Path;

use crate::ast::{Decorator, IntentFile};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate security and authentication logic
pub fn generate_security(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();
    let mut content = String::new();

    content.push_str("# Intent Compiler Generated Security & Auth\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import os\n");
    content.push_str("from datetime import datetime, timedelta, timezone\n");
    content.push_str("from typing import Optional, Any\n");
    content.push_str("import jwt\n");
    content.push_str("from passlib.context import CryptContext\n");
    content.push_str("from fastapi import Depends, HTTPException, status\n");
    content.push_str("from fastapi.security import OAuth2PasswordBearer\n\n");
    content.push_str("from db.database import settings\n");
    content.push_str("from sqlalchemy.orm import Session\n");
    content.push_str("from db.database import get_db\n\n");

    content.push_str("# Password hashing setup\n");
    content.push_str("pwd_context = CryptContext(schemes=[\"bcrypt\"], deprecated=\"auto\")\n");
    content.push_str("oauth2_scheme = OAuth2PasswordBearer(tokenUrl=\"login\")\n\n");

    content.push_str("def verify_password(plain_password, hashed_password):\n");
    content.push_str("    return pwd_context.verify(plain_password, hashed_password)\n\n");

    content.push_str("def get_password_hash(password):\n");
    content.push_str("    return pwd_context.hash(password)\n\n");

    content.push_str("def create_access_token(data: dict, expires_delta: Optional[timedelta] = None):\n");
    content.push_str("    to_encode = data.copy()\n");
    content.push_str("    if expires_delta:\n");
    content.push_str("        expire = datetime.now(timezone.utc) + expires_delta\n");
    content.push_str("    else:\n");
    content.push_str("        expire = datetime.now(timezone.utc) + timedelta(minutes=settings.access_token_expire_minutes)\n");
    content.push_str("    to_encode.update({\"exp\": expire})\n");
    content.push_str("    encoded_jwt = jwt.encode(to_encode, settings.secret_key, algorithm=settings.algorithm)\n");
    content.push_str("    return encoded_jwt\n\n");

    content.push_str("async def get_current_user_token(token: str = Depends(oauth2_scheme)):\n");
    content.push_str("    credentials_exception = HTTPException(\n");
    content.push_str("        status_code=status.HTTP_401_UNAUTHORIZED,\n");
    content.push_str("        detail=\"Could not validate credentials\",\n");
    content.push_str("        headers={\"WWW-Authenticate\": \"Bearer\"},\n");
    content.push_str("    )\n");
    content.push_str("    try:\n");
    content.push_str("        payload = jwt.decode(token, settings.secret_key, algorithms=[settings.algorithm])\n");
    content.push_str("        email: str = payload.get(\"sub\")\n");
    content.push_str("        if email is None:\n");
    content.push_str("            raise credentials_exception\n");
    content.push_str("        return {\"email\": email}\n");
    content.push_str("    except jwt.PyJWTError:\n");
    content.push_str("        raise credentials_exception\n\n");

    // Generate entity-specific current user dependencies
    let mut entities_with_auth = std::collections::HashSet::new();
    for action in &ast.actions {
        for decorator in &action.decorators {
            if let Decorator::Auth { name: Some(entity_name), .. } = decorator {
                let first_char = entity_name.chars().next().unwrap_or(' ');
                if first_char.is_uppercase() {
                    entities_with_auth.insert(entity_name.clone());
                }
            }
        }
    }

    // Always include User if it exists
    if ast.find_entity("User").is_some() {
        entities_with_auth.insert("User".to_string());
    }

    for entity_name in entities_with_auth {
        let name_lower = entity_name.to_lowercase();
        content.push_str(&format!(
            "async def get_current_{}(token: str = Depends(oauth2_scheme), db: Session = Depends(get_db)):\n",
            name_lower
        ));
        content.push_str("    credentials_exception = HTTPException(\n");
        content.push_str("        status_code=status.HTTP_401_UNAUTHORIZED,\n");
        content.push_str("        detail=\"Could not validate credentials\",\n");
        content.push_str("        headers={\"WWW-Authenticate\": \"Bearer\"},\n");
        content.push_str("    )\n");
        content.push_str("    try:\n");
        content.push_str("        payload = jwt.decode(token, settings.secret_key, algorithms=[settings.algorithm])\n");
        content.push_str("        email: str = payload.get(\"sub\")\n");
        content.push_str("        if email is None:\n");
        content.push_str("            raise credentials_exception\n");
        content.push_str("    except jwt.PyJWTError:\n");
        content.push_str("        raise credentials_exception\n\n");
        content.push_str(&format!(
            "    from db.models import {}Model\n",
            entity_name
        ));
        content.push_str(&format!(
            "    user = db.query({}Model).filter({}Model.email == email).first()\n",
            entity_name, entity_name
        ));
        content.push_str("    if user is None:\n");
        content.push_str("        raise credentials_exception\n");
        content.push_str("    return user\n\n");
    }

    let path = output_dir.join("core/security.py");
    let lines = content.lines().count();
    fs::write(&path, &content)?;
    result.add_file("core/security.py", lines);

    // Generate core/__init__.py
    let init_path = output_dir.join("core/__init__.py");
    fs::write(&init_path, "# Generated by Intent Compiler\nfrom . import security\n")?;
    result.add_file("core/__init__.py", 2);

    Ok(result)
}
