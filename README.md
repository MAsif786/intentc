# Intent Compiler (intentc)

> Transform high-level intent definitions into production-ready Python backend code.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

## What is Intent Compiler?

Intent Compiler is a **code generation tool** that transforms a simple, human-readable Intent Definition Language (IDL) into a complete Python backend application. Write 25 lines of intent, get 500+ lines of production-ready code.

### The Problem

Building backends involves repetitive boilerplate:
- Pydantic models for validation
- SQLAlchemy models for database
- FastAPI routes for API endpoints
- Business rule validation
- Database migrations
- Test scaffolding

### The Solution

Define your **intent** once, let the compiler generate everything:

```intent
entity User:
    id: string @primary
    name: string
    email: string @unique
    age: number
    status: active | inactive

action create_user:
    name: string
    email: string
    age: number
    @api POST /users
    @returns User

rule ValidateAge:
    when User.age < 18
    then reject("User must be 18 or older")
```

**Output:** A complete FastAPI application with Pydantic models, SQLAlchemy ORM, API routes, business rules, migrations, and tests.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/MAsif786/intentc.git
cd intentc

# Build with Cargo
cargo build --release

# The binary is at ./target/release/intentc
```

### Requirements

- Rust 1.70 or higher
- Cargo (comes with Rust)

## Quick Start

### 1. Create an Intent File

Create `app.intent`:

```intent
entity Product:
    id: string @primary
    name: string
    price: number
    category: electronics | clothing | food
    in_stock: boolean

action create_product:
    name: string
    price: number
    category: string
    @api POST /products
    @returns Product

action get_product:
    id: string
    @api GET /products/{id}
    @returns Product
```

### 2. Compile

```bash
intentc compile -i app.intent -o my-api
```

### 3. Run the Generated API

```bash
cd my-api
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install -r requirements.txt
python main.py
```

Your API is now running at `http://localhost:8000`!

## CLI Commands

### `compile` - Generate Code

```bash
intentc compile -i <input.intent> -o <output-dir> [options]

Options:
  -i, --input     Input .intent file path (required)
  -o, --output    Output directory (default: ./output)
  -t, --target    Target language (default: python)
  -v, --verbose   Enable verbose output
```

### `check` - Validate Syntax

```bash
intentc check -i <input.intent>

# Validates the intent file without generating code
```

### `init` - Create New Project

```bash
intentc init my-project

# Creates a new project with:
# - src/app.intent (example)
# - README.md
# - .gitignore
```

## Intent Definition Language (IDL)

### Entities

Define your data models:

```intent
entity User:
    id: string @primary
    name: string
    email: string @unique
    age: number
    is_active: boolean
    created_at: datetime @default(now)
```

#### Field Types

| Type | Description | Python Type |
|------|-------------|-------------|
| `string` | Text data | `str` |
| `number` | Numeric data | `float` |
| `boolean` | True/false | `bool` |
| `datetime` | Date and time | `datetime` |
| `a \| b \| c` | Enum values | `Literal["a", "b", "c"]` |
| `EntityName` | Reference | Foreign key |
| `[type]` | Array | `List[type]` |
| `type?` | Optional | `Optional[type]` |

#### Field Decorators

| Decorator | Description |
|-----------|-------------|
| `@primary` | Primary key field |
| `@unique` | Unique constraint |
| `@optional` | Nullable field |
| `@index` | Database index |
| `@default(value)` | Default value |

### Actions

Define API endpoints:

```intent
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
```

#### Action Decorators

| Decorator | Description |
|-----------|-------------|
| `@api METHOD /path` | HTTP endpoint (GET, POST, PUT, PATCH, DELETE) |
| `@returns EntityName` | Response type |
| `@auth` | Requires authentication |

### Rules

Define business logic:

```intent
rule ValidateAge:
    when User.age < 18
    then reject("User must be 18 or older")

rule LogNewUser:
    when User.status == active
    then log("New active user created")

rule NotifyAdmin:
    when Order.total > 1000
    then send_notification(admin_email)
```

#### Expressions

- Comparisons: `==`, `!=`, `>`, `<`, `>=`, `<=`
- Logical: `and`, `or`, `not`
- Field access: `Entity.field`

#### Consequences

| Consequence | Description |
|-------------|-------------|
| `reject("message")` | Raise HTTP 400 error |
| `log("message")` | Log message |
| `action_name(args)` | Call another action |

## Generated Output

```
output/
├── main.py              # FastAPI entry point
├── requirements.txt     # Python dependencies
├── .env.example         # Environment template
├── api/
│   ├── __init__.py
│   └── routes.py        # API endpoints
├── db/
│   ├── __init__.py
│   ├── database.py      # SQLAlchemy setup
│   ├── models.py        # ORM models
│   └── migrations/      # Alembic migrations
├── models/
│   ├── __init__.py
│   └── <entity>.py      # Pydantic models
├── logic/
│   ├── __init__.py
│   └── rules.py         # Business rules
└── tests/
    ├── __init__.py
    ├── conftest.py      # pytest fixtures
    ├── test_models.py   # Model tests
    └── test_api.py      # API tests
```

## Generated Technology Stack

| Component | Technology |
|-----------|------------|
| Web Framework | FastAPI |
| Validation | Pydantic v2 |
| ORM | SQLAlchemy 2.0 |
| Migrations | Alembic |
| Testing | pytest + pytest-asyncio |
| HTTP Client | httpx |

## Examples

### E-commerce API

```intent
entity Product:
    id: string @primary
    name: string
    description: string?
    price: number
    category: electronics | clothing | books
    in_stock: boolean

entity Order:
    id: string @primary
    user_id: string
    total: number
    status: pending | processing | shipped | delivered
    created_at: datetime @default(now)

action create_order:
    user_id: string
    product_ids: [string]
    @api POST /orders
    @returns Order

rule MinimumOrder:
    when Order.total < 10
    then reject("Minimum order is $10")
```

### Blog API

```intent
entity Post:
    id: string @primary
    title: string
    content: string
    author_id: string
    status: draft | published | archived
    created_at: datetime @default(now)

entity Comment:
    id: string @primary
    post_id: string
    author: string
    content: string
    created_at: datetime @default(now)

action publish_post:
    id: string
    @api PATCH /posts/{id}/publish
    @returns Post

rule ContentRequired:
    when Post.content == ""
    then reject("Post content cannot be empty")
```

## Development

### Building

```bash
cargo build           # Debug build
cargo build --release # Release build
cargo test            # Run tests
```

### Project Structure

```
src/
├── main.rs          # Entry point
├── cli.rs           # CLI with clap
├── ast.rs           # AST definitions
├── grammar.pest     # PEG grammar
├── parser.rs        # Parser implementation
├── validator.rs     # Semantic validation
├── error.rs         # Error types
└── codegen/
    ├── mod.rs       # CodeGenerator trait
    └── python/      # Python generators
        ├── mod.rs
        ├── models.rs    # Pydantic
        ├── orm.rs       # SQLAlchemy
        ├── api.rs       # FastAPI
        ├── rules.rs     # Business logic
        ├── migrations.rs
        └── tests.rs
```

## Roadmap

- [ ] TypeScript/Node.js target
- [ ] Go target
- [ ] GraphQL support
- [ ] Authentication/authorization
- [ ] OpenAPI export
- [ ] VS Code extension
- [ ] Language server (LSP)

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) first.

## License

Apache-2.0 - see [LICENSE](LICENSE) for details.

---

**Intent Compiler** - Focus on what your app should do, not how to build it.
