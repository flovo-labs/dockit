# Installation

Flovo Dockit distributes as a single, static `x86_64-unknown-linux-musl` binary that runs on any x86_64 Linux system (glibc or musl) with zero runtime dependencies.

---

## 1. Automated Install Script (Recommended)

To install the latest stable release into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/flovo-labs/dockit/main/install.sh | sh
```

### Pinning a Version or Directory

You can customize the target directory or pin a specific version using environment variables or command-line flags:

```bash
# Pin version v1.1.0 into /usr/local/bin
curl -fsSL https://raw.githubusercontent.com/flovo-labs/dockit/main/install.sh \
  | DOCKIT_VERSION=v1.1.0 DOCKIT_INSTALL_DIR=/usr/local/bin sh
```

---

## 2. Manual Download & Checksum Verification

To download and manually verify the SHA-256 integrity:

```bash
VERSION="v1.1.0"
TARGET="x86_64-unknown-linux-musl"
BASE="https://github.com/flovo-labs/dockit/releases/download/${VERSION}"

# Download binary and sha256 checksum
curl -fsSLO "${BASE}/dockit-${VERSION}-${TARGET}"
curl -fsSLO "${BASE}/dockit-${VERSION}-${TARGET}.sha256"

# Verify SHA-256
sha256sum -c "dockit-${VERSION}-${TARGET}.sha256"

# Install into PATH
install -m 0755 "dockit-${VERSION}-${TARGET}" ~/.local/bin/dockit
```

---

## 3. Building From Source

If you are running on macOS, ARM64/aarch64 Linux, or want the bleeding-edge main branch:

```bash
# Clone the repository
git clone https://github.com/flovo-labs/dockit.git
cd dockit

# Compile optimized release binary
cargo build --release

# Copy to ~/.local/bin
install -m 0755 target/release/dockit ~/.local/bin/dockit
```

---

## 4. Shell Completions

Dockit includes dynamic, repository-aware shell completions that complete subcommands, options, and discovered project names from your repository.

### Bash
Add the following to your `~/.bashrc`:

```bash
source <(dockit completions bash)
```

### Zsh
Write the completion file to a directory in your `$fpath` (before calling `compinit`):

```zsh
mkdir -p ~/.zsh/completions
dockit completions zsh > ~/.zsh/completions/_dockit
```

### Fish
Save the completion script to fish's completions directory:

```fish
mkdir -p ~/.config/fish/completions
dockit completions fish > ~/.config/fish/completions/dockit.fish
```

---

## 5. Verification

Verify that Dockit is installed and accessible in your shell:

```bash
dockit --version
# Output: dockit 1.1.0

dockit -h
# Output: Your containers, under control.
```
