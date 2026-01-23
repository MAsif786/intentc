# Intent Compiler (intentc)

> Transform high-level intent definitions into production-ready Python backend code.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Version](https://img.shields.io/badge/version-v0.3.1-green)](https://github.com/MAsif786/intentc/releases/tag/v0.3.1)

## What is Intent Compiler?

Intent Compiler is a **code generation tool** that transforms a simple, human-readable Intent Definition Language (IDL) into a complete Python backend application with clean layered architecture. Write 30 lines of intent, get 2000+ lines of production-ready code.

### The Problem

Building backends involves repetitive boilerplate:
- Pydantic models for validation
- SQLAlchemy models for database
- FastAPI routes for API endpoints
- Business rule validation
- Database migrations
- Security & Auth implementation
- Test scaffolding

### The Solution (v0.3)

Define your **intent** once, let the compiler generate everything:

```intent
# Designate User as the auth entity
auth entity User:
    id: uuid @primary @default(uuid)
    email: email @unique @index
    password_hash: string
    role: string @default("user")

@api POST /login
action login:
    input:
        email: email
        password: string
    process:
        derive user = select User where email == input.email
        derive valid = compute verify_hash(password, user.password_hash)
        derive token = system jwt.create(user.email)
    output: User(id, email, token)

policy AdminOnly:
    subject: @auth  # References the auth entity
    require subject.role == "admin"
```

**Output:** A complete FastAPI application with Pydantic models, SQLAlchemy ORM, API routes, business rules, policies, migrations, and tests.

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

- Rust 1.75 or higher
- Cargo (comes with Rust)
- Python 3.10+ (for running generated code)

## Quick Start

### 1. Create an Intent File

Create `app.intent`:

```intent
entity Product:
    id: uuid @primary @default(uuid)
    name: string @index
    price: number @validate(min: 0)
    stock: number @default(0)

@api POST /products
@auth
action create_product:
    input:
        name: string
        price: number
    output: Product(id, name)

@api GET /products/{id}
action get_product:
    input:
        id: uuid
    output: Product(id, name, price)
```

### 2. Compile

```bash
intentc compile -i app.intent -o my-api
```

### 3. Run the Generated API

The generated code includes a `Makefile` for convenience:

```bash
cd my-api
make setup  # Creates venv, installs deps, runs migrations
make run    # Starts the FastAPI server
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

# Validates the intent file without generating code (v0.3)
```

## Intent Definition Language (IDL)

### Entities

Define your data models with fields and constraints:

```intent
entity User:
    id: uuid @primary @default(uuid)
    email: email @unique @index
    full_name: string
    status: active | inactive @default("active")
    created_at: datetime @default(now)

    # Scoped policy
    policy CanUpdateProfile:
        subject: @auth
        require subject.id == User.id
```

#### Auth Entity (v0.3.1)

Designate a special entity for authentication:

```intent
auth entity User:
    id: uuid @primary @default(uuid)
    email: email @unique
    password_hash: string
```

- Only **one** `auth entity` per file is allowed
- `@auth` decorator without arguments uses the auth entity
- Policies can use `@auth` as subject to reference the auth entity

#### Field Types

| Type | Description | Python Type |
|------|-------------|-------------|
| `string` | Text data | `str` |
| `number` | Numeric data | `float` |
| `boolean` | True/false | `bool` |
| `datetime` | Date and time | `datetime` |
| `uuid` | UUID string | `UUID` |
| `email` | Email string | `EmailStr` |
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
| `@default(value)` | Default value (supports `now`, `uuid`) |
| `@validate(...)` | Constraints like `min: 0`, `max: 100` |

### Actions (v0.3 Structured Syntax)

Actions define API endpoints and business flows:

```intent
@api POST /login
action login:
    input:
        email: email
        password: string
    process:
        derive user = select User where email == input.email
        derive valid = compute verify_hash(password, user.password_hash)
        derive token = system jwt.create(user.email)
    output: User(id, email, token)
```

#### Process Block (v0.3)

| Command | Description | Example |
|---------|-------------|---------|
| `select` | Query database | `derive u = select User where email == e` |
| `compute`| Call business logic | `derive v = compute hash(pass)` |
| `system` | External capability | `derive t = system jwt.create(sub)` |

#### Action Decorators

| Decorator | Description |
|-----------|-------------|
| `@api METHOD /path` | HTTP endpoint mapping |
| `@auth` | Requires JWT authentication |
| `@auth(validate(id))` | Custom auth validation |
| `@policy(Name)` | Enforces a specific policy |
| `@map(field, hash)` | Transforms input field (e.g. password) |

### Policies (v0.3)

Declare authorization and access control rules:

```intent
policy AdminOnly:
    subject: @auth
    require subject.role == "admin"

@api DELETE /users/{id}
@auth
@policy(AdminOnly)
action delete_user:
    input:
        id: uuid
```

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
| `action_call(args)` | Call another action |

## Generated Output

```
output/
├── main.py              # FastAPI entry point
├── requirements.txt     # Python dependencies
├── .env.example         # Environment template
├── api/
│   └── routes.py        # API endpoints
├── controllers/         # Request handlers (Singleton Pattern)
├── services/            # Business logic (Singleton Pattern)
├── repositories/        # Data access (Singleton Pattern)
│   ├── base.py          # Generic CRUD
│   └── <entity>_repository.py
├── db/
│   ├── database.py      # SQLAlchemy setup
│   ├── models.py        # ORM models
│   └── migrations/      # Alembic migrations
├── models/
│   └── <entity>.py      # Pydantic models
├── core/
│   └── security.py      # JWT & password hashing
├── logic/
│   ├── rules.py         # Business rules
│   └── policies.py      # Access policies
└── tests/
    ├── conftest.py      # pytest fixtures
    ├── test_models.py   # Model tests
    └── test_api.py      # API tests
```

### Layered Architecture (v0.3)

| Layer | Responsibility |
|-------|----------------|
| **Controllers** | Handle HTTP requests, call services (Singleton) |
| **Services** | Business logic, validation, transformations (Singleton) |
| **Repositories** | Database CRUD operations (Singleton Pattern) |

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
    id: uuid @primary @default(uuid)
    name: string
    description: string?
    price: number
    category: electronics | clothing | books
    in_stock: boolean

entity Order:
    id: uuid @primary @default(uuid)
    user_id: uuid
    total: number
    status: pending | processing | shipped | delivered
    created_at: datetime @default(now)

@api POST /orders
@auth
action create_order:
    input:
        product_ids: [uuid]
    output: Order(id, status, total)

rule MinimumOrder:
    when Order.total < 10
    then reject("Minimum order is $10")
```

### Blog API

```intent
entity Post:
    id: uuid @primary @default(uuid)
    title: string
    content: string
    author_id: uuid
    status: draft | published | archived
    created_at: datetime @default(now)

entity Comment:
    id: uuid @primary @default(uuid)
    post_id: uuid
    author: string
    content: string
    created_at: datetime @default(now)

@api PATCH /posts/{id}/publish
@auth
action publish_post:
    input:
        id: uuid
    output: Post(id, title, status)

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

- [x] JWT Authentication (`@auth`)
- [x] Password hashing (`@map` with hash transform)
- [x] Layered architecture (Repository/Service/Controller)
- [x] Process engine with `derive` syntax (v0.3)
- [x] Authorization Policies (v0.3)
- [x] UUID and Email primitive types (v0.3)
- [x] Dedicated `auth entity` syntax (v0.3.1)
- [ ] TypeScript/Node.js target
- [ ] Go target
- [ ] OpenAPI export
- [ ] VS Code extension
- [ ] Language server (LSP)

## Contributing

Contributions are welcome! Please reach out to [EMAIL_ADDRESS]
## License

Apache-2.0 - see [LICENSE](LICENSE) for details.

---

**Intent Compiler** - Focus on what your app should do, not how to build it.
