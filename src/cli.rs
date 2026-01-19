// Intent Compiler - CLI Interface
// Command line interface using clap

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::codegen::TargetLanguage;

/// Intent Compiler - Transform IDL into production-ready backend code
#[derive(Parser, Debug)]
#[command(name = "intentc")]
#[command(author = "Muhammad Asif")]
#[command(version)]
#[command(about = "Intent Compiler - Transform IDL into production-ready backend code", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compile an intent file to target language
    Compile {
        /// Input .intent file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for generated code
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,

        /// Target language (python)
        #[arg(short, long, default_value = "python")]
        target: String,

        /// Generate tests
        #[arg(long, default_value = "true")]
        tests: bool,
    },

    /// Validate an intent file without generating code
    Check {
        /// Input .intent file path
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Initialize a new intent project
    Init {
        /// Project name / directory
        name: String,

        /// Include example intent file
        #[arg(long, default_value = "true")]
        example: bool,
    },
}

impl Cli {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}

/// Get target language from string
pub fn parse_target_language(target: &str) -> Result<TargetLanguage, String> {
    target.parse()
}
