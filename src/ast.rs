// Intent Compiler - AST Definitions
// These types represent the parsed structure of Intent Definition Language files

use serde::{Deserialize, Serialize};

/// Root node representing an entire .intent file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentFile {
    pub entities: Vec<Entity>,
    pub actions: Vec<Action>,
    pub rules: Vec<Rule>,
    /// Source file path for error reporting
    pub source_path: Option<String>,
}

/// Entity definition - represents a data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub fields: Vec<Field>,
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

/// Supported field types in IDL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    DateTime,
    /// Enum type with possible values: status: active | inactive
    Enum(Vec<String>),
    /// Reference to another entity: author: User
    Reference(String),
    /// Array of a type: tags: [string]
    Array(Box<FieldType>),
    /// Optional type: email: string?
    Optional(Box<FieldType>),
}

/// Field and action decorators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Decorator {
    /// @primary - marks field as primary key
    Primary,
    /// @unique - marks field as unique
    Unique,
    /// @optional - marks field as optional
    Optional,
    /// @default(value) - sets default value
    Default(String),
    /// @api METHOD /path - defines API endpoint
    Api { method: HttpMethod, path: String },
    /// @returns TypeName - specifies return type
    Returns(String),
    /// @auth - requires authentication
    Auth,
    /// @index - creates database index
    Index,
    /// @hash - hashes the value (e.g. password)
    Hash,
    /// @map(name) - maps field to another name
    Map(String),
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

/// Action definition - represents an operation/endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub params: Vec<ActionParam>,
    pub decorators: Vec<Decorator>,
    pub location: SourceLocation,
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
            source_path: None,
        }
    }

    /// Find an entity by name
    pub fn find_entity(&self, name: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.name == name)
    }

    /// Find an action by name
    pub fn find_action(&self, name: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.name == name)
    }

    /// Get all entity names for validation
    pub fn entity_names(&self) -> Vec<&str> {
        self.entities.iter().map(|e| e.name.as_str()).collect()
    }

    /// Get all action names for validation
    pub fn action_names(&self) -> Vec<&str> {
        self.actions.iter().map(|a| a.name.as_str()).collect()
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
            FieldType::Enum(values) => format!("Literal[{}]", values.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(", ")),
            FieldType::Reference(name) => name.clone(),
            FieldType::Array(inner) => format!("list[{}]", inner.to_python_type()),
            FieldType::Optional(inner) => format!("Optional[{}]", inner.to_python_type()),
        }
    }

    /// Convert to SQLAlchemy column type
    pub fn to_sqlalchemy_type(&self) -> String {
        match self {
            FieldType::String => "String".to_string(),
            FieldType::Number => "Float".to_string(),
            FieldType::Boolean => "Boolean".to_string(),
            FieldType::DateTime => "DateTime".to_string(),
            FieldType::Enum(values) => format!("Enum({})", values.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(", ")),
            FieldType::Reference(name) => format!("ForeignKey(\"{}.id\")", name.to_lowercase()),
            FieldType::Array(_) => "JSON".to_string(), // Arrays stored as JSON
            FieldType::Optional(inner) => inner.to_sqlalchemy_type(),
        }
    }
}
