# Intent Compiler — v0 Scope

## Overview

This document defines the scope for v0 of the Intent Compiler — a Rust-based compiler that transforms Intent Definition Language (IDL) into production-ready Python backend code.

---

## Goals for v0

1. **Working Compiler**: Compile a single `.intent` file into a runnable Python backend
2. **Core IDL Constructs**: Support `entity`, `action`, and `rule` definitions
3. **Python Output**: Generate FastAPI + SQLAlchemy + Pydantic code
4. **ORM Migrations**: Auto-generate Alembic-compatible migrations
5. **Test Generation**: Optional test scaffolding for generated code
6. **CLI Binary**: Single executable that works across platforms

---

## IDL Syntax (v0)

### Entities
```
entity User:
    id: string @primary
    name: string
    age: number
    email: string @unique
    status: active | inactive | suspended
    created_at: datetime @default(now)
```

**Supported Types**: `string`, `number`, `boolean`, `datetime`  
**Supported Decorators**: `@primary`, `@unique`, `@default(value)`, `@optional`  
**Enum Types**: `value1 | value2 | value3`

---

### Actions
```
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
```

**Supported HTTP Methods**: GET, POST, PUT, PATCH, DELETE
**Decorators**: `@api`, `@returns`, `@auth`

---

### Rules
```
rule ValidateAge:
    when User.age < 18
    then reject("User must be 18 or older")

rule EnablePremium:
    when User.age > 18 and User.status == active
    then enable_premium(User.id)
```

**Supported Operators**: `>`, `<`, `>=`, `<=`, `==`, `!=`, `and`, `or`, `not`  
**Built-in Actions**: `reject(message)`, `log(message)`, custom action calls

---

## Compiler Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Intent Compiler                       │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────┐   ┌──────────┐   ┌──────────────────────┐ │
│  │  Lexer   │ → │  Parser  │ → │   AST (Typed)        │ │
│  └──────────┘   └──────────┘   └──────────────────────┘ │
│                                          │               │
│                                          ▼               │
│                               ┌──────────────────────┐  │
│                               │  Semantic Analyzer   │  │
│                               │  (Type Check, Lint)  │  │
│                               └──────────────────────┘  │
│                                          │               │
│                                          ▼               │
│                               ┌──────────────────────┐  │
│                               │   Code Generator     │  │
│                               │   (Trait-based)      │  │
│                               └──────────────────────┘  │
│                                          │               │
│                    ┌─────────────────────┼───────────┐  │
│                    ▼                     ▼           ▼  │
│              ┌──────────┐         ┌──────────┐  ┌─────┐ │
│              │  Python  │         │   Go     │  │ ... │ │
│              │ Generator│         │ Generator│  │     │ │
│              └──────────┘         └──────────┘  └─────┘ │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## Python Output Structure (v0)

```
output/
├── models/
│   ├── __init__.py
│   ├── user.py          # Pydantic models
│   └── ...
├── db/
│   ├── __init__.py
│   ├── models.py        # SQLAlchemy ORM models
│   └── migrations/      # Alembic migrations
├── api/
│   ├── __init__.py
│   ├── routes.py        # FastAPI routes
│   └── ...
├── logic/
│   ├── __init__.py
│   └── rules.py         # Business rules
├── tests/
│   ├── test_models.py
│   ├── test_api.py
│   └── ...
├── main.py              # Entry point
├── requirements.txt
└── alembic.ini
```

---

## CLI Commands (v0)

```bash
# Compile IDL to Python
intentc compile app.intent --output ./output --target python

# Initialize new project
intentc init my_project

# Validate IDL without generating code
intentc check app.intent

# Show version
intentc --version
```

---

## Technology Stack

| Component | Technology |
|-----------|------------|
| Compiler Language | Rust |
| Parser | pest (PEG parser) |
| CLI | clap |
| Serialization | serde |
| Python Web Framework | FastAPI |
| Python ORM | SQLAlchemy |
| Python Validation | Pydantic |
| Migrations | Alembic |

---

## Out of Scope (v0)

- Multiple `.intent` file compilation
- Custom plugins/escape hatches
- Language targets other than Python (architecture supports it)
- IDE/LSP support
- Watch mode / hot reload
- AI integration features

---

## Success Criteria

| Criteria | Metric |
|----------|--------|
| Single file compilation | One `.intent` file → working Python API |
| Deterministic output | Same input → same output always |
| Clear error messages | Line numbers, suggestions |
| Performance | Compile in < 1 second for typical files |
| Binary size | < 10MB standalone executable |
