# Contributing to Vetty

Thank you for your interest in contributing to Vetty! This guide will help you get started.

## Development Setup

### Prerequisites

- Linux (x86_64) with KVM support
- Rust 1.75+ with `x86_64-unknown-linux-musl` target
- Node.js 18+ with npm
- `e2fsprogs`, `curl`, `sudo`
- Firecracker binary on PATH

### Getting Started

```bash
# Clone your fork
git clone https://github.com/<your-username>/vetty.git
cd vetty

# Build everything
make build

# Run tests
make test

# Run linters
make lint
```

## Code Style

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- All code must pass `cargo clippy` with no warnings
- Write doc comments for public APIs
- Use `anyhow::Result` for error handling in binaries
- Use `thiserror` for library error types when appropriate

### TypeScript (GUI)

- Follow the ESLint configuration in `gui/eslint.config.js`
- Use TypeScript strict mode
- Prefer functional components with hooks

### Shell Scripts

- Use `#!/usr/bin/env bash` shebang
- Always set `set -euo pipefail`
- Quote all variables

## Making Changes

### Branch Naming

- `feat/description` — New features
- `fix/description` — Bug fixes
- `docs/description` — Documentation changes
- `refactor/description` — Code refactoring
- `ci/description` — CI/CD changes

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `ci`, `chore`

**Scopes:** `agent`, `daemon`, `cli`, `vm`, `disk`, `common`, `gui`, `image`, `ci`

**Examples:**
```
feat(agent): add file event deduplication
fix(daemon): handle vsock reconnection gracefully
docs(readme): add troubleshooting section
ci: add cargo clippy to CI pipeline
```

## Pull Request Process

1. **Fork** the repository and create your branch from `main`
2. **Make** your changes with clear, atomic commits
3. **Test** your changes locally (`make test && make lint`)
4. **Update** documentation if you changed behavior
5. **Open** a pull request with a clear description of what and why

### PR Checklist

- [ ] Code compiles without warnings (`cargo build`)
- [ ] All tests pass (`cargo test`)
- [ ] Linters pass (`cargo clippy`, `cd gui && npm run lint`)
- [ ] New public APIs have doc comments
- [ ] Breaking changes are documented

## Project Structure

See the [Architecture](docs/00-overview.md) document for a detailed overview of how the crates fit together.

| Crate | What it does |
|---|---|
| `vetty-common` | Shared event types and protocol definitions |
| `vetty-disk` | Builds ext4 disk images from host directories |
| `vetty-agent` | Guest-side binary: parses strace, sends events over vsock |
| `vetty-vm` | Firecracker VM launcher and serial relay |
| `vetty-daemon` | Host daemon: vsock listener + REST API + WebSocket |
| `vetty-cli` | CLI binary that orchestrates disk → VM → network |

## Reporting Issues

- Use GitHub Issues with the appropriate template
- Include your OS, Rust version, and Firecracker version
- For bugs, include steps to reproduce and relevant logs

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
