// Intent Compiler - Error Types
// All error types for parsing, validation, and code generation

use colored::Colorize;
use thiserror::Error;

use crate::ast::SourceLocation;

/// Main compiler error type
#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        location: SourceLocation,
        snippet: Option<String>,
    },

    #[error("Validation error: {message}")]
    ValidationError {
        message: String,
        location: SourceLocation,
        hint: Option<String>,
    },

    #[error("Code generation error: {message}")]
    CodeGenError { message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Multiple errors occurred")]
    MultipleErrors(Vec<CompileError>),
}

impl CompileError {
    /// Create a parse error with location
    pub fn parse(message: impl Into<String>, line: usize, column: usize) -> Self {
        CompileError::ParseError {
            message: message.into(),
            location: SourceLocation::new(line, column),
            snippet: None,
        }
    }

    /// Create a parse error with a code snippet
    pub fn parse_with_snippet(
        message: impl Into<String>,
        line: usize,
        column: usize,
        snippet: impl Into<String>,
    ) -> Self {
        CompileError::ParseError {
            message: message.into(),
            location: SourceLocation::new(line, column),
            snippet: Some(snippet.into()),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>, location: SourceLocation) -> Self {
        CompileError::ValidationError {
            message: message.into(),
            location,
            hint: None,
        }
    }

    /// Create a validation error with a hint
    pub fn validation_with_hint(
        message: impl Into<String>,
        location: SourceLocation,
        hint: impl Into<String>,
    ) -> Self {
        CompileError::ValidationError {
            message: message.into(),
            location,
            hint: Some(hint.into()),
        }
    }

    /// Create a code generation error
    pub fn codegen(message: impl Into<String>) -> Self {
        CompileError::CodeGenError {
            message: message.into(),
        }
    }

    /// Create a template error
    pub fn template(message: impl Into<String>) -> Self {
        CompileError::TemplateError(message.into())
    }

    /// Format error for terminal output with colors
    pub fn format_colored(&self, source: Option<&str>) -> String {
        match self {
            CompileError::ParseError {
                message,
                location,
                snippet,
            } => {
                let mut output = format!(
                    "{}: {}\n",
                    "error".red().bold(),
                    message.white().bold()
                );
                output.push_str(&format!(
                    "  {} {}:{}\n",
                    "-->".blue().bold(),
                    location.line,
                    location.column
                ));

                if let Some(snip) = snippet {
                    output.push_str(&format_snippet(snip, location.line, location.column));
                } else if let Some(src) = source {
                    if let Some(line_content) = src.lines().nth(location.line.saturating_sub(1)) {
                        output.push_str(&format_snippet(line_content, location.line, location.column));
                    }
                }

                output
            }
            CompileError::ValidationError {
                message,
                location,
                hint,
            } => {
                let mut output = format!(
                    "{}: {}\n",
                    "error".red().bold(),
                    message.white().bold()
                );
                output.push_str(&format!(
                    "  {} {}:{}\n",
                    "-->".blue().bold(),
                    location.line,
                    location.column
                ));

                if let Some(h) = hint {
                    output.push_str(&format!(
                        "  {} {}\n",
                        "hint:".cyan().bold(),
                        h
                    ));
                }

                output
            }
            CompileError::CodeGenError { message } => {
                format!(
                    "{}: {}\n",
                    "error".red().bold(),
                    message.white().bold()
                )
            }
            CompileError::IoError(e) => {
                format!(
                    "{}: {}\n",
                    "io error".red().bold(),
                    e.to_string().white()
                )
            }
            CompileError::TemplateError(msg) => {
                format!(
                    "{}: {}\n",
                    "template error".red().bold(),
                    msg.white()
                )
            }
            CompileError::MultipleErrors(errors) => {
                let mut output = String::new();
                for (i, err) in errors.iter().enumerate() {
                    if i > 0 {
                        output.push('\n');
                    }
                    output.push_str(&err.format_colored(source));
                }
                output.push_str(&format!(
                    "\n{}: {} errors generated\n",
                    "error".red().bold(),
                    errors.len()
                ));
                output
            }
        }
    }
}

/// Format a code snippet with line number and pointer
fn format_snippet(line_content: &str, line_num: usize, column: usize) -> String {
    let line_num_str = line_num.to_string();
    let padding = " ".repeat(line_num_str.len());

    let mut output = format!("{}|\n", padding.blue());
    output.push_str(&format!(
        "{} {} {}\n",
        line_num_str.blue().bold(),
        "|".blue(),
        line_content
    ));
    output.push_str(&format!(
        "{}{} {}",
        padding.blue(),
        "|".blue(),
        " ".repeat(column.saturating_sub(1))
    ));
    output.push_str(&format!("{}\n", "^".red().bold()));

    output
}

/// Result type for compiler operations
pub type CompileResult<T> = Result<T, CompileError>;

/// Validation warning (non-fatal)
#[derive(Debug, Clone)]
pub struct Warning {
    pub message: String,
    pub location: SourceLocation,
    pub hint: Option<String>,
}

impl Warning {
    pub fn new(message: impl Into<String>, location: SourceLocation) -> Self {
        Self {
            message: message.into(),
            location,
            hint: None,
        }
    }

    pub fn with_hint(message: impl Into<String>, location: SourceLocation, hint: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location,
            hint: Some(hint.into()),
        }
    }

    pub fn format_colored(&self) -> String {
        let mut output = format!(
            "{}: {}\n",
            "warning".yellow().bold(),
            self.message.white()
        );
        output.push_str(&format!(
            "  {} {}:{}\n",
            "-->".blue().bold(),
            self.location.line,
            self.location.column
        ));

        if let Some(h) = &self.hint {
            output.push_str(&format!(
                "  {} {}\n",
                "hint:".cyan().bold(),
                h
            ));
        }

        output
    }
}
