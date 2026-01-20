# Intent Compiler Makefile
# Compile, run, and test intent files

# Configuration
INTENT_FILE ?= examples/app.intent
OUTPUT_DIR ?= output
PYTHON ?= python3
VENV ?= $(OUTPUT_DIR)/.venv

.PHONY: all build compile run test clean setup help

# Default target
all: compile

# Build the Rust compiler
build:
	@echo "📦 Building intentc compiler..."
	cargo build --release

# Compile intent file to Python
compile: build
	@echo "🔧 Compiling $(INTENT_FILE)..."
	@mkdir -p $(OUTPUT_DIR)
	cargo run --release -- compile --input $(INTENT_FILE) --output $(OUTPUT_DIR)
	@echo "✅ Output generated in $(OUTPUT_DIR)"

# Setup Python virtual environment and install dependencies
setup: compile
	@echo "🐍 Setting up Python environment..."
	@if [ ! -d "$(VENV)" ]; then \
		$(PYTHON) -m venv $(VENV); \
	fi
	@$(VENV)/bin/pip install --quiet --upgrade pip
	@$(VENV)/bin/pip install --quiet -r $(OUTPUT_DIR)/requirements.txt
	@echo "📦 Running database migrations..."
	@cd $(OUTPUT_DIR) && ../$(VENV)/bin/alembic upgrade head 2>/dev/null || true
	@echo "✅ Setup complete"

# Run the generated FastAPI server
run: setup
	@echo "🚀 Starting server at http://localhost:18000..."
	@cd $(OUTPUT_DIR) && ../$(VENV)/bin/uvicorn main:app --reload --host 0.0.0.0 --port 18000

# Run compiler tests (Rust)
test:
	@echo "🧪 Running compiler tests..."
	cargo test

# Run generated Python tests
test-python: setup
	@echo "🧪 Running Python tests..."
	@cd $(OUTPUT_DIR) && ../$(VENV)/bin/pytest tests/ -v

# Run all tests
test-all: test test-python

# Clean generated files
clean:
	@echo "🧹 Cleaning..."
	@rm -rf $(OUTPUT_DIR)
	@cargo clean
	@echo "✅ Clean complete"

# Clean only output (keep Rust build)
clean-output:
	@echo "🧹 Cleaning output..."
	@rm -rf $(OUTPUT_DIR)

# Development: compile and run in one command
dev: compile setup run

# Watch mode: recompile on intent file changes (requires entr)
watch:
	@echo "👀 Watching $(INTENT_FILE) for changes..."
	@echo $(INTENT_FILE) | entr -r make compile

# Help
help:
	@echo "Intent Compiler Makefile"
	@echo ""
	@echo "Usage:"
	@echo "  make [target] [INTENT_FILE=path/to/file.intent] [OUTPUT_DIR=output]"
	@echo ""
	@echo "Targets:"
	@echo "  build        Build the Rust compiler"
	@echo "  compile      Compile intent file to Python (default)"
	@echo "  setup        Setup Python venv and install dependencies"
	@echo "  run          Start the FastAPI server"
	@echo "  dev          Compile, setup, and run in one command"
	@echo "  test         Run Rust compiler tests"
	@echo "  test-python  Run generated Python tests"
	@echo "  test-all     Run all tests"
	@echo "  clean        Remove all generated files"
	@echo "  clean-output Remove only Python output"
	@echo "  watch        Watch intent file for changes"
	@echo "  help         Show this help"
	@echo ""
	@echo "Examples:"
	@echo "  make compile INTENT_FILE=examples/app.intent"
	@echo "  make run OUTPUT_DIR=my_app"
	@echo "  make dev INTENT_FILE=my.intent OUTPUT_DIR=server"
