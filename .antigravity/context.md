# Intent Compiler - Project Context

## Project Overview
`intentc` is a Rust-based compiler that transforms a custom **Intent Definition Language (IDL)** into a fully functional **Python backend** using FastAPI, SQLAlchemy, and Pydantic.

## Current Version: v0.4

## Technology Stack
- **Compiler**: Rust (using `pest` for PEG parsing, `clap` for CLI).
- **Target Stack**: 
  - Web: FastAPI
  - ORM: SQLAlchemy 2.0 (SQLite by default)
  - Validation: Pydantic v2 (Strict mode with `extra='forbid'`)
  - Migrations: Alembic
  - Testing: pytest + pytest-asyncio
  - Auth: PyJWT + passlib (bcrypt)

## Key Directories
- `src/`: Rust source code for the compiler.
  - `ast.rs`: AST definitions (entities, actions, rules, policies).
  - `grammar.pest`: PEG grammar rules.
  - `parser.rs`: Parsing logic.
  - `validator.rs`: Semantic validation.
  - `codegen/python/`: Modules for generating Python backend components.
- `examples/`: Example `.intent` files for testing.
- `output/`: The latest generated Python project.

## Current Features (v0.4)
- [x] **Parser**: Entities, Actions (structured syntax), Rules, Policies.
- [x] **Validator**: Type checking, decorator validation, policy verification.
- [x] **Auth Entity**: Dedicated `auth entity Name:` syntax for authentication.
- [x] **Process Engine**: `derive` syntax with `select`, `compute`, `system` commands.
- [x] **Atomic Ops**: `mutate` (Create/Update) and `delete` operations in process flow.
- [x] **Indented Output**: Support for multiline `output:` projections.
- [x] **Layered Architecture**: Repository/Service/Controller pattern (singletons).
- [x] **JWT Authentication**: `@auth` decorator with protected routes.
- [x] **Password Hashing**: `@map(field, hash)` for bcrypt transforms.

## Recent Milestones
- **v0.4**: Added `mutate` (create/update) and `delete` operations, indented output syntax.
- **v0.3.1**: Added `auth entity` syntax with validation (single auth entity, @auth requirements).
- **v0.3**: Structured actions with `input`/`process`/`output`, policies, layered architecture.

## Common Commands
```bash
# Compile intent file
cargo run -- compile --input examples/app.intent --output output

# Build compiler (release)
cargo build --release

# Run all tests
make test-all

# Run generated API
cd output && make setup && make run
```

## Grammar Highlights
```intent
# Explicit Mutate Create (v0.4)
action signup:
    input:
        email: email
        password: string @map(password_hash, hash)
    process:
        mutate User:
            set email = input.email
            set password_hash = input.password
    output:
        User(id, email)

# Explicit Mutate Update (v0.4)
action cancel_order:
    input:
        id: uuid
    process:
        mutate Order where id == input.id:
            set status = "cancelled"
    output: 
        Order(id, status)

# Delete operation (v0.4)
action delete_review:
    input:
        id: uuid
    process:
        delete Review where id == input.id
    output: Review(id)
```

## Next Steps / Roadmap
1. **OpenAPI Export**: Generate OpenAPI/Swagger JSON from intent files.
2. **TypeScript Target**: Add Node.js/TypeScript code generation.
3. **Go Target**: Add Go/Gin code generation.
4. **VS Code Extension**: Syntax highlighting and LSP for `.intent` files.
