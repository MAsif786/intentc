# Intent Compiler - Project Context

## Project Overview
`intentc` is a Rust-based compiler that transforms a custom **Intent Definition Language (IDL)** into a fully functional **Python backend** using FastAPI, SQLAlchemy, and Pydantic.

## Current Version: v0.3.1

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

## Current Features (v0.3.1)
- [x] **Parser**: Entities, Actions (structured syntax), Rules, Policies.
- [x] **Validator**: Type checking, decorator validation, policy verification.
- [x] **Auth Entity**: Dedicated `auth entity Name:` syntax for authentication.
- [x] **Process Engine**: `derive` syntax with `select`, `compute`, `system` commands.
- [x] **Layered Architecture**: Repository/Service/Controller pattern (singletons).
- [x] **JWT Authentication**: `@auth` decorator with protected routes.
- [x] **Password Hashing**: `@map(field, hash)` for bcrypt transforms.
- [x] **Policies**: Authorization rules with `@policy(Name)` decorator.
- [x] **Field Types**: `uuid`, `email`, enums, optionals, arrays, references.

## Recent Milestones
- **v0.3.1**: Added `auth entity` syntax with validation (single auth entity, @auth requirements).
- **v0.3**: Structured actions with `input`/`process`/`output`, policies, layered architecture.
- **v0.2**: JWT authentication, password hashing, `@auth` decorator.

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
# Auth entity (v0.3.1)
auth entity User:
    id: uuid @primary @default(uuid)
    email: email @unique
    password_hash: string

# Policy with @auth subject
policy AdminOnly:
    subject: @auth
    require subject.role == "admin"

# Structured action
@api POST /login
action login:
    input:
        email: email
        password: string
    process:
        derive user = select User where email == input.email
        derive token = system jwt.create(user.email)
    output: User(id, token)
```

## Next Steps / Roadmap
1. **OpenAPI Export**: Generate OpenAPI/Swagger JSON from intent files.
2. **TypeScript Target**: Add Node.js/TypeScript code generation.
3. **Go Target**: Add Go/Gin code generation.
4. **VS Code Extension**: Syntax highlighting and LSP for `.intent` files.
