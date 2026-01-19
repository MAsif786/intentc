# Intent Compiler — First Draft (AI-Friendly v0)

## 1. What it is

**Intent Compiler** is a system that turns **human or AI intent** into **production-ready backend code**.

Instead of writing Python, developers or AI agents describe *what they want* using a simple, structured **Intent Definition Language (IDL)**.  
The compiler then **validates**, **understands**, and **generates** native Python code automatically.

AI helps **before compilation**.  
The compiler handles **everything after**.

---

## 2. Why this exists

AI agents are good at:
- Understanding intent
- Reading natural language
- Explaining systems

But they struggle with:
- Large codebases
- Refactoring
- Keeping changes consistent
- Token cost and context limits

Intent Compiler fixes this by:
- Making intent explicit
- Keeping code generation deterministic
- Reducing how much AI needs to read or write
- Letting small or offline models work reliably

---

## 3. Key idea

> **Intent is the source of truth.  
> Code is a generated artifact.**

AI writes or helps write **intent**.  
The compiler turns intent into **real code**.

---

## 4. High-level flow

```
Human or AI writes intent
↓
Intent (IDL)
↓
Compiler
↓
AST (internal structure)
↓
Code Generator
↓
Python backend (API, DB, logic)
```


AI is optional but **fits naturally at the top**.

---

## 5. Intent Definition Language (IDL)

IDL is:
- Small
- Declarative
- Easy for humans
- Easy for AI models
- Strict enough for compilation

It describes:
- Data models
- Business rules
- Actions
- APIs
- Dependencies

### Example

```
entity User:
id: string
age: number
status: active | inactive

action enable_premium:
user_id: string
@api POST /premium/enable

rule EnablePremium:
when User.age > 18 and User.status == active
then enable_premium(User.id)
```


No glue code.  
No framework boilerplate.  
Just intent.

---

## 6. Compiler responsibilities

### 6.1 Parse
- Read IDL
- Build an internal structure (AST)

### 6.2 Validate
- Types exist
- Rules make sense
- Actions match signatures
- APIs are well defined

Errors happen **early and clearly**.

### 6.3 Generate code (Python v0)

The compiler generates:
- Database models
- Migrations
- API endpoints
- Business logic
- Dependency config
- Optional tests

Generated code:
- Is readable
- Runs at native speed
- Does not depend on the compiler at runtime

---

## 7. Role of AI

AI is a **first-class user**, not a replacement for the compiler.

AI can:
- Convert free-form text → IDL
- Suggest intent changes
- Explain compiler errors
- Help migrate existing systems

AI does **not**:
- Modify generated code
- Run in production
- Decide runtime behavior

This makes the system:
- Safer
- Cheaper
- Easier to debug

---

## 8. Offline and online friendly

The system works:
- With cloud models
- With small local models
- With no AI at all

Because:
- IDL is small
- Context is limited
- Compilation is deterministic

This allows:
- Offline development
- CI usage
- Enterprise adoption

---

## 9. Extensibility

Developers can extend behavior using:
- Custom actions
- Declared Python packages
- Plugins
- Explicit escape hatches (`raw` blocks)

Advanced users are **not limited**, but defaults stay simple.

---

## 10. What this is not

- Not a runtime framework
- Not an AI agent
- Not a low-code UI tool
- Not replacing Python

Python is a **target language**, not the interface.

---

## 11. Success criteria (v0)

- One intent file generates a working backend
- AI can reliably produce valid IDL
- Compilation succeeds deterministically
- Token usage is drastically lower than code generation
- Generated code matches handwritten performance

---

## 12. One-sentence summary

> **Intent Compiler lets humans and AI describe systems in intent, and reliably compiles that intent into real backend code.**
