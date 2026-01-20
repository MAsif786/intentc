// Intent Compiler - Alembic Migration Generator
// Generates Alembic migrations from entity definitions

use std::fs;
use std::path::Path;
use chrono::Utc;

use crate::ast::{Entity, FieldType, Decorator, IntentFile};
use crate::codegen::GenerationResult;
use crate::error::CompileResult;

/// Generate Alembic migrations
pub fn generate_migrations(ast: &IntentFile, output_dir: &Path) -> CompileResult<GenerationResult> {
    let mut result = GenerationResult::new();

    // Generate alembic.ini
    let alembic_ini = generate_alembic_ini();
    fs::write(output_dir.join("alembic.ini"), &alembic_ini)?;
    result.add_file("alembic.ini", alembic_ini.lines().count());

    // Generate env.py
    let env_py = generate_env_py();
    fs::write(output_dir.join("db/migrations/env.py"), &env_py)?;
    result.add_file("db/migrations/env.py", env_py.lines().count());

    // Generate script.py.mako
    let script_mako = generate_script_mako();
    fs::write(output_dir.join("db/migrations/script.py.mako"), &script_mako)?;
    result.add_file("db/migrations/script.py.mako", script_mako.lines().count());

    // Generate initial migration
    let (migration, lines) = generate_initial_migration(ast)?;
    let migration_filename = format!("001_initial.py");
    fs::write(output_dir.join("db/migrations/versions").join(&migration_filename), &migration)?;
    result.add_file(format!("db/migrations/versions/{}", migration_filename), lines);

    Ok(result)
}

/// Generate alembic.ini
fn generate_alembic_ini() -> String {
    r#"# Intent Compiler Generated Alembic Configuration
# A generic, single database configuration.

[alembic]
script_location = db/migrations
prepend_sys_path = .
version_path_separator = os

sqlalchemy.url = sqlite:///./app.db

[post_write_hooks]

[loggers]
keys = root,sqlalchemy,alembic

[handlers]
keys = console

[formatters]
keys = generic

[logger_root]
level = WARN
handlers = console
qualname =

[logger_sqlalchemy]
level = WARN
handlers =
qualname = sqlalchemy.engine

[logger_alembic]
level = INFO
handlers =
qualname = alembic

[handler_console]
class = StreamHandler
args = (sys.stderr,)
level = NOTSET
formatter = generic

[formatter_generic]
format = %(levelname)-5.5s [%(name)s] %(message)s
datefmt = %H:%M:%S
"#.to_string()
}

/// Generate env.py for Alembic
fn generate_env_py() -> String {
    r#"# Intent Compiler Generated Alembic Environment
from logging.config import fileConfig

from sqlalchemy import engine_from_config
from sqlalchemy import pool

from alembic import context

import sys
from pathlib import Path

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from db.models import Base

config = context.config

if config.config_file_name is not None:
    fileConfig(config.config_file_name)

target_metadata = Base.metadata


def run_migrations_offline() -> None:
    """Run migrations in 'offline' mode."""
    url = config.get_main_option("sqlalchemy.url")
    context.configure(
        url=url,
        target_metadata=target_metadata,
        literal_binds=True,
        dialect_opts={"paramstyle": "named"},
    )

    with context.begin_transaction():
        context.run_migrations()


def run_migrations_online() -> None:
    """Run migrations in 'online' mode."""
    connectable = engine_from_config(
        config.get_section(config.config_ini_section, {}),
        prefix="sqlalchemy.",
        poolclass=pool.NullPool,
    )

    with connectable.connect() as connection:
        context.configure(
            connection=connection, target_metadata=target_metadata
        )

        with context.begin_transaction():
            context.run_migrations()


if context.is_offline_mode():
    run_migrations_offline()
else:
    run_migrations_online()
"#.to_string()
}

/// Generate script.py.mako template
fn generate_script_mako() -> String {
    r#""""${message}

Revision ID: ${up_revision}
Revises: ${down_revision | comma,n}
Create Date: ${create_date}

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa
${imports if imports else ""}

# revision identifiers, used by Alembic.
revision: str = ${repr(up_revision)}
down_revision: Union[str, None] = ${repr(down_revision)}
branch_labels: Union[str, Sequence[str], None] = ${repr(branch_labels)}
depends_on: Union[str, Sequence[str], None] = ${repr(depends_on)}


def upgrade() -> None:
    ${upgrades if upgrades else "pass"}


def downgrade() -> None:
    ${downgrades if downgrades else "pass"}
"#.to_string()
}

