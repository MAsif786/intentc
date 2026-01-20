// Intent Compiler - AST Definitions
// These types represent the parsed structure of Intent Definition Language files
// v0.1 Spec Implementation

use serde::{Deserialize, Serialize};

/// Root node representing an entire .intent file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentFile {
    pub entities: Vec<Entity>,
    pub actions: Vec<Action>,
    pub rules: Vec<Rule>,
    pub policies: Vec<Policy>,
    /// Source file path for error reporting
    pub source_path: Option<String>,
}

/// Entity definition - represents a data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub fields: Vec<Field>,
    pub policies: Vec<Policy>,
    pub location: SourceLocation,
}

/// Field within an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub decorators: Vec<Decorator>,
    pub location: SourceLocation,
}

/// Supported field types in IDL (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    DateTime,
    Uuid,   // v0.1: UUID type
    Email,  // v0.1: Email type with format validation
    /// Enum type with possible values: status: active | inactive
    Enum(Vec<String>),
    /// Reference to another entity: author: User
    Reference(String),
    /// v0.1 ref type: ref<User>
    Ref(String),
    /// Array of a type: tags: [string]
    Array(Box<FieldType>),
    /// v0.1 list type: list<string>
    List(Box<FieldType>),
    /// Optional type: email: string?
    Optional(Box<FieldType>),
}

/// Field and action decorators (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Decorator {
    /// @primary - marks field as primary key
    Primary,
    /// @unique - marks field as unique
    Unique,
    /// @optional - marks field as optional
    Optional,
    /// @auto - auto-generated field (e.g., timestamps, UUIDs)
    Auto,
    /// @index - creates database index
    Index,
    /// @default(value) - sets default value
    Default(String),
    /// @validate(min:, max:, pattern:) - validation constraints (v0.1)
    Validate(ValidationConstraints),
    /// @api METHOD /path - defines API endpoint
    Api { method: HttpMethod, path: String },
    /// @auth or @auth(Entity) or @auth(action(args)) - requires authentication
    Auth { name: Option<String>, args: Vec<String> },
    /// @map(target, transform) - maps field with optional transform (v0.1)
    Map { target: String, transform: MapTransform },
    /// @policy(Name) - enforces a policy
    Policy(String),
}

/// Validation constraints for @validate decorator (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ValidationConstraints {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub pattern: Option<String>,
    pub required: Option<bool>,
}

/// Map transform types for @map decorator (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MapTransform {
    None,
    Hash,
}

impl Default for MapTransform {
    fn default() -> Self {
        MapTransform::None
    }
}

/// HTTP methods for API decorators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

/// Action definition - represents an operation/endpoint (v0.1 structured)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    /// Pre-action decorators: @api, @auth
    pub decorators: Vec<Decorator>,
    /// Input section with parameters
    pub input: Option<InputSection>,
    /// Process section with derive statements
    pub process: Option<ProcessSection>,
    /// Output section with entity projection
    pub output: Option<OutputSection>,
    pub location: SourceLocation,
}

/// Input section for action (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSection {
    pub fields: Vec<ActionParam>,
}

/// Process section for action (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSection {
    pub derives: Vec<DeriveStatement>,
}

/// Derive statement in process section (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeriveStatement {
    pub name: String,
    pub value: DeriveValue,
    pub location: SourceLocation,
}

/// Value for derive statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeriveValue {
    Literal(LiteralValue),
    FieldAccess { path: Vec<String> },
    Identifier(String),
    FunctionCall { name: String, args: Vec<FunctionArg> },
}

/// Argument for function call in derive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionArg {
    TypeName(String),
    Identifier(String),
    FieldAccess { path: Vec<String> },
    Literal(LiteralValue),
}

/// Output section for action (v0.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSection {
    pub entity: String,
    pub fields: Vec<String>,
}

/// Parameter for an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionParam {
    pub name: String,
    pub param_type: FieldType,
    pub decorators: Vec<Decorator>,
    pub location: SourceLocation,
}

/// Rule definition - represents business logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub condition: Expression,
    pub consequence: Consequence,
    pub location: SourceLocation,
}

/// Policy definition - authorization constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub subject: String,
    pub require: Expression,
    pub location: SourceLocation,
}


/// Expression for rule conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    /// Binary comparison: User.age > 18
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    /// Logical operations: condition1 and condition2
    Logical {
        left: Box<Expression>,
        operator: LogicalOperator,
        right: Box<Expression>,
    },
    /// Negation: not condition
    Not(Box<Expression>),
    /// Field access: User.age
    FieldAccess { entity: String, field: String },
    /// Literal values: 18, "active", true
    Literal(LiteralValue),
    /// Identifier: variable or enum value
    Identifier(String),
}

/// Binary comparison operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinaryOperator {
    Equal,        // ==
    NotEqual,     // !=
    GreaterThan,  // >
    LessThan,     // <
    GreaterEqual, // >=
    LessEqual,    // <=
}

impl std::fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOperator::Equal => write!(f, "=="),
            BinaryOperator::NotEqual => write!(f, "!="),
            BinaryOperator::GreaterThan => write!(f, ">"),
            BinaryOperator::LessThan => write!(f, "<"),
            BinaryOperator::GreaterEqual => write!(f, ">="),
            BinaryOperator::LessEqual => write!(f, "<="),
        }
    }
}

/// Logical operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

impl std::fmt::Display for LogicalOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogicalOperator::And => write!(f, "and"),
            LogicalOperator::Or => write!(f, "or"),
        }
    }
}

/// Literal values in expressions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// Rule consequence - what happens when condition is true
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Consequence {
    /// Call an action: enable_premium(User.id)
    ActionCall { action: String, args: Vec<Expression> },
    /// Reject with message: reject("Must be 18+")
    Reject(String),
    /// Log a message: log("User enabled")
    Log(String),
}

/// Source location for error reporting
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub span: Option<(usize, usize)>, // start, end positions
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            span: None,
        }
    }

    pub fn with_span(line: usize, column: usize, start: usize, end: usize) -> Self {
        Self {
            line,
            column,
            span: Some((start, end)),
        }
    }
}

impl IntentFile {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            actions: Vec::new(),
            rules: Vec::new(),
            policies: Vec::new(),
            source_path: None,
        }
    }

    /// Find an entity by name
    pub fn find_entity(&self, name: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.name == name)
    }

}

impl Default for IntentFile {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldType {
    /// Convert to Python type string
    pub fn to_python_type(&self) -> String {
        match self {
            FieldType::String => "str".to_string(),
            FieldType::Number => "float".to_string(),
            FieldType::Boolean => "bool".to_string(),
            FieldType::DateTime => "datetime".to_string(),
            FieldType::Uuid => "UUID".to_string(),
            FieldType::Email => "EmailStr".to_string(),
            FieldType::Enum(values) => format!("Literal[{}]", values.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(", ")),
            FieldType::Reference(name) => name.clone(),
            FieldType::Ref(name) => name.clone(),
            FieldType::Array(inner) => format!("list[{}]", inner.to_python_type()),
            FieldType::List(inner) => format!("list[{}]", inner.to_python_type()),
            FieldType::Optional(inner) => format!("Optional[{}]", inner.to_python_type()),
        }
    }

}
