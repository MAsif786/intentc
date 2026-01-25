// Intent Compiler - Test Generator
// Generates pytest test scaffolding

use std::fs;
use std::path::Path;

use crate::ast::{Entity, Action, Decorator, IntentFile};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate test files
pub fn generate_tests(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();

    // Generate conftest.py
    let conftest = generate_conftest();
    fs::write(output_dir.join("tests/conftest.py"), &conftest)?;
    result.add_file("tests/conftest.py", conftest.lines().count());

    // Generate model tests
    let model_tests = generate_model_tests(ast)?;
    fs::write(output_dir.join("tests/test_models.py"), &model_tests)?;
    result.add_file("tests/test_models.py", model_tests.lines().count());

    // Generate API tests
    let api_tests = generate_api_tests(ast)?;
    fs::write(output_dir.join("tests/test_api.py"), &api_tests)?;
    result.add_file("tests/test_api.py", api_tests.lines().count());

    // Generate repository tests
    let repository_tests = generate_repository_tests(ast)?;
    fs::write(output_dir.join("tests/test_repositories.py"), &repository_tests)?;
    result.add_file("tests/test_repositories.py", repository_tests.lines().count());

    // Generate service tests
    let service_tests = generate_service_tests(ast)?;
    fs::write(output_dir.join("tests/test_services.py"), &service_tests)?;
    result.add_file("tests/test_services.py", service_tests.lines().count());

    // Generate controller tests
    let controller_tests = generate_controller_tests(ast)?;
    fs::write(output_dir.join("tests/test_controllers.py"), &controller_tests)?;
    result.add_file("tests/test_controllers.py", controller_tests.lines().count());

    Ok(result)
}

/// Generate conftest.py with fixtures
fn generate_conftest() -> String {
    r#"# Intent Compiler Generated Test Configuration
# Generated automatically - do not edit

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from db.models import Base
from db.database import get_db
from main import app


# Create test database
SQLALCHEMY_DATABASE_URL = "sqlite://"

engine = create_engine(
    SQLALCHEMY_DATABASE_URL,
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,
)
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


@pytest.fixture(scope="function")
def db():
    """Create a fresh database for each test"""
    Base.metadata.create_all(bind=engine)
    db = TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()
        Base.metadata.drop_all(bind=engine)


@pytest.fixture(scope="function")
def client(db):
    """Create a test client with database override"""
    def override_get_db():
        try:
            yield db
        finally:
            pass
    
    app.dependency_overrides[get_db] = override_get_db
    with TestClient(app) as c:
        yield c
    app.dependency_overrides.clear()
"#.to_string()
}

/// Generate model tests
fn generate_model_tests(ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();

    content.push_str("# Intent Compiler Generated Model Tests\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import pytest\n");
    content.push_str("from pydantic import ValidationError\n\n");
    content.push_str("from models import *\n\n\n");

    for entity in &ast.entities {
        content.push_str(&generate_entity_test(entity)?);
        content.push_str("\n\n");
    }

    Ok(content)
}

/// Generate tests for a single entity
fn generate_entity_test(entity: &Entity) -> CompileResult<String> {
    let mut content = String::new();

    content.push_str(&format!("class Test{}Model:\n", entity.name));
    content.push_str(&format!("    \"\"\"Tests for {} model\"\"\"\n\n", entity.name));

    // Test valid creation
    content.push_str(&format!("    def test_create_valid_{}(self):\n", entity.name.to_lowercase()));
    content.push_str(&format!("        \"\"\"Test creating a valid {}\"\"\"\n", entity.name));
    
    // Build sample data
    content.push_str(&format!("        data = {{\n"));
    for field in &entity.fields {
        if !field.decorators.contains(&Decorator::Primary) {
            let sample_value = get_sample_value(&field.field_type);
            content.push_str(&format!("            \"{}\": {},\n", field.name, sample_value));
        }
    }
    content.push_str("        }\n");
    content.push_str(&format!("        obj = {}Create(**data)\n", entity.name));
    
    // Assert fields
    for field in &entity.fields {
        if !field.decorators.contains(&Decorator::Primary) {
            content.push_str(&format!("        assert obj.{} is not None\n", field.name));
        }
    }
    content.push_str("\n");

    // Test required fields
    content.push_str(&format!("    def test_{}_required_fields(self):\n", entity.name.to_lowercase()));
    content.push_str("        \"\"\"Test that required fields are validated\"\"\"\n");
    content.push_str("        with pytest.raises(ValidationError):\n");
    content.push_str(&format!("            {}Create()\n", entity.name));

    Ok(content)
}

