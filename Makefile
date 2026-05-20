# ==============================================================================
# Vetty — Build & Run Orchestration
# ==============================================================================

SHELL := /bin/bash
.DEFAULT_GOAL := help
.PHONY: help build build-host build-agent gui-install setup run run-daemon run-gui run-vm clean lint test check

# Colors
BOLD   := \033[1m
GREEN  := \033[32m
YELLOW := \033[33m
CYAN   := \033[36m
RESET  := \033[0m

# Paths
ROOTFS       := image/rootfs.ext4
KERNEL       := image/vmlinux
AGENT_BIN    := target/x86_64-unknown-linux-musl/release/vetty-agent
GUI_DIR      := gui
DIR          ?= ./sample-code
MEMORY       ?= 128
CPUS         ?= 1

# ==============================================================================
# Help
# ==============================================================================

help: ## Show this help message
	@echo ""
	@echo "  $(BOLD)🛡️  Vetty$(RESET) — Firecracker sandbox for untrusted code"
	@echo ""
	@echo "  $(BOLD)Usage:$(RESET)"
	@echo "    make $(GREEN)<target>$(RESET) [DIR=./your-code]"
	@echo ""
	@echo "  $(BOLD)Targets:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "    $(GREEN)%-18s$(RESET) %s\n", $$1, $$2}'
	@echo ""

# ==============================================================================
# Build
# ==============================================================================

build: build-host build-agent ## Build all Rust crates (host + guest agent)
	@echo -e "$(GREEN)✓ All crates built successfully$(RESET)"

build-host: ## Build host crates (debug)
	@echo -e "$(CYAN)→ Building host crates...$(RESET)"
	cargo build

build-agent: ## Cross-compile guest agent (musl, release)
	@echo -e "$(CYAN)→ Building vetty-agent (musl static binary)...$(RESET)"
	@rustup target list --installed | grep -q x86_64-unknown-linux-musl || \
		rustup target add x86_64-unknown-linux-musl
	cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent

gui-install: ## Install GUI npm dependencies
	@echo -e "$(CYAN)→ Installing GUI dependencies...$(RESET)"
	cd $(GUI_DIR) && npm install --no-audit --no-fund

# ==============================================================================
# VM Image Setup
# ==============================================================================

$(KERNEL):
	@echo -e "$(CYAN)→ Downloading kernel...$(RESET)"
	cd image && bash download-kernel.sh

$(AGENT_BIN): build-agent

$(ROOTFS): $(AGENT_BIN)
	@echo -e "$(CYAN)→ Building rootfs (requires sudo)...$(RESET)"
	cd image && sudo bash build-rootfs.sh

setup: $(KERNEL) $(ROOTFS) ## Download kernel + build rootfs with agent
	@echo -e "$(GREEN)✓ VM assets ready$(RESET)"

setup-quick: $(KERNEL) ## Download pre-built kernel + rootfs (no sudo, no agent)
	@echo -e "$(CYAN)→ Downloading pre-built rootfs...$(RESET)"
	cd image && bash download-rootfs.sh
	@echo -e "$(GREEN)✓ Quick setup complete$(RESET)"
	@echo -e "$(YELLOW)⚠ Pre-built rootfs does not include vetty-agent. Use 'make setup' for full functionality.$(RESET)"

# ==============================================================================
# Run — One Command Launch
# ==============================================================================

run: build gui-install ## Build and run everything (daemon + GUI + VM)
	@DIR="$(DIR)" MEMORY="$(MEMORY)" CPUS="$(CPUS)" \
		bash scripts/run-all.sh

run-daemon: build-host ## Start only the daemon
	@echo -e "$(CYAN)→ Starting vetty-daemon...$(RESET)"
	cargo run -p vetty-daemon

run-gui: gui-install ## Start only the GUI
	@echo -e "$(CYAN)→ Starting Electron GUI...$(RESET)"
	cd $(GUI_DIR) && npm run electron:dev

run-vm: build-host ## Launch only the sandbox VM
	@echo -e "$(CYAN)→ Launching sandbox VM with $(DIR)...$(RESET)"
	cargo run -p vetty-cli -- --dir $(DIR) --rootfs $(ROOTFS) --kernel $(KERNEL) --memory $(MEMORY) --cpus $(CPUS)

# ==============================================================================
# Quality
# ==============================================================================

lint: ## Run all linters (clippy + eslint)
	@echo -e "$(CYAN)→ Running cargo clippy...$(RESET)"
	cargo clippy --workspace -- -D warnings
	@echo -e "$(CYAN)→ Running eslint...$(RESET)"
	cd $(GUI_DIR) && npm run lint
	@echo -e "$(GREEN)✓ All linters passed$(RESET)"

test: ## Run all tests
	@echo -e "$(CYAN)→ Running cargo test...$(RESET)"
	cargo test --workspace
	@echo -e "$(GREEN)✓ All tests passed$(RESET)"

check: ## Run cargo check (fast compilation check)
	cargo check --workspace

fmt: ## Format all Rust code
	cargo fmt --all

fmt-check: ## Check Rust formatting without modifying files
	cargo fmt --all -- --check

# ==============================================================================
# Clean
# ==============================================================================

clean: ## Remove all build artifacts
	@echo -e "$(CYAN)→ Cleaning Rust build artifacts...$(RESET)"
	cargo clean
	@echo -e "$(CYAN)→ Cleaning GUI build artifacts...$(RESET)"
	rm -rf $(GUI_DIR)/dist $(GUI_DIR)/node_modules
	@echo -e "$(GREEN)✓ Clean complete$(RESET)"

clean-vm: ## Remove downloaded VM assets (kernel + rootfs)
	rm -f $(KERNEL) $(ROOTFS) image/*.tar.gz
	@echo -e "$(GREEN)✓ VM assets removed$(RESET)"

distclean: clean clean-vm ## Remove everything (build + VM assets)
	@echo -e "$(GREEN)✓ Full clean complete$(RESET)"