/// Generate initial migration from entities
fn generate_initial_migration(ast: &IntentFile) -> CompileResult<(String, usize)> {
    let mut content = String::new();

    content.push_str("\"\"\"Initial migration - create all tables\n\n");
    content.push_str("Revision ID: 001_initial\n");
    content.push_str("Revises: \n");
    content.push_str(&format!("Create Date: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));
    content.push_str("\"\"\"\n");
    content.push_str("from typing import Sequence, Union\n\n");
    content.push_str("from alembic import op\n");
    content.push_str("import sqlalchemy as sa\n\n\n");
    
    content.push_str("# revision identifiers, used by Alembic.\n");
    content.push_str("revision: str = '001_initial'\n");
    content.push_str("down_revision: Union[str, None] = None\n");
    content.push_str("branch_labels: Union[str, Sequence[str], None] = None\n");
    content.push_str("depends_on: Union[str, Sequence[str], None] = None\n\n\n");

    // Generate upgrade function
    content.push_str("def upgrade() -> None:\n");
    content.push_str("    \"\"\"Create all tables\"\"\"\n");
    
    for entity in &ast.entities {
        content.push_str(&generate_create_table(entity)?);
    }
    
    content.push_str("\n\n");

    // Generate downgrade function
    content.push_str("def downgrade() -> None:\n");
    content.push_str("    \"\"\"Drop all tables\"\"\"\n");
    
    for entity in ast.entities.iter().rev() {
        let table_name = entity.name.to_lowercase() + "s";
        content.push_str(&format!("    op.drop_table('{}')\n", table_name));
    }

    let lines = content.lines().count();
    Ok((content, lines))
}

/// Generate create_table call for an entity
fn generate_create_table(entity: &Entity) -> CompileResult<String> {
    let table_name = entity.name.to_lowercase() + "s";
    let mut content = String::new();

    content.push_str(&format!("    op.create_table(\n"));
    content.push_str(&format!("        '{}',\n", table_name));

    for field in &entity.fields {
        content.push_str(&format!("        {},\n", generate_column_def(field)?));
    }

    content.push_str("    )\n");

    // Create indexes
    for field in &entity.fields {
        if field.decorators.contains(&Decorator::Index) {
            content.push_str(&format!(
                "    op.create_index('ix_{0}_{1}', '{0}', ['{1}'])\n",
                table_name, field.name
            ));
        }
    }

    Ok(content)
}

/// Generate a column definition for migration
fn generate_column_def(field: &crate::ast::Field) -> CompileResult<String> {
    let is_primary = field.decorators.contains(&Decorator::Primary);
    let is_nullable = field.decorators.contains(&Decorator::Optional);
    let is_unique = field.decorators.contains(&Decorator::Unique);

    let col_type = match &field.field_type {
        FieldType::String => "sa.String(255)".to_string(),
        FieldType::Number => "sa.Float()".to_string(),
        FieldType::Boolean => "sa.Boolean()".to_string(),
        FieldType::DateTime => "sa.DateTime()".to_string(),
        FieldType::Uuid => "sa.String(36)".to_string(),  // UUID as 36-char string
        FieldType::Email => "sa.String(255)".to_string(), // Email as string
        FieldType::Enum(values) => {
            let vals = values.iter().map(|v| format!("'{}'", v)).collect::<Vec<_>>().join(", ");
            format!("sa.Enum({})", vals)
        }
        FieldType::Reference(name) => {
            format!("sa.String(255), sa.ForeignKey('{}.id')", name.to_lowercase() + "s")
        }
        FieldType::Ref(name) => {
            format!("sa.String(255), sa.ForeignKey('{}.id')", name.to_lowercase() + "s")
        }
        FieldType::Array(_) => "sa.JSON()".to_string(),
        FieldType::List(_) => "sa.JSON()".to_string(),
        FieldType::Optional(inner) => match inner.as_ref() {
            FieldType::String => "sa.String(255)".to_string(),
            FieldType::Number => "sa.Float()".to_string(),
            FieldType::Boolean => "sa.Boolean()".to_string(),
            FieldType::DateTime => "sa.DateTime()".to_string(),
            FieldType::Uuid => "sa.String(36)".to_string(),
            FieldType::Email => "sa.String(255)".to_string(),
            _ => "sa.String(255)".to_string(),
        },
    };

    let mut options = Vec::new();
    if is_primary {
        options.push("primary_key=True");
    }
    if !is_nullable && !is_primary {
        options.push("nullable=False");
    }
    if is_unique {
        options.push("unique=True");
    }

    let options_str = if options.is_empty() {
        String::new()
    } else {
        format!(", {}", options.join(", "))
    };

    Ok(format!("sa.Column('{}', {}{})", field.name, col_type, options_str))
}