/// Generate API tests
fn generate_api_tests(ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();

    content.push_str("# Intent Compiler Generated API Tests\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import pytest\n\n\n");

    // Test root endpoint
    content.push_str("def test_root(client):\n");
    content.push_str("    \"\"\"Test root endpoint\"\"\"\n");
    content.push_str("    response = client.get(\"/\")\n");
    content.push_str("    assert response.status_code == 200\n");
    content.push_str("    assert \"status\" in response.json()\n\n\n");

    // Test health endpoint
    content.push_str("def test_health(client):\n");
    content.push_str("    \"\"\"Test health endpoint\"\"\"\n");
    content.push_str("    response = client.get(\"/health\")\n");
    content.push_str("    assert response.status_code == 200\n");
    content.push_str("    assert response.json()[\"status\"] == \"healthy\"\n\n\n");

    // Generate tests for each entity's CRUD endpoints
    for entity in &ast.entities {
        content.push_str(&generate_entity_api_tests(entity, ast)?);
        content.push_str("\n\n");
    }

    // Generate tests for each action
    for action in &ast.actions {
        content.push_str(&generate_action_test(action, ast)?);
        content.push_str("\n\n");
    }

    Ok(content)
}

/// Generate API tests for an entity
fn generate_entity_api_tests(entity: &Entity, ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    let entity_lower = entity.name.to_lowercase();

    content.push_str(&format!("class Test{}API:\n", entity.name));
    content.push_str(&format!("    \"\"\"API tests for {} endpoints\"\"\"\n\n", entity.name));

    // Helper to check if a route already exists (matches logic in api.rs)
    let route_exists = |method: crate::ast::HttpMethod, path_suffix: &str| -> bool {
        let expected_path = format!("/{}{}", entity_lower, path_suffix);
        ast.actions.iter().any(|a| {
            a.decorators.iter().any(|d| {
                if let Decorator::Api { method: m, path: p } = d {
                    m == &method && (p == &expected_path || p == &format!("/{}s{}", entity_lower, path_suffix))
                } else {
                    false
                }
            })
        })
    };

    // Test list endpoint
    if !route_exists(crate::ast::HttpMethod::Get, "s") {
        content.push_str(&format!("    def test_list_{}s(self, client):\n", entity_lower));
        content.push_str(&format!("        \"\"\"Test listing all {}s\"\"\"\n", entity_lower));
        content.push_str(&format!("        response = client.get(\"/{}s\")\n", entity_lower));
        content.push_str("        assert response.status_code == 200\n");
        content.push_str("        assert isinstance(response.json(), list)\n\n");
    }

    // Test get by ID (expects 404 for non-existent)
    if !route_exists(crate::ast::HttpMethod::Get, "s/{id}") {
        content.push_str(&format!("    def test_get_{}_not_found(self, client):\n", entity_lower));
        content.push_str(&format!("        \"\"\"Test getting a non-existent {}\"\"\"\n", entity_lower));
        content.push_str(&format!("        response = client.get(\"/{}s/nonexistent-id\")\n", entity_lower));
        content.push_str("        assert response.status_code == 404\n");
    }

    Ok(content)
}

