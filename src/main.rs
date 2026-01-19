// Intent Compiler - Main Entry Point
// Orchestrates parsing, validation, and code generation

mod ast;
mod cli;
mod codegen;
mod error;
mod parser;
mod validator;

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use colored::Colorize;

use cli::{Cli, Commands};
use codegen::{create_generator, TargetLanguage};
use error::CompileResult;

fn main() -> ExitCode {
    let cli = Cli::parse_args();

    let result = match cli.command {
        Commands::Compile { input, output, target, tests: _ } => {
            compile_intent(&input, &output, &target, cli.verbose)
        }
        Commands::Check { input } => {
            check_intent(&input, cli.verbose)
        }
        Commands::Init { name, example } => {
            init_project(&name, example, cli.verbose)
        }
    };

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e.format_colored(None));
            ExitCode::FAILURE
        }
    }
}

/// Compile an intent file to target language
fn compile_intent(input: &Path, output: &Path, target: &str, verbose: bool) -> CompileResult<()> {
    if verbose {
        println!("{} {} → {}", "Compiling".green().bold(), input.display(), output.display());
    }

    // Read source file
    let source = fs::read_to_string(input)?;

    // Parse
    if verbose {
        println!("  {} Parsing...", "→".blue());
    }
    let ast = parser::parse_intent(&source)?;

    if verbose {
        println!("    {} {} entities, {} actions, {} rules", 
            "✓".green(),
            ast.entities.len(),
            ast.actions.len(),
            ast.rules.len()
        );
    }

    // Validate
    if verbose {
        println!("  {} Validating...", "→".blue());
    }
    let ctx = validator::validate(&ast)?;

    // Print warnings
    for warning in &ctx.warnings {
        eprintln!("{}", warning.format_colored());
    }

    if verbose {
        println!("    {} Validation passed", "✓".green());
    }

    // Parse target language
    let target_lang: TargetLanguage = target.parse()
        .map_err(|e: String| error::CompileError::codegen(e))?;

    // Generate code
    if verbose {
        println!("  {} Generating {} code...", "→".blue(), target_lang);
    }

    // Create output directory
    fs::create_dir_all(output)?;

    let generator = create_generator(target_lang);
    let result = generator.generate(&ast, output)?;

    if verbose {
        println!("    {} Generated {} files ({} lines)", 
            "✓".green(),
            result.files_created.len(),
            result.lines_generated
        );
        
        for file in &result.files_created {
            println!("      {} {}", "→".blue(), file);
        }
    }

    // Print warnings from generation
    for warning in &result.warnings {
        eprintln!("{}: {}", "warning".yellow().bold(), warning);
    }

    println!("{} Compilation complete!", "✓".green().bold());
    println!("  Output: {}", output.display());

    Ok(())
}

/// Check an intent file without generating code
fn check_intent(input: &Path, verbose: bool) -> CompileResult<()> {
    if verbose {
        println!("{} {}", "Checking".green().bold(), input.display());
    }

    // Read source file
    let source = fs::read_to_string(input)?;

    // Parse
    let ast = parser::parse_intent(&source)?;

    if verbose {
        println!("  {} Parsed {} entities, {} actions, {} rules", 
            "✓".green(),
            ast.entities.len(),
            ast.actions.len(),
            ast.rules.len()
        );
    }

    // Validate
    let ctx = validator::validate(&ast)?;

    // Print warnings
    for warning in &ctx.warnings {
        eprintln!("{}", warning.format_colored());
    }

    println!("{} No errors found!", "✓".green().bold());

    Ok(())
}

/// Initialize a new intent project
fn init_project(name: &str, include_example: bool, verbose: bool) -> CompileResult<()> {
    if verbose {
        println!("{} new project: {}", "Initializing".green().bold(), name);
    }

    // Create project directory
    let project_dir = Path::new(name);
    fs::create_dir_all(project_dir)?;

    // Create src directory
    fs::create_dir_all(project_dir.join("src"))?;

    // Create README
    let readme = format!(r#"# {}

An Intent Compiler project.

## Getting Started

1. Edit `src/app.intent` to define your application
2. Run `intentc compile -i src/app.intent -o output`
3. Start the generated API: `cd output && python main.py`

## Learn More

Visit the Intent Compiler documentation for more information.
"#, name);
    fs::write(project_dir.join("README.md"), readme)?;

    // Create example intent file
    if include_example {
        let example = r#"# Example Intent File
# Define your entities, actions, and rules here

entity User:
    id: string @primary
    name: string
    email: string @unique
    age: number
    status: active | inactive
    created_at: datetime @default(now)

entity Post:
    id: string @primary
    title: string
    content: string
    author_id: string
    created_at: datetime @default(now)

action create_user:
    name: string
    email: string
    age: number
    @api POST /users
    @returns User

action get_user:
    id: string
    @api GET /users/{id}
    @returns User

action update_user:
    id: string
    name: string
    email: string
    @api PUT /users/{id}
    @returns User

action delete_user:
    id: string
    @api DELETE /users/{id}

rule ValidateAge:
    when User.age < 18
    then reject("User must be 18 or older")

rule LogNewUser:
    when User.status == active
    then log("New active user created")
"#;
        fs::write(project_dir.join("src/app.intent"), example)?;
    }

    // Create .gitignore
    let gitignore = r#"# Generated code
output/

# Python
__pycache__/
*.py[cod]
*.egg-info/
.eggs/
*.egg
.venv/
venv/

# Database
*.db
*.sqlite

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
"#;
    fs::write(project_dir.join(".gitignore"), gitignore)?;

    println!("{} Project initialized!", "✓".green().bold());
    println!("  Created: {}/", name);
    println!("  ");
    println!("  Next steps:");
    println!("    cd {}", name);
    if include_example {
        println!("    intentc compile -i src/app.intent -o output");
    } else {
        println!("    # Create your .intent files in src/");
        println!("    intentc compile -i src/app.intent -o output");
    }

    Ok(())
}
