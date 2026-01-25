use crate::ast::*;

/// Inject default auth actions if an auth entity is defined
pub fn inject_auth_actions(file: &mut IntentFile) {
    let auth_entity_name = match &file.auth_entity {
        Some(name) => name.clone(),
        None => return,
    };

    // Find the auth entity to see what fields it has
    let auth_entity = file.find_entity(&auth_entity_name).cloned();
    let password_field = if let Some(ent) = &auth_entity {
        if ent.fields.iter().any(|f| f.name == "password_hash") {
            "password_hash".to_string()
        } else {
            "password".to_string()
        }
    } else {
        "password_hash".to_string()
    };

    // 1. Signup Action
    if !file.actions.iter().any(|a| a.name == "signup") {
        let mut signup_params = vec![
             ActionParam { 
                name: "email".to_string(), 
                param_type: FieldType::Email, 
                decorators: vec![],
                location: SourceLocation::default(),
            },
            ActionParam { 
                name: "password".to_string(), 
                param_type: FieldType::String, 
                decorators: vec![],
                location: SourceLocation::default(),
            },
        ];

        let mut signup_setters = vec![
             MutateSetter { 
                field: "email".to_string(), 
                value: DeriveValue::FieldAccess { path: vec!["input".to_string(), "email".to_string()] },
                location: SourceLocation::default(),
            },
            MutateSetter { 
                field: password_field.clone(), 
                value: DeriveValue::Compute { 
                    function: "hash".to_string(), 
                    args: vec![FunctionArg::FieldAccess { path: vec!["input".to_string(), "password".to_string()] }] 
                },
                location: SourceLocation::default(),
            },
        ];

        // Add other non-auto fields from auth entity to signup
        if let Some(ent) = &auth_entity {
            for field in &ent.fields {
                if field.name != "email" && field.name != password_field && 
                   !field.decorators.contains(&Decorator::Primary) && 
                   !field.decorators.contains(&Decorator::Auto) {
                    
                    let mut param_type = field.field_type.clone();
                    if field.decorators.iter().any(|d| matches!(d, Decorator::Default(_))) {
                        param_type = FieldType::Optional(Box::new(param_type));
                    }

                    signup_params.push(ActionParam {
                        name: field.name.clone(),
                        param_type,
                        decorators: field.decorators.clone(),
                        location: SourceLocation::default(),
                    });
                    
                    signup_setters.push(MutateSetter {
                        field: field.name.clone(),
                        value: DeriveValue::FieldAccess { path: vec!["input".to_string(), field.name.clone()] },
                        location: SourceLocation::default(),
                    });
                }
            }
        }

        let entity_prefix = format!("/{}s", auth_entity_name.to_lowercase());

        file.actions.push(Action {
            name: "signup".to_string(),
            decorators: vec![
                Decorator::Api { method: HttpMethod::Post, path: format!("{}/signup", entity_prefix) }
            ],
            input: Some(InputSection { fields: signup_params }),
            process: Some(ProcessSection {
                steps: vec![
                    ProcessStep::Mutate(MutateBlock {
                        entity: auth_entity_name.clone(),
                        predicate: None,
                        setters: signup_setters,
                        location: SourceLocation::default(),
                    })
                ]
            }),
            output: Some(OutputSection { 
                entity: auth_entity_name.clone(), 
                fields: vec!["id".to_string(), "email".to_string()] 
            }),
            location: SourceLocation::default(),
        });
    }

    let entity_prefix = format!("/{}s", auth_entity_name.to_lowercase());

    // 2. Login Action
    if !file.actions.iter().any(|a| a.name == "login") {
        file.actions.push(Action {
            name: "login".to_string(),
            decorators: vec![
                Decorator::Api { method: HttpMethod::Post, path: format!("{}/login", entity_prefix) }
            ],
            input: Some(InputSection {
                fields: vec![
                    ActionParam { 
                        name: "email".to_string(), 
                        param_type: FieldType::Email, 
                        decorators: vec![],
                        location: SourceLocation::default(),
                    },
                    ActionParam { 
                        name: "password".to_string(), 
                        param_type: FieldType::String, 
                        decorators: vec![],
                        location: SourceLocation::default(),
                    },
                ]
            }),
            process: Some(ProcessSection {
                steps: vec![
                    ProcessStep::Derive(DeriveStatement {
                        name: "user".to_string(),
                        value: DeriveValue::Select { 
                            entity: auth_entity_name.clone(), 
                            predicate: Predicate {
                                field: FieldReference::InputField("email".to_string()),
                                operator: CompareOp::Equal,
                                value: FieldReference::InputField("email".to_string()),
                            }
                        },
                        location: SourceLocation::default(),
                    }),
                    ProcessStep::Derive(DeriveStatement {
                        name: "valid".to_string(),
                        value: DeriveValue::Compute { 
                            function: "verify_hash".to_string(), 
                            args: vec![
                                FunctionArg::FieldAccess { path: vec!["input".to_string(), "password".to_string()] },
                                FunctionArg::FieldAccess { path: vec!["user".to_string(), password_field.clone()] },
                            ]
                        },
                        location: SourceLocation::default(),
                    }),
                    ProcessStep::Derive(DeriveStatement {
                        name: "token".to_string(),
                        value: DeriveValue::SystemCall { 
                            namespace: "jwt".to_string(),
                            capability: "create".to_string(),
                            args: vec![FunctionArg::FieldAccess { path: vec!["user".to_string(), "email".to_string()] }] 
                        },
                        location: SourceLocation::default(),
                    }),
                ]
            }),
            output: Some(OutputSection { 
                entity: auth_entity_name.clone(), 
                fields: vec!["id".to_string(), "token".to_string()] 
            }),
            location: SourceLocation::default(),
        });
    }

    // 3. Get Me
    if !file.actions.iter().any(|a| a.name == "get_me") {
        file.actions.push(Action {
            name: "get_me".to_string(),
            decorators: vec![
                Decorator::Api { method: HttpMethod::Get, path: format!("{}/me", entity_prefix) },
                Decorator::Auth { name: None, args: vec![] }
            ],
            input: None,
            process: None,
            output: Some(OutputSection { 
                entity: auth_entity_name.clone(), 
                fields: vec!["id".to_string(), "email".to_string(), "role".to_string()] 
            }),
            location: SourceLocation::default(),
        });
    }

    // 4. Logout Action
    if !file.actions.iter().any(|a| a.name == "logout") {
        file.actions.push(Action {
            name: "logout".to_string(),
            decorators: vec![
                Decorator::Api { method: HttpMethod::Post, path: format!("{}/logout", entity_prefix) },
                Decorator::Auth { name: None, args: vec![] }
            ],
            input: None,
            process: Some(ProcessSection { steps: vec![] }),
            output: Some(OutputSection { entity: auth_entity_name.clone(), fields: vec![] }),
            location: SourceLocation::default(),
        });
    }

    // 5. Token Refresh
    if !file.actions.iter().any(|a| a.name == "refresh_token") {
        file.actions.push(Action {
            name: "refresh_token".to_string(),
            decorators: vec![
                Decorator::Api { method: HttpMethod::Post, path: format!("{}/token/refresh", entity_prefix) }
            ],
            input: Some(InputSection {
                fields: vec![
                    ActionParam { 
                        name: "refresh_token".to_string(), 
                        param_type: FieldType::String, 
                        decorators: vec![],
                        location: SourceLocation::default(),
                    },
                ]
            }),
            process: Some(ProcessSection {
                steps: vec![
                    ProcessStep::Derive(DeriveStatement {
                        name: "token".to_string(),
                        value: DeriveValue::Literal(LiteralValue::String("new_token_placeholder".to_string())),
                        location: SourceLocation::default(),
                    }),
                ]
            }),
            output: Some(OutputSection { entity: auth_entity_name.clone(), fields: vec!["token".to_string()] }),
            location: SourceLocation::default(),
        });
    }

    // 6. Forgot Password
    if !file.actions.iter().any(|a| a.name == "forgot_password") {
        file.actions.push(Action {
            name: "forgot_password".to_string(),
            decorators: vec![
                Decorator::Api { method: HttpMethod::Post, path: format!("{}/forgot-password", entity_prefix) }
            ],
            input: Some(InputSection {
                fields: vec![
                    ActionParam { 
                        name: "email".to_string(), 
                        param_type: FieldType::Email, 
                        decorators: vec![],
                        location: SourceLocation::default(),
                    },
                ]
            }),
            process: Some(ProcessSection { steps: vec![] }),
            output: Some(OutputSection { entity: auth_entity_name.clone(), fields: vec![] }),
            location: SourceLocation::default(),
        });
    }

    // 7. Reset Password
    if !file.actions.iter().any(|a| a.name == "reset_password") {
        file.actions.push(Action {
            name: "reset_password".to_string(),
            decorators: vec![
                Decorator::Api { method: HttpMethod::Post, path: format!("{}/reset-password", entity_prefix) }
            ],
            input: Some(InputSection {
                fields: vec![
                    ActionParam { 
                        name: "token".to_string(), 
                        param_type: FieldType::String, 
                        decorators: vec![],
                        location: SourceLocation::default(),
                    },
                    ActionParam { 
                        name: "new_password".to_string(), 
                        param_type: FieldType::String, 
                        decorators: vec![],
                        location: SourceLocation::default(),
                    },
                ]
            }),
            process: Some(ProcessSection { steps: vec![] }),
            output: Some(OutputSection { entity: auth_entity_name.clone(), fields: vec![] }),
            location: SourceLocation::default(),
        });
    }
}