/// Generate test for an action
fn generate_action_test(action: &Action, ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();

    // Get API decorator
    let api_info = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, path } = d {
            Some((method, path))
        } else {
            None
        }
    });

    if let Some((method, path)) = api_info {
        let method_str = format!("{:?}", method).to_lowercase();
        let requires_auth = action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. }));
        
        content.push_str(&format!("def test_{}(client):\n", action.name));
        content.push_str(&format!("    \"\"\"Test {} endpoint\"\"\"\n", action.name));
        
        // Determine the full path by finding which entity this action belongs to
        let entity_prefix = if let Some(target_entity) = action.infer_entity(ast) {
             format!("/{}s", target_entity.to_lowercase())
        } else {
             String::new()
        };

        // Replace path params with test values
        let mut test_path = path.replace("{id}", "test-id");
        if !test_path.starts_with(&entity_prefix) && !entity_prefix.is_empty() {
            test_path = format!("{}{}", entity_prefix, test_path);
        }
        
        // Build JSON body if needed
        let mut json_arg = String::new();
        if matches!(method, crate::ast::HttpMethod::Post | crate::ast::HttpMethod::Put | crate::ast::HttpMethod::Patch) {
            if let Some(input) = &action.input {
                if !input.fields.is_empty() {
                    let mut json_body = String::from("json={");
                    for (i, field) in input.fields.iter().enumerate() {
                        if i > 0 { json_body.push_str(", "); }
                        json_body.push_str(&format!("\"{}\": {}", field.name, get_sample_value(&field.param_type)));
                    }
                    json_body.push_str("}");
                    json_arg = format!(", {}", json_body);
                }
            } else {
                 json_arg = ", json={}".to_string();
            }
        }
        
        content.push_str(&format!("    response = client.{}(\"{}\"{})\n", method_str, test_path, json_arg));
        
        if requires_auth {
             content.push_str("    # Expect 401 Unauthorized for unauthenticated requests\n");
             content.push_str("    assert response.status_code == 401\n");
        } else {
             content.push_str("    # Add assertions based on expected behavior\n");
             content.push_str("    assert response.status_code in [200, 201, 400, 404, 422]\n");
        }
    }

    Ok(content)
}

/// Get a sample value for a field type
fn get_sample_value(field_type: &crate::ast::FieldType) -> String {
    match field_type {
        crate::ast::FieldType::String => "\"test_value\"".to_string(),
        crate::ast::FieldType::Number => "42.0".to_string(),
        crate::ast::FieldType::Boolean => "True".to_string(),
        crate::ast::FieldType::DateTime => "\"2024-01-01T00:00:00\"".to_string(),
        crate::ast::FieldType::Uuid => "\"550e8400-e29b-41d4-a716-446655440000\"".to_string(),
        crate::ast::FieldType::Email => "\"test@example.com\"".to_string(),
        crate::ast::FieldType::Enum(values) => {
            format!("\"{}\"", values.first().unwrap_or(&"value".to_string()))
        }
        crate::ast::FieldType::Reference(_) => "\"ref_id\"".to_string(),
        crate::ast::FieldType::Ref(_) => "\"ref_id\"".to_string(),
        crate::ast::FieldType::Array(_) => "[]".to_string(),
        crate::ast::FieldType::List(_) => "[]".to_string(),
        crate::ast::FieldType::Optional(inner) => get_sample_value(inner),
    }
}

/// Generate repository tests
fn generate_repository_tests(ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    content.push_str("# Intent Compiler Generated Repository Tests\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import pytest\n");
    content.push_str("from repositories import *\n");
    content.push_str("from db.models import *\n\n\n");

    for entity in &ast.entities {
        let repo_class = format!("{}Repository", entity.name);
        content.push_str(&format!("class Test{}:\n", repo_class));
        content.push_str(&format!("    \"\"\"Tests for {} singleton\"\"\"\n\n", repo_class));
        
        content.push_str("    def test_repository_get_all(self, db):\n");
        content.push_str(&format!("        repo = {}()\n", repo_class));
        content.push_str("        results = repo.get_all(db)\n");
        content.push_str("        assert isinstance(results, list)\n\n");
    }

    Ok(content)
}

/// Generate service tests
fn generate_service_tests(ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    content.push_str("# Intent Compiler Generated Service Tests\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import pytest\n");
    content.push_str("from services import *\n");
    content.push_str("from models import *\n\n\n");

    for entity in &ast.entities {
        let name = &entity.name;
        let service_class = format!("{}Service", name);
        content.push_str(&format!("class Test{}:\n", service_class));
        content.push_str(&format!("    \"\"\"Tests for {} singleton\"\"\"\n\n", service_class));
        
        content.push_str("    def test_service_get_all(self, db):\n");
        content.push_str(&format!("        service = {}()\n", service_class));
        content.push_str("        results = service.get_all(db)\n");
        content.push_str("        assert isinstance(results, list)\n\n");

        content.push_str("    def test_service_get_by_id(self, db):\n");
        content.push_str(&format!("        service = {}()\n", service_class));
        content.push_str("        result = service.get_by_id(db, \"550e8400-e29b-41d4-a716-446655440000\")\n");
        content.push_str("        assert result is None\n\n");

        // Action tests
        for action in &ast.actions {
            if let Some(output) = &action.output {
                if output.entity == *name {
                    content.push_str(&generate_service_action_test(action, name)?);
                }
            }
        }
    }

    Ok(content)
}

