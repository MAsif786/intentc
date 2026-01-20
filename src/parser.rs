// Intent Compiler - Parser
// Transforms .intent files into typed AST

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
            Rule::entity_def => file.entities.push(parse_entity(inner)?),
            Rule::action_def => file.actions.push(parse_action(inner)?),
            Rule::rule_def => file.rules.push(parse_rule(inner)?),
            _ => {}
        }
    }
    Ok(())
}

/// Parse entity definition
fn parse_entity(pair: pest::iterators::Pair<Rule>) -> CompileResult<Entity> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut fields = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::entity_name => name = inner.as_str().to_string(),
            Rule::entity_fields => {
                for field_wrapper in inner.into_inner() {
                    if field_wrapper.as_rule() == Rule::entity_field {
                        for field_inner in field_wrapper.into_inner() {
                            if field_inner.as_rule() == Rule::field_def {
                                fields.push(parse_field(field_inner)?);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Entity { name, fields, location })
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

/// Parse field type
fn parse_field_type(pair: pest::iterators::Pair<Rule>) -> CompileResult<FieldType> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
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

/// Parse base type
fn parse_base_type(pair: pest::iterators::Pair<Rule>) -> CompileResult<FieldType> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::primitive_type => {
                return Ok(match inner.as_str() {
                    "string" => FieldType::String,
                    "number" => FieldType::Number,
                    "boolean" => FieldType::Boolean,
                    "datetime" => FieldType::DateTime,
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

/// Parse decorator
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
                            "auth" => Decorator::Auth,
                            "index" => Decorator::Index,
                            "hash" => Decorator::Hash,
                            _ => return Ok(None),
                        }));
                    }
                    Rule::map_decorator => {
                        for map_inner in dec_inner.into_inner() {
                            if map_inner.as_rule() == Rule::identifier {
                                return Ok(Some(Decorator::Map(map_inner.as_str().to_string())));
                            }
                        }
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
                    Rule::returns_decorator => {
                        for ret_inner in dec_inner.into_inner() {
                            if ret_inner.as_rule() == Rule::type_name {
                                return Ok(Some(Decorator::Returns(ret_inner.as_str().to_string())));
                            }
                        }
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

/// Parse action definition
fn parse_action(pair: pest::iterators::Pair<Rule>) -> CompileResult<Action> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut params = Vec::new();
    let mut decorators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::action_name => name = inner.as_str().to_string(),
            Rule::action_lines => {
                for line in inner.into_inner() {
                    if line.as_rule() == Rule::action_line {
                        for line_inner in line.into_inner() {
                            match line_inner.as_rule() {
                                Rule::action_param => params.push(parse_action_param(line_inner)?),
                                Rule::decorator => {
                                    if let Some(dec) = parse_decorator(line_inner)? {
                                        decorators.push(dec);
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

    Ok(Action { name, params, decorators, location })
}

/// Parse action parameter
fn parse_action_param(pair: pest::iterators::Pair<Rule>) -> CompileResult<ActionParam> {
    let location = get_location(&pair);
    let mut name = String::new();
    let mut param_type = FieldType::String;
    let mut decorators = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::param_name => name = inner.as_str().to_string(),
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
    fn test_parse_entity_with_enum() {
        let source = "entity User:\n    status: active | inactive | suspended\n";
        let result = parse_intent(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        let field = &file.entities[0].fields[0];
        match &field.field_type {
            FieldType::Enum(values) => {
                assert_eq!(values.len(), 3);
                assert!(values.contains(&"active".to_string()));
            }
            _ => panic!("Expected enum type"),
        }
    }

    #[test]
    fn test_parse_action() {
        let source = "action create_user:\n    name: string\n    age: number\n    @api POST /users\n    @returns User\n";
        let result = parse_intent(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.actions.len(), 1);
        assert_eq!(file.actions[0].name, "create_user");
        assert_eq!(file.actions[0].params.len(), 2);
    }

    #[test]
    fn test_parse_rule() {
        let source = "rule ValidateAge:\n    when User.age < 18\n    then reject(\"Must be 18 or older\")\n";
        let result = parse_intent(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.rules.len(), 1);
        assert_eq!(file.rules[0].name, "ValidateAge");
    }
}
