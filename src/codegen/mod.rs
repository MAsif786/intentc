// Intent Compiler - Code Generator Module
// Trait-based architecture for multi-language code generation

pub mod python;

pub const VERSION: &str = "0.2.0";

use std::path::Path;

use crate::ast::IntentFile;
use crate::error::CompileResult;

/// Target language for code generation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetLanguage {
    Python,
    // Future: Go, TypeScript, etc.
}

impl std::str::FromStr for TargetLanguage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Ok(TargetLanguage::Python),
            _ => Err(format!("Unknown target language: {}. Supported: python", s)),
        }
    }
}

impl std::fmt::Display for TargetLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetLanguage::Python => write!(f, "python"),
        }
    }
}

/// Code generator trait - implement for each target language
#[allow(dead_code)]
pub trait CodeGenerator {
    /// Generate code from an intent file
    fn generate(&self, ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult>;

    /// Get the target language
    fn language(&self) -> TargetLanguage;

    /// Get the file extension for generated files
    fn file_extension(&self) -> &str;
}

/// Result of code generation
#[derive(Debug, Default)]
pub struct GenerationResult {
    /// Files that were generated
    pub files_created: Vec<String>,
    /// Total lines of code generated
    pub lines_generated: usize,
    /// Any warnings during generation
    pub warnings: Vec<String>,
}

impl GenerationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: impl Into<String>, lines: usize) {
        self.files_created.push(path.into());
        self.lines_generated += lines;
    }


    pub fn merge(&mut self, other: GenerationResult) {
        self.files_created.extend(other.files_created);
        self.lines_generated += other.lines_generated;
        self.warnings.extend(other.warnings);
    }
}

/// Create a code generator for the given target language
pub fn create_generator(target: TargetLanguage) -> Box<dyn CodeGenerator> {
    match target {
        TargetLanguage::Python => Box::new(python::PythonGenerator::new()),
    }
}