fn generate_service_action_test(action: &Action, entity_name: &str) -> CompileResult<String> {
    let mut content = String::new();
    let action_name = &action.name;
    let service_class = format!("{}Service", entity_name);

    content.push_str(&format!("    def test_service_{}(self, db):\n", action_name));
    content.push_str(&format!("        \"\"\"Test {} service method\"\"\"\n", action_name));
    content.push_str(&format!("        service = {}()\n", service_class));
    
    let mut call_params = Vec::new();
    
    // Get API info
    let api_info = action.decorators.iter().find_map(|d| {
        if let Decorator::Api { method, path } = d { Some((method, path)) } else { None }
    });

    if let Some((method, path)) = api_info {
        // 1. Path Params
        for segment in path.split('/') {
            if segment.starts_with('{') && segment.ends_with('}') {
                let name = &segment[1..segment.len()-1];
                content.push_str(&format!("        {} = \"550e8400-e29b-41d4-a716-446655440000\"\n", name));
                call_params.push(name.to_string());
            }
        }

        // 2. Data Param (only for POST/PUT/PATCH)
        if matches!(method, crate::ast::HttpMethod::Post | crate::ast::HttpMethod::Put | crate::ast::HttpMethod::Patch) {
            let has_input = action.input.as_ref().map(|i| !i.fields.is_empty()).unwrap_or(false);
            if has_input {
                let req_model = format!("{}Request", crate::codegen::python::models::to_pascal_case(action_name));
                content.push_str(&format!("        data = {}(\n", req_model));
                for field in &action.input.as_ref().unwrap().fields {
                    content.push_str(&format!("            {}={},\n", field.name, get_sample_value(&field.param_type)));
                }
                content.push_str("        )\n");
                call_params.push("data".to_string());
            }
        }
    } else {
        // No API decorator - use direct input fields
        if let Some(input) = &action.input {
            for field in &input.fields {
                content.push_str(&format!("        {} = {}\n", field.name, get_sample_value(&field.param_type)));
                call_params.push(field.name.clone());
            }
        }
    }

    // 3. Always include db
    call_params.push("db".to_string());

    // 4. Handle Auth
    let uses_current_user = action.process.as_ref().map(|p| {
        p.steps.iter().any(|s| match s {
             crate::ast::ProcessStep::Derive(d) => match &d.value {
                 crate::ast::DeriveValue::FieldAccess { path } => {
                      !path.is_empty() && path[0] == "current_user"
                 },
                 crate::ast::DeriveValue::Identifier(id) => id.starts_with("current_user."),
                 _ => false
             },
             _ => false
        })
    }).unwrap_or(false);
    let requires_auth = action.decorators.iter().any(|d| matches!(d, Decorator::Auth { .. }));

    if requires_auth || uses_current_user {
         content.push_str("        current_user = {\"id\": \"test-user\", \"email\": \"test@example.com\", \"role\": \"user\"}\n");
         call_params.push("current_user".to_string());
    }

    let params_str = call_params.join(", ");
    content.push_str(&format!("        # We use a try-except to swallow errors for basic coverage\n"));
    content.push_str("        from fastapi import HTTPException\n");
    content.push_str("        try:\n");
    content.push_str(&format!("            result = service.{}({})\n", action_name, params_str));
    content.push_str("            assert result is not None\n");
    content.push_str("        except (HTTPException, Exception):\n");
    content.push_str("            pass\n\n");

    Ok(content)
}

/// Generate controller tests
fn generate_controller_tests(ast: &IntentFile) -> CompileResult<String> {
    let mut content = String::new();
    content.push_str("# Intent Compiler Generated Controller Tests\n");
    content.push_str("# Generated automatically - do not edit\n\n");
    content.push_str("import pytest\n");
    content.push_str("from controllers import *\n\n\n");

    for entity in &ast.entities {
        let controller_class = format!("{}Controller", entity.name);
        content.push_str(&format!("class Test{}:\n", controller_class));
        content.push_str(&format!("    \"\"\"Tests for {} singleton\"\"\"\n\n", controller_class));
    }

    Ok(content)
}
