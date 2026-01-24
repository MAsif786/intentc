// Intent Compiler - Parser
// Transforms .intent files into typed AST
//  Spec Implementation

use pest::Parser;
use pest_derive::Parser;

use crate::ast::*;
use crate::error::{CompileError, CompileResult};

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct IntentParser;

/// Parse an intent file from source string
pub fn parse_intent(source: &str) -> CompileResult<IntentFile> {
    let pairs = IntentParser::parse(Rule::intent_file, source).map_err(|e| {
        let (line, column) = match e.line_col {
            pest::error::LineColLocation::Pos((l, c)) => (l, c),
            pest::error::LineColLocation::Span((l, c), _) => (l, c),
        };
        CompileError::parse_with_snippet(
            format!("Syntax error: {}", e.variant.message()),
            line,
            column,
            source.lines().nth(line.saturating_sub(1)).unwrap_or(""),
        )
    })?;

    let mut intent_file = IntentFile::new();

    for pair in pairs {
        if pair.as_rule() == Rule::intent_file {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::definition {
                    parse_definition(inner, &mut intent_file)?;
                }
            }
        }
    }

    Ok(intent_file)
}

/// Parse a top-level definition
fn parse_definition(pair: pest::iterators::Pair<Rule>, file: &mut IntentFile) -> CompileResult<()> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::entity_def => file.entities.push(parse_entity(inner, false)?),
            Rule::auth_entity_def => {
                let entity = parse_entity(inner, true)?;
                file.auth_entity = Some(entity.name.clone());
                file.entities.push(entity);
            }
            Rule::full_action_def => file.actions.push(parse_action(inner)?),
            Rule::rule_def => file.rules.push(parse_rule(inner)?),
            Rule::policy_def => file.policies.push(parse_policy(inner)?),
            _ => {}
        }
    }
    Ok(())
}

/// Parse entity definition
fn parse_entity(pair: pest::iterators::Pair<Rule>, is_auth: bool) -> CompileResult<Entity> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut fields = Vec::new();
    let mut policies = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::entity_name => name = inner.as_str().to_string(),
            Rule::entity_fields => {
                for item_wrapper in inner.into_inner() {
                    if item_wrapper.as_rule() == Rule::entity_item {
                        for item_inner in item_wrapper.into_inner() {
                            match item_inner.as_rule() {
                                Rule::entity_field => {
                                    for field_inner in item_inner.into_inner() {
                                        if field_inner.as_rule() == Rule::field_def {
                                            fields.push(parse_field(field_inner)?);
                                        }
                                    }
                                }
                                Rule::entity_policy => {
                                    for policy_inner in item_inner.into_inner() {
                                        if policy_inner.as_rule() == Rule::nested_policy_def {
                                            policies.push(parse_policy(policy_inner)?);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Entity { name, fields, policies, is_auth, location })
}

/// Parse field definition
fn parse_field(pair: pest::iterators::Pair<Rule>) -> CompileResult<Field> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut field_type = FieldType::String;
    let mut decorators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::field_name => name = inner.as_str().to_string(),
            Rule::field_type => field_type = parse_field_type(inner)?,
            Rule::decorator => {
                if let Some(dec) = parse_decorator(inner)? {
                    decorators.push(dec);
                }
            }
            _ => {}
        }
    }

    Ok(Field { name, field_type, decorators, location })
}

/// Parse field type ( with ref<T> and list<T>)
fn parse_field_type(pair: pest::iterators::Pair<Rule>) -> CompileResult<FieldType> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::list_type => {
                for list_inner in inner.into_inner() {
                    if list_inner.as_rule() == Rule::field_type {
                        return Ok(FieldType::List(Box::new(parse_field_type(list_inner)?)));
                    }
                }
            }
            Rule::ref_type => {
                for ref_inner in inner.into_inner() {
                    if ref_inner.as_rule() == Rule::type_name {
                        return Ok(FieldType::Ref(ref_inner.as_str().to_string()));
                    }
                }
            }
            Rule::array_type => {
                for arr_inner in inner.into_inner() {
                    if arr_inner.as_rule() == Rule::base_type {
                        return Ok(FieldType::Array(Box::new(parse_base_type(arr_inner)?)));
                    }
                }
            }
            Rule::optional_type => {
                for opt_inner in inner.into_inner() {
                    if opt_inner.as_rule() == Rule::base_type {
                        return Ok(FieldType::Optional(Box::new(parse_base_type(opt_inner)?)));
                    }
                }
            }
            Rule::base_type => return parse_base_type(inner),
            _ => {}
        }
    }
    Ok(FieldType::String)
}

