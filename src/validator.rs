// Intent Compiler - Semantic Validator
// Validates the AST for type correctness and semantic rules

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::{CompileError, CompileResult, Warning};

/// Validation context holding symbol tables
pub struct ValidationContext {
    pub entities: HashMap<String, Entity>,
    pub actions: HashMap<String, Action>,
    pub policies: HashMap<String, Policy>,
    pub warnings: Vec<Warning>,
    /// The designated auth entity name, if one is defined
    pub auth_entity: Option<String>,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            actions: HashMap::new(),
            policies: HashMap::new(),
            warnings: Vec::new(),
            auth_entity: None,
        }
    }

    pub fn add_warning(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate an intent file
pub fn validate(file: &IntentFile) -> CompileResult<ValidationContext> {
    let mut ctx = ValidationContext::new();
    let mut errors: Vec<CompileError> = Vec::new();

    // First pass: collect all entity and action names
    for entity in &file.entities {
        if ctx.entities.contains_key(&entity.name) {
            errors.push(CompileError::validation(
                format!("Duplicate entity name: {}", entity.name),
                entity.location.clone(),
            ));
        } else {
            ctx.entities.insert(entity.name.clone(), entity.clone());
        }

        // Check for auth entity
        if entity.is_auth {
            if ctx.auth_entity.is_some() {
                errors.push(CompileError::validation(
                    format!("Multiple auth entities defined: '{}' and '{}'. Only one auth entity is allowed.",
                        ctx.auth_entity.as_ref().unwrap(), entity.name),
                    entity.location.clone(),
                ));
            } else {
                ctx.auth_entity = Some(entity.name.clone());
            }
        }
    }

    for action in &file.actions {
        if ctx.actions.contains_key(&action.name) {
            errors.push(CompileError::validation(
                format!("Duplicate action name: {}", action.name),
                action.location.clone(),
            ));
        } else {
            ctx.actions.insert(action.name.clone(), action.clone());
        }
    }


    // Collect policies (global and entity-scoped)
    for policy in &file.policies {
        if ctx.policies.contains_key(&policy.name) {
             errors.push(CompileError::validation(
                format!("Duplicate policy name: {}", policy.name),
                policy.location.clone(),
            ));
        } else {
            ctx.policies.insert(policy.name.clone(), policy.clone());
        }
    }

    for entity in &file.entities {
        for policy in &entity.policies {
            let full_name = format!("{}.{}", entity.name, policy.name);
            if ctx.policies.contains_key(&full_name) {
                 errors.push(CompileError::validation(
                    format!("Duplicate policy name: {}", full_name),
                    policy.location.clone(),
                ));
            } else {
                ctx.policies.insert(full_name, policy.clone());
            }
        }
    }

    // Second pass: validate each construct
    for entity in &file.entities {
        if let Err(e) = validate_entity(entity, &mut ctx) {
            errors.push(e);
        }
    }

    for action in &file.actions {
        if let Err(e) = validate_action(action, &ctx) {
            errors.push(e);
        }
    }

    for rule in &file.rules {
        if let Err(e) = validate_rule(rule, &ctx) {
            errors.push(e);
        }
    }

    // Validate global policies
    for policy in &file.policies {
        if let Err(e) = validate_policy(policy, &ctx) {
            errors.push(e);
        }
    }

    // Validate entity-scoped policies
    for entity in &file.entities {
        for policy in &entity.policies {
            if let Err(e) = validate_policy(policy, &ctx) {
                errors.push(e);
            }
        }
    }

    if errors.is_empty() {
        Ok(ctx)
    } else if errors.len() == 1 {
        Err(errors.remove(0))
    } else {
        Err(CompileError::MultipleErrors(errors))
    }
}

/// Validate an entity definition
fn validate_entity(entity: &Entity, ctx: &mut ValidationContext) -> CompileResult<()> {
    let mut field_names = HashSet::new();
    let mut has_primary = false;

    for field in &entity.fields {
        // Check for duplicate field names
        if field_names.contains(&field.name) {
            return Err(CompileError::validation(
                format!("Duplicate field name '{}' in entity '{}'", field.name, entity.name),
                field.location.clone(),
            ));
        }
        field_names.insert(&field.name);

        // Check for primary key
        if field.decorators.contains(&Decorator::Primary) {
            if has_primary {
                return Err(CompileError::validation(
                    format!("Entity '{}' has multiple @primary fields", entity.name),
                    field.location.clone(),
                ));
            }
            has_primary = true;
        }

        // Validate field type
        validate_field_type(&field.field_type, ctx, &field.location)?;

        // Validate decorator combinations
        validate_decorators(&field.decorators, &field.location)?;
    }

    // Warn if no primary key
    if !has_primary {
        ctx.add_warning(Warning::with_hint(
            format!("Entity '{}' has no @primary field", entity.name),
            entity.location.clone(),
            "Consider adding @primary to an id field",
        ));
    }

    Ok(())
}

/// Validate a field type
fn validate_field_type(
    field_type: &FieldType,
    ctx: &ValidationContext,
    location: &SourceLocation,
) -> CompileResult<()> {
    match field_type {
        FieldType::Reference(name) | FieldType::Ref(name) => {
            if !ctx.entities.contains_key(name) {
                return Err(CompileError::validation_with_hint(
                    format!("Unknown entity reference: {}", name),
                    location.clone(),
                    format!("Available entities: {:?}", ctx.entities.keys().collect::<Vec<_>>()),
                ));
            }
        }
        FieldType::Array(inner) | FieldType::List(inner) => {
            validate_field_type(inner, ctx, location)?;
        }
        FieldType::Optional(inner) => {
            validate_field_type(inner, ctx, location)?;
        }
        FieldType::Enum(values) => {
            if values.is_empty() {
                return Err(CompileError::validation(
                    "Enum type must have at least one value",
                    location.clone(),
                ));
            }
            let unique: HashSet<_> = values.iter().collect();
            if unique.len() != values.len() {
                return Err(CompileError::validation(
                    "Enum type has duplicate values",
                    location.clone(),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Validate decorator combinations
fn validate_decorators(decorators: &[Decorator], location: &SourceLocation) -> CompileResult<()> {
    let mut has_primary = false;
    let mut has_optional = false;

    for dec in decorators {
        match dec {
            Decorator::Primary => has_primary = true,
            Decorator::Optional => has_optional = true,
            _ => {}
        }
    }

    if has_primary && has_optional {
        return Err(CompileError::validation(
            "Field cannot be both @primary and @optional",
            location.clone(),
        ));
    }

    Ok(())
}

/// Validate an action definition
fn validate_action(action: &Action, ctx: &ValidationContext) -> CompileResult<()> {
    let mut has_api = false;
    let mut param_names = HashSet::new();

    // Validate input fields ( structured syntax)
    if let Some(input) = &action.input {
        for param in &input.fields {
            if param_names.contains(&param.name) {
                return Err(CompileError::validation(
                    format!("Duplicate parameter '{}' in action '{}'", param.name, action.name),
                    param.location.clone(),
                ));
            }
            param_names.insert(&param.name);

            validate_field_type(&param.param_type, ctx, &param.location)?;
        }
    }

    // Validate output section
    if let Some(output) = &action.output {
        if !ctx.entities.contains_key(&output.entity) {
            return Err(CompileError::validation_with_hint(
                format!("Unknown output type: {}", output.entity),
                action.location.clone(),
                "Output type must be a defined entity",
            ));
        }
    }

    // Validate decorators
    for decorator in &action.decorators {
        match decorator {
            Decorator::Api { method: _, path } => {
                has_api = true;
                validate_api_path(path, &param_names, &action.location)?;
            }
            Decorator::Auth { name, args } => {
                if let Some(name) = name {
                    let first_char = name.chars().next().unwrap_or(' ');
                    if first_char.is_uppercase() {
                        if !ctx.entities.contains_key(name) {
                            return Err(CompileError::validation_with_hint(
                                format!("Unknown entity in @auth: {}", name),
                                action.location.clone(),
                                format!("Available entities: {:?}", ctx.entities.keys().collect::<Vec<_>>()),
                            ));
                        }
                    } else {
                        if !ctx.actions.contains_key(name) {
                            return Err(CompileError::validation_with_hint(
                                format!("Unknown action in @auth: {}", name),
                                action.location.clone(),
                                format!("Available actions: {:?}", ctx.actions.keys().collect::<Vec<_>>()),
                            ));
                        }
                    }

                    // Validate arguments
                    for arg in args {
                        if !param_names.contains(arg) {
                            return Err(CompileError::validation_with_hint(
                                format!("Unknown argument '{}' in @auth", arg),
                                action.location.clone(),
                                format!("Available parameters: {:?}", param_names.iter().collect::<Vec<_>>()),
                            ));
                        }
                    }
                } else {
                    // @auth without arguments requires an auth entity to be defined
                    if ctx.auth_entity.is_none() {
                        return Err(CompileError::validation_with_hint(
                            "@auth decorator used without arguments, but no auth entity is defined".to_string(),
                            action.location.clone(),
                            "Define an auth entity using 'auth entity EntityName:' or specify an entity/action in @auth(Name)",
                        ));
                    }
                }
            }
            Decorator::Policy(name) => {
                // Check if policy exists
                if !ctx.policies.contains_key(name) {
                     return Err(CompileError::validation_with_hint(
                        format!("Unknown policy: {}", name),
                        action.location.clone(),
                        format!("Available policies: {:?}", ctx.policies.keys().collect::<Vec<_>>()),
                    ));
                }
            }
            _ => {}
        }
    }

    if !has_api {
        // Actions without @api are internal and don't generate routes
        // For  we just allow them
    }

    Ok(())
}

/// Validate an API path
fn validate_api_path(
    path: &str,
    param_names: &HashSet<&String>,
    location: &SourceLocation,
) -> CompileResult<()> {
    // Extract path parameters like {id}
    let path_params: Vec<&str> = path
        .split('/')
        .filter(|s| s.starts_with('{') && s.ends_with('}'))
        .map(|s| &s[1..s.len() - 1])
        .collect();

    // Check that path parameters are defined in action params
    for path_param in path_params {
        if !param_names.iter().any(|p| p.as_str() == path_param) {
            return Err(CompileError::validation_with_hint(
                format!("Path parameter '{{{}}}' not found in action parameters", path_param),
                location.clone(),
                format!("Add '{}: type' to action parameters", path_param),
            ));
        }
    }

    Ok(())
}

/// Validate a rule definition
fn validate_rule(rule: &crate::ast::Rule, ctx: &ValidationContext) -> CompileResult<()> {
    // Validate the condition expression
    validate_expression(&rule.condition, ctx, &rule.location)?;

    // Validate the consequence
    validate_consequence(&rule.consequence, ctx, &rule.location)?;

    Ok(())
}

/// Validate an expression
fn validate_expression(
    expr: &Expression,
    ctx: &ValidationContext,
    location: &SourceLocation,
) -> CompileResult<()> {
    match expr {
        Expression::Binary { left, operator: _, right } => {
            validate_expression(left, ctx, location)?;
            validate_expression(right, ctx, location)?;
        }
        Expression::Logical { left, operator: _, right } => {
            validate_expression(left, ctx, location)?;
            validate_expression(right, ctx, location)?;
        }
        Expression::Not(inner) => {
            validate_expression(inner, ctx, location)?;
        }
        Expression::FieldAccess { entity, field } => {
            if entity == "subject" {
                // 'subject' refers to the auth entity or the specified subject in the policy
                // For now, we mainly check if it's the auth entity
                if let Some(auth_entity_name) = &ctx.auth_entity {
                    if let Some(ent) = ctx.entities.get(auth_entity_name) {
                        if !ent.fields.iter().any(|f| &f.name == field) {
                            return Err(CompileError::validation_with_hint(
                                format!("Field '{}' not found in auth entity '{}' (referenced via 'subject')", field, auth_entity_name),
                                location.clone(),
                                format!(
                                    "Available fields: {:?}",
                                    ent.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
                                ),
                            ));
                        }
                        return Ok(());
                    }
                }
                
                // If it's a specific entity subject, validate that later or just allow for now
                // if we don't have enough context in validate_expression
                return Ok(());
            }
            // Check entity exists
            if let Some(ent) = ctx.entities.get(entity) {
                // Check field exists
                if !ent.fields.iter().any(|f| &f.name == field) {
                    return Err(CompileError::validation_with_hint(
                        format!("Field '{}' not found in entity '{}'", field, entity),
                        location.clone(),
                        format!(
                            "Available fields: {:?}",
                            ent.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
                        ),
                    ));
                }
            } else {
                return Err(CompileError::validation(
                    format!("Unknown entity: {}", entity),
                    location.clone(),
                ));
            }
        }
        Expression::Literal(_) => {}
        Expression::Identifier(_) => {}
    }
    Ok(())
}

/// Validate a consequence
fn validate_consequence(
    consequence: &Consequence,
    ctx: &ValidationContext,
    location: &SourceLocation,
) -> CompileResult<()> {
    match consequence {
        Consequence::ActionCall { action, args } => {
            // Check action exists (skip built-in actions)
            if !ctx.actions.contains_key(action) {
                return Err(CompileError::validation_with_hint(
                    format!("Unknown action: {}", action),
                    location.clone(),
                    format!("Available actions: {:?}", ctx.actions.keys().collect::<Vec<_>>()),
                ));
            }

            // Validate arguments
            for arg in args {
                validate_expression(arg, ctx, location)?;
            }
        }
        Consequence::Reject(message) | Consequence::Log(message) => {
            if message.is_empty() {
                return Err(CompileError::validation(
                    "Empty message in reject/log",
                    location.clone(),
                ));
            }
        }
    }
    Ok(())
}

/// Validate a policy definition
fn validate_policy(policy: &Policy, ctx: &ValidationContext) -> CompileResult<()> {
    if policy.subject != "@auth" && !ctx.entities.contains_key(&policy.subject) {
        return Err(CompileError::validation_with_hint(
            format!("Unknown subject in policy '{}': {}", policy.name, policy.subject),
            policy.location.clone(),
            format!("Subject must be '@auth' or a defined entity name. Available entities: {:?}", ctx.entities.keys().collect::<Vec<_>>()),
        ));
    }
    validate_expression(&policy.require, ctx, &policy.location)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_intent;

    #[test]
    fn test_validate_duplicate_entity() {
        let source = r#"entity User:
    id: string @primary

entity User:
    name: string
"#;
        let file = parse_intent(source).unwrap();
        let result = validate(&file);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_unknown_reference() {
        let source = r#"entity Post:
    id: string @primary
    author: UnknownEntity
"#;
        let file = parse_intent(source).unwrap();
        let result = validate(&file);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_file() {
        let source = r#"entity User:
    id: string @primary
    name: string

@api POST /users
action create_user:
    input:
        name: string
    output: User(id, name)
"#;
        let file = parse_intent(source).unwrap();
        let result = validate(&file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_auth_unknown_entity() {
        let source = r#"
entity User:
    id: string @primary

@api GET /private
@auth(UnknownUser)
action private_action:
    output: User(id)
"#;
        let file = parse_intent(source).unwrap();
        let result = validate(&file);
        assert!(result.is_err());
        match result.err().unwrap() {
            CompileError::ValidationError { message, .. } => {
                assert!(message.contains("Unknown entity in @auth"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_validate_nested_policy() {
        let source = r#"
entity User:
    id: uuid @primary
    role: string

auth entity UserAuth:
    id: uuid @primary

entity Order:
    id: uuid @primary
    user_id: uuid
    policy OwnsOrder:
        subject: @auth
        require Order.user_id == subject.id

@api GET /orders/{id}
@auth
@policy(Order.OwnsOrder)
action get_order:
    input:
        id: uuid
    output: Order(id)
"#;
        let file = crate::parser::parse_intent(source).unwrap();
        let result = validate(&file);
        assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    #[test]
    fn test_validate_policy_invalid_subject_field() {
        let source = r#"
auth entity User:
    id: uuid @primary

policy AdminOnly:
    subject: @auth
    require subject.role == "admin"
"#;
        let file = crate::parser::parse_intent(source).unwrap();
        let result = validate(&file);
        assert!(result.is_err());
        match result.err().unwrap() {
            CompileError::ValidationError { message, .. } => {
                assert!(message.contains("Field 'role' not found in auth entity 'User'"));
            }
            _ => panic!("Expected validation error"),
        }
    }
}