/// Parse base type ( with uuid, email)
fn parse_base_type(pair: pest::iterators::Pair<Rule>) -> CompileResult<FieldType> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::primitive_type => {
                return Ok(match inner.as_str() {
                    "string" => FieldType::String,
                    "number" => FieldType::Number,
                    "boolean" => FieldType::Boolean,
                    "datetime" => FieldType::DateTime,
                    "uuid" => FieldType::Uuid,
                    "email" => FieldType::Email,
                    _ => FieldType::String,
                });
            }
            Rule::enum_type => {
                let values: Vec<String> = inner
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::enum_value)
                    .map(|p| p.as_str().to_string())
                    .collect();
                return Ok(FieldType::Enum(values));
            }
            Rule::reference_type => {
                return Ok(FieldType::Reference(inner.as_str().to_string()));
            }
            _ => {}
        }
    }
    Ok(FieldType::String)
}

/// Parse decorator ( with @validate, @auto, updated @map)
fn parse_decorator(pair: pest::iterators::Pair<Rule>) -> CompileResult<Option<Decorator>> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::decorator_type {
            for dec_inner in inner.into_inner() {
                match dec_inner.as_rule() {
                    Rule::simple_decorator => {
                        return Ok(Some(match dec_inner.as_str() {
                            "primary" => Decorator::Primary,
                            "unique" => Decorator::Unique,
                            "optional" => Decorator::Optional,
                            "auto" => Decorator::Auto,
                            "index" => Decorator::Index,
                            _ => return Ok(None),
                        }));
                    }
                    Rule::auth_decorator => {
                        let mut name: Option<String> = None;
                        let mut args: Vec<String> = Vec::new();
                        
                        for auth_inner in dec_inner.into_inner() {
                            if auth_inner.as_rule() == Rule::auth_target {
                                for target_inner in auth_inner.into_inner() {
                                    match target_inner.as_rule() {
                                        Rule::type_name | Rule::identifier => {
                                            if name.is_none() {
                                                name = Some(target_inner.as_str().to_string());
                                            }
                                        }
                                        Rule::auth_args => {
                                            for arg_inner in target_inner.into_inner() {
                                                if arg_inner.as_rule() == Rule::identifier {
                                                    args.push(arg_inner.as_str().to_string());
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        return Ok(Some(Decorator::Auth { name, args }));
                    }
                    Rule::policy_decorator => {
                        for policy_inner in dec_inner.into_inner() {
                            if policy_inner.as_rule() == Rule::policy_target {
                                let mut name_parts = Vec::new();
                                for name_inner in policy_inner.into_inner() {
                                     name_parts.push(name_inner.as_str().to_string());
                                }
                                return Ok(Some(Decorator::Policy(name_parts.join("."))));
                            }
                        }
                    }
                    Rule::map_decorator => {
                        let mut target = String::new();
                        let mut transform = MapTransform::None;
                        
                        for map_inner in dec_inner.into_inner() {
                            match map_inner.as_rule() {
                                Rule::identifier => target = map_inner.as_str().to_string(),
                                Rule::transform_type => {
                                    transform = match map_inner.as_str() {
                                        "hash" => MapTransform::Hash,
                                        _ => MapTransform::None,
                                    };
                                }
                                _ => {}
                            }
                        }
                        return Ok(Some(Decorator::Map { target, transform }));
                    }
                    Rule::api_decorator => {
                        let mut method = HttpMethod::Get;
                        let mut path = String::new();

                        for api_inner in dec_inner.into_inner() {
                            match api_inner.as_rule() {
                                Rule::http_method => {
                                    method = match api_inner.as_str() {
                                        "GET" => HttpMethod::Get,
                                        "POST" => HttpMethod::Post,
                                        "PUT" => HttpMethod::Put,
                                        "PATCH" => HttpMethod::Patch,
                                        "DELETE" => HttpMethod::Delete,
                                        _ => HttpMethod::Get,
                                    };
                                }
                                Rule::api_path => path = api_inner.as_str().to_string(),
                                _ => {}
                            }
                        }

                        return Ok(Some(Decorator::Api { method, path }));
                    }
                    Rule::validate_decorator => {
                        let mut constraints = ValidationConstraints::default();
                        
                        for validate_inner in dec_inner.into_inner() {
                            if validate_inner.as_rule() == Rule::validate_args {
                                for arg in validate_inner.into_inner() {
                                    if arg.as_rule() == Rule::validate_arg {
                                        let mut key = String::new();
                                        let mut value = String::new();
                                        
                                        for arg_inner in arg.into_inner() {
                                            match arg_inner.as_rule() {
                                                Rule::validate_key => key = arg_inner.as_str().to_string(),
                                                Rule::validate_value => {
                                                    for val_inner in arg_inner.into_inner() {
                                                        value = val_inner.as_str().trim_matches('"').to_string();
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        
                                        match key.as_str() {
                                            "min" => constraints.min = value.parse().ok(),
                                            "max" => constraints.max = value.parse().ok(),
                                            "pattern" => constraints.pattern = Some(value),
                                            "required" => constraints.required = Some(value == "true"),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        return Ok(Some(Decorator::Validate(constraints)));
                    }
                    Rule::default_decorator => {
                        for def_inner in dec_inner.into_inner() {
                            if def_inner.as_rule() == Rule::default_value {
                                let value = def_inner.as_str().trim_matches('"').to_string();
                                return Ok(Some(Decorator::Default(value)));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}

/// Parse action definition ( structured syntax)
fn parse_action(pair: pest::iterators::Pair<Rule>) -> CompileResult<Action> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut decorators = Vec::new();
    let mut input: Option<InputSection> = None;
    let mut process: Option<ProcessSection> = None;
    let mut output: Option<OutputSection> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::pre_action_decorators => {
                for dec_wrapper in inner.into_inner() {
                    if dec_wrapper.as_rule() == Rule::pre_action_decorator {
                        for dec in dec_wrapper.into_inner() {
                            if dec.as_rule() == Rule::decorator {
                                if let Some(d) = parse_decorator(dec)? {
                                    decorators.push(d);
                                }
                            }
                        }
                    }
                }
            }
            Rule::action_name => name = inner.as_str().to_string(),
            Rule::action_body => {
                for body_inner in inner.into_inner() {
                    match body_inner.as_rule() {
                        Rule::input_section => {
                            input = Some(parse_input_section(body_inner)?);
                        }
                        Rule::process_section => {
                            process = Some(parse_process_section(body_inner)?);
                        }
                        Rule::output_section => {
                            output = Some(parse_output_section(body_inner)?);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Action { name, decorators, input, process, output, location })
}

/// Parse input section
fn parse_input_section(pair: pest::iterators::Pair<Rule>) -> CompileResult<InputSection> {
    let mut fields = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::input_fields {
            for field in inner.into_inner() {
                if field.as_rule() == Rule::input_field {
                    fields.push(parse_input_field(field)?);
                }
            }
        }
    }

    Ok(InputSection { fields })
}

/// Parse input field
fn parse_input_field(pair: pest::iterators::Pair<Rule>) -> CompileResult<ActionParam> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut param_type = FieldType::String;
    let mut decorators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::field_name => name = inner.as_str().to_string(),
            Rule::field_type => param_type = parse_field_type(inner)?,
            Rule::decorator => {
                if let Some(dec) = parse_decorator(inner)? {
                    decorators.push(dec);
                }
            }
            _ => {}
        }
    }

    Ok(ActionParam { name, param_type, decorators, location })
}

/// Parse process section
fn parse_process_section(pair: pest::iterators::Pair<Rule>) -> CompileResult<ProcessSection> {
    let mut steps = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::process_step {
             let inner_step = inner.into_inner().next().unwrap();
             match inner_step.as_rule() {
                 Rule::derive_statement => {
                     steps.push(ProcessStep::Derive(parse_derive_statement(inner_step)?));
                 }
                 Rule::mutate_block => {
                     steps.push(ProcessStep::Mutate(parse_mutate_block(inner_step)?));
                 }
                 Rule::delete_statement => {
                     steps.push(ProcessStep::Delete(parse_delete_statement(inner_step)?));
                 }
                 _ => {}
             }
        }
    }

    Ok(ProcessSection { steps })
}

/// Parse derive statement
fn parse_derive_statement(pair: pest::iterators::Pair<Rule>) -> CompileResult<DeriveStatement> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut value = DeriveValue::Literal(LiteralValue::String(String::new()));

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                if name.is_empty() {
                    name = inner.as_str().to_string();
                } else {
                    value = DeriveValue::Identifier(inner.as_str().to_string());
                }
            }
            Rule::derive_expr => {
                value = parse_derive_expr(inner)?;
            }
            _ => {}
        }
    }

    Ok(DeriveStatement { name, value, location })
}

/// Parse mutate block
fn parse_mutate_block(pair: pest::iterators::Pair<Rule>) -> CompileResult<MutateBlock> {
    let location = get_location(&pair);
    let mut entity = String::new();
    let mut predicate = None;
    let mut setters = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::type_name => entity = inner.as_str().to_string(),
            Rule::predicate => predicate = Some(parse_predicate(inner)?),
            Rule::mutate_setters => {
                for setter in inner.into_inner() {
                    if setter.as_rule() == Rule::mutate_setter {
                        setters.push(parse_mutate_setter(setter)?);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(MutateBlock { entity, predicate, setters, location })
}

/// Parse mutate setter
fn parse_mutate_setter(pair: pest::iterators::Pair<Rule>) -> CompileResult<MutateSetter> {
    let location = get_location(&pair);
    let mut field = String::new();
    let mut value = DeriveValue::Literal(LiteralValue::String(String::new()));

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::field_name => field = inner.as_str().to_string(),
            Rule::derive_expr => value = parse_derive_expr(inner)?,
            _ => {}
        }
    }

    Ok(MutateSetter { field, value, location })
}

/// Parse delete statement
fn parse_delete_statement(pair: pest::iterators::Pair<Rule>) -> CompileResult<DeleteStatement> {
    let location = get_location(&pair);
    let mut entity = String::new();
    let mut predicate = Predicate {
        field: FieldReference::InputField(String::new()),
        operator: CompareOp::Equal,
        value: FieldReference::InputField(String::new()),
    };

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::type_name => entity = inner.as_str().to_string(),
            Rule::predicate => predicate = parse_predicate(inner)?,
            _ => {}
        }
    }

    Ok(DeleteStatement { entity, predicate, location })
}

/// Parse derive expression (v0.3)
fn parse_derive_expr(pair: pest::iterators::Pair<Rule>) -> CompileResult<DeriveValue> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::compute_expr => {
                return parse_compute_expr(inner);
            }
            Rule::select_expr => {
                return parse_select_expr(inner);
            }
            Rule::system_expr => {
                return parse_system_expr(inner);
            }
            Rule::dotted_path => {
                let path: Vec<String> = inner.into_inner()
                    .filter(|p| p.as_rule() == Rule::path_segment)
                    .map(|p| p.as_str().to_string())
                    .collect();
                return Ok(DeriveValue::FieldAccess { path });
            }
            Rule::literal => {
                for lit_inner in inner.into_inner() {
                    return Ok(DeriveValue::Literal(parse_literal_value(lit_inner)?));
                }
            }
            Rule::identifier => {
                return Ok(DeriveValue::Identifier(inner.as_str().to_string()));
            }
            _ => {}
        }
    }
    Ok(DeriveValue::Literal(LiteralValue::String(String::new())))
}

/// Parse compute expression: compute function_name(args)
fn parse_compute_expr(pair: pest::iterators::Pair<Rule>) -> CompileResult<DeriveValue> {
    let mut function = String::new();
    let mut args = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                function = inner.as_str().to_string();
            }
            Rule::function_args => {
                for arg in inner.into_inner() {
                    if arg.as_rule() == Rule::function_arg {
                        args.push(parse_function_arg(arg)?);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(DeriveValue::Compute { function, args })
}

/// Parse select expression: select Entity where predicate
fn parse_select_expr(pair: pest::iterators::Pair<Rule>) -> CompileResult<DeriveValue> {
    let mut entity = String::new();
    let mut predicate = Predicate {
        field: FieldReference::InputField(String::new()),
        operator: CompareOp::Equal,
        value: FieldReference::InputField(String::new()),
    };

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::type_name => {
                entity = inner.as_str().to_string();
            }
            Rule::predicate => {
                predicate = parse_predicate(inner)?;
            }
            _ => {}
        }
    }

   Ok(DeriveValue::Select { entity, predicate })
}

/// Parse system call expression: system namespace.capability(args)
fn parse_system_expr(pair: pest::iterators::Pair<Rule>) -> CompileResult<DeriveValue> {
    let mut namespace = String::new();
    let mut capability = String::new();
    let mut args = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::namespace => {
                namespace = inner.as_str().to_string();
            }
            Rule::identifier => {
                capability = inner.as_str().to_string();
            }
            Rule::function_args => {
                for arg in inner.into_inner() {
                    if arg.as_rule() == Rule::function_arg {
                        args.push(parse_function_arg(arg)?);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(DeriveValue::SystemCall { namespace, capability, args })
}

/// Parse predicate: field_ref op field_ref
fn parse_predicate(pair: pest::iterators::Pair<Rule>) -> CompileResult<Predicate> {
    let mut items: Vec<pest::iterators::Pair<Rule>> = pair.into_inner().collect();
    
    if items.len() < 3 {
        return Err(CompileError::parse("Invalid predicate: needs field operator field", 0, 0));
    }

    let left = parse_field_ref(items.remove(0))?;
    let op_str = items.remove(0).as_str();
    let right = parse_field_ref(items.remove(0))?;

    let operator = match op_str {
        "==" => CompareOp::Equal,
        "!=" => CompareOp::NotEqual,
        "<" => CompareOp::Less,
        ">" => CompareOp::Greater,
        _ => return Err(CompileError::parse(format!("Unknown operator: {}", op_str), 0, 0)),
    };

    Ok(Predicate { field: left, operator, value: right })
}

/// Parse field reference for predicates
fn parse_field_ref(pair: pest::iterators::Pair<Rule>) -> CompileResult<FieldReference> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::dotted_path => {
                let path: Vec<String> = inner.into_inner()
                    .filter(|p| p.as_rule() == Rule::path_segment)
                    .map(|p| p.as_str().to_string())
                    .collect();
                
                // Check if this is input.field reference
                if path.len() == 2 && path[0] == "input" {
                    return Ok(FieldReference::InputField(path[1].clone()));
                }
                // Check if this is a derived field reference (name.field)
                else if path.len() == 2 {
                    return Ok(FieldReference::DerivedField {
                        name: path[0].clone(),
                        field: path[1].clone(),
                    });
                }
                // Single identifier
                else if path.len() == 1 {
                    return Ok(FieldReference::InputField(path[0].clone()));
                }
            }
            Rule::identifier => {
                return Ok(FieldReference::InputField(inner.as_str().to_string()));
            }
            Rule::literal => {
                for lit_inner in inner.into_inner() {
                    return Ok(FieldReference::Literal(parse_literal_value(lit_inner)?));
                }
            }
            _ => {}
        }
    }
    Err(CompileError::parse("Invalid field reference", 0, 0))
}

/// Parse function argument
fn parse_function_arg(pair: pest::iterators::Pair<Rule>) -> CompileResult<crate::ast::FunctionArg> {
    use crate::ast::FunctionArg;
    
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::type_name => {
                return Ok(FunctionArg::TypeName(inner.as_str().to_string()));
            }
            Rule::dotted_path => {
                let path: Vec<String> = inner.into_inner()
                    .filter(|p| p.as_rule() == Rule::path_segment)
                    .map(|p| p.as_str().to_string())
                    .collect();
                return Ok(FunctionArg::FieldAccess { path });
            }
            Rule::literal => {
                for lit_inner in inner.into_inner() {
                    return Ok(FunctionArg::Literal(parse_literal_value(lit_inner)?));
                }
            }
            Rule::identifier => {
                return Ok(FunctionArg::Identifier(inner.as_str().to_string()));
            }
            _ => {}
        }
    }
    Ok(FunctionArg::Identifier(String::new()))
}

/// Parse literal value
fn parse_literal_value(pair: pest::iterators::Pair<Rule>) -> CompileResult<LiteralValue> {
    match pair.as_rule() {
        Rule::string_literal => {
            let s = pair.as_str();
            Ok(LiteralValue::String(s[1..s.len()-1].to_string()))
        }
        Rule::number_literal => {
            let value: f64 = pair.as_str().parse().unwrap_or(0.0);
            Ok(LiteralValue::Number(value))
        }
        Rule::boolean_literal => {
            Ok(LiteralValue::Boolean(pair.as_str() == "true"))
        }
        _ => Ok(LiteralValue::String(String::new())),
    }
}

/// Parse output section
fn parse_output_section(pair: pest::iterators::Pair<Rule>) -> CompileResult<OutputSection> {
    let mut entity = String::new();
    let mut fields = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::type_projection {
            for proj_inner in inner.into_inner() {
                match proj_inner.as_rule() {
                    Rule::type_name => entity = proj_inner.as_str().to_string(),
                    Rule::projection_fields => {
                        for field in proj_inner.into_inner() {
                            if field.as_rule() == Rule::identifier {
                                fields.push(field.as_str().to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(OutputSection { entity, fields })
}

/// Parse rule definition
fn parse_rule(pair: pest::iterators::Pair<Rule>) -> CompileResult<crate::ast::Rule> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut condition = Expression::Literal(LiteralValue::Boolean(true));
    let mut consequence = Consequence::Log("".to_string());

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::rule_name => name = inner.as_str().to_string(),
            Rule::when_clause => {
                for when_inner in inner.into_inner() {
                    if when_inner.as_rule() == Rule::expression {
                        condition = parse_expression(when_inner)?;
                    }
                }
            }
            Rule::then_clause => {
                for then_inner in inner.into_inner() {
                    if then_inner.as_rule() == Rule::consequence {
                        consequence = parse_consequence(then_inner)?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(crate::ast::Rule { name, condition, consequence, location })
}

/// Parse policy definition
fn parse_policy(pair: pest::iterators::Pair<Rule>) -> CompileResult<Policy> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut subject = String::new();
    let mut require = Expression::Literal(LiteralValue::Boolean(false));

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::policy_name => name = inner.as_str().to_string(),
            Rule::subject_name => subject = inner.as_str().to_string(),
            Rule::expression => require = parse_expression(inner)?,
            _ => {}
        }
    }

    Ok(Policy { name, subject, require, location })
}

/// Parse expression
fn parse_expression(pair: pest::iterators::Pair<Rule>) -> CompileResult<Expression> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::or_expr {
            return parse_or_expr(inner);
        }
    }
    Err(CompileError::parse("Empty expression", 0, 0))
}

/// Parse OR expression
fn parse_or_expr(pair: pest::iterators::Pair<Rule>) -> CompileResult<Expression> {
    let mut result: Option<Expression> = None;

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::and_expr {
            let expr = parse_and_expr(inner)?;
            result = match result {
                None => Some(expr),
                Some(left) => Some(Expression::Logical {
                    left: Box::new(left),
                    operator: LogicalOperator::Or,
                    right: Box::new(expr),
                }),
            };
        }
    }

    result.ok_or_else(|| CompileError::parse("Empty OR expression", 0, 0))
}

/// Parse AND expression
fn parse_and_expr(pair: pest::iterators::Pair<Rule>) -> CompileResult<Expression> {
    let mut result: Option<Expression> = None;

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::not_expr {
            let expr = parse_not_expr(inner)?;
            result = match result {
                None => Some(expr),
                Some(left) => Some(Expression::Logical {
                    left: Box::new(left),
                    operator: LogicalOperator::And,
                    right: Box::new(expr),
                }),
            };
        }
    }

    result.ok_or_else(|| CompileError::parse("Empty AND expression", 0, 0))
}

/// Parse NOT expression
fn parse_not_expr(pair: pest::iterators::Pair<Rule>) -> CompileResult<Expression> {
    let s = pair.as_str();
    let is_not = s.trim().starts_with("not ");
    
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::comparison {
            let expr = parse_comparison(inner)?;
            return if is_not {
                Ok(Expression::Not(Box::new(expr)))
            } else {
                Ok(expr)
            };
        }
    }

    Err(CompileError::parse("Invalid NOT expression", 0, 0))
}

/// Parse comparison
fn parse_comparison(pair: pest::iterators::Pair<Rule>) -> CompileResult<Expression> {
    let mut items: Vec<pest::iterators::Pair<Rule>> = pair.into_inner().collect();

    if items.is_empty() {
        return Err(CompileError::parse("Empty comparison", 0, 0));
    }

    if items.len() == 1 {
        return parse_primary(items.remove(0));
    }

    if items.len() >= 2 {
        let left = parse_primary(items.remove(0))?;
        if !items.is_empty() {
            let op_pair = items.remove(0);
            if !items.is_empty() {
                let right = parse_primary(items.remove(0))?;
                let operator = match op_pair.as_str() {
                    "==" => BinaryOperator::Equal,
                    "!=" => BinaryOperator::NotEqual,
                    ">" => BinaryOperator::GreaterThan,
                    "<" => BinaryOperator::LessThan,
                    ">=" => BinaryOperator::GreaterEqual,
                    "<=" => BinaryOperator::LessEqual,
                    _ => BinaryOperator::Equal,
                };
                return Ok(Expression::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                });
            }
        }
        return Ok(left);
    }

    Err(CompileError::parse("Invalid comparison", 0, 0))
}

/// Parse primary expression
fn parse_primary(pair: pest::iterators::Pair<Rule>) -> CompileResult<Expression> {
    match pair.as_rule() {
        Rule::primary => {
            for inner in pair.into_inner() {
                return parse_primary(inner);
            }
            Err(CompileError::parse("Empty primary", 0, 0))
        }
        Rule::paren_expr => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::expression {
                    return parse_expression(inner);
                }
            }
            Err(CompileError::parse("Empty paren", 0, 0))
        }
        Rule::field_access => {
            let mut entity = String::new();
            let mut field = String::new();
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::subject_prefix => entity = "subject".to_string(),
                    Rule::entity_ref => entity = inner.as_str().to_string(),
                    Rule::field_ref => field = inner.as_str().to_string(),
                    _ => {}
                }
            }
            Ok(Expression::FieldAccess { entity, field })
        }
        Rule::literal => {
            for inner in pair.into_inner() {
                return parse_literal(inner);
            }
            Err(CompileError::parse("Empty literal", 0, 0))
        }
        Rule::identifier => Ok(Expression::Identifier(pair.as_str().to_string())),
        _ => Err(CompileError::parse(format!("Unexpected: {:?}", pair.as_rule()), 0, 0)),
    }
}

/// Parse literal
fn parse_literal(pair: pest::iterators::Pair<Rule>) -> CompileResult<Expression> {
    match pair.as_rule() {
        Rule::string_literal => {
            let s = pair.as_str();
            Ok(Expression::Literal(LiteralValue::String(s[1..s.len()-1].to_string())))
        }
        Rule::number_literal => {
            let value: f64 = pair.as_str().parse().unwrap_or(0.0);
            Ok(Expression::Literal(LiteralValue::Number(value)))
        }
        Rule::boolean_literal => {
            Ok(Expression::Literal(LiteralValue::Boolean(pair.as_str() == "true")))
        }
        _ => Err(CompileError::parse("Unknown literal", 0, 0)),
    }
}

/// Parse consequence
fn parse_consequence(pair: pest::iterators::Pair<Rule>) -> CompileResult<Consequence> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::reject_call => {
                for rej_inner in inner.into_inner() {
                    if rej_inner.as_rule() == Rule::string_literal {
                        let s = rej_inner.as_str();
                        return Ok(Consequence::Reject(s[1..s.len()-1].to_string()));
                    }
                }
            }
            Rule::log_call => {
                for log_inner in inner.into_inner() {
                    if log_inner.as_rule() == Rule::string_literal {
                        let s = log_inner.as_str();
                        return Ok(Consequence::Log(s[1..s.len()-1].to_string()));
                    }
                }
            }
            Rule::action_call => {
                let mut action_name = String::new();
                let mut args = Vec::new();

                for call_inner in inner.into_inner() {
                    match call_inner.as_rule() {
                        Rule::identifier => action_name = call_inner.as_str().to_string(),
                        Rule::call_args => {
                            for arg in call_inner.into_inner() {
                                if arg.as_rule() == Rule::call_arg {
                                    for arg_inner in arg.into_inner() {
                                        if arg_inner.as_rule() == Rule::expression {
                                            args.push(parse_expression(arg_inner)?);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                return Ok(Consequence::ActionCall { action: action_name, args });
            }
            _ => {}
        }
    }

    Err(CompileError::parse("Invalid consequence", 0, 0))
}

/// Get source location
fn get_location(pair: &pest::iterators::Pair<Rule>) -> SourceLocation {
    let span = pair.as_span();
    let (line, column) = span.start_pos().line_col();
    SourceLocation::with_span(line, column, span.start(), span.end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_entity() {
        let source = "entity User:\n    id: string @primary\n    name: string\n    age: number\n";
        let result = parse_intent(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.entities.len(), 1);
        assert_eq!(file.entities[0].name, "User");
        assert_eq!(file.entities[0].fields.len(), 3);
    }

    #[test]
    fn test_parse_entity_with_new_types() {
        let source = "entity User:\n    id: uuid @primary\n    email: email @unique\n    created_at: datetime @auto\n";
        let result = parse_intent(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.entities[0].fields[0].field_type, FieldType::Uuid);
        assert_eq!(file.entities[0].fields[1].field_type, FieldType::Email);
    }

    #[test]
    fn test_parse_entity_with_ref_and_list() {
        let source = "entity Post:\n    id: uuid @primary\n    author: ref<User>\n    tags: list<string>\n";
        let result = parse_intent(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        assert!(matches!(file.entities[0].fields[1].field_type, FieldType::Ref(_)));
        assert!(matches!(file.entities[0].fields[2].field_type, FieldType::List(_)));
    }

    #[test]
    fn test_parse_v01_action() {
        let source = r#"@api POST /signup
action signup:
    input:
        email: email
        password: string @map(hashed_password, hash)
    output: User(id, email)
"#;
        let result = parse_intent(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.actions.len(), 1);
        assert_eq!(file.actions[0].name, "signup");
        assert!(file.actions[0].input.is_some());
        assert!(file.actions[0].output.is_some());
    }
}
