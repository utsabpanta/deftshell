#!/usr/bin/env bash
set -euo pipefail

# DeftShell Release Script
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.2.0

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

# Validate version format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in semver format (e.g., 0.2.0)"
    exit 1
fi

echo "=== DeftShell Release v${VERSION} ==="
echo

# Check for clean working directory
if [[ -n "$(git status --porcelain)" ]]; then
    echo "Error: Working directory is not clean. Commit or stash changes first."
    exit 1
fi

# Check we're on main branch
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$BRANCH" != "main" ]]; then
    echo "Warning: Not on main branch (currently on '$BRANCH')"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    [[ $REPLY =~ ^[Yy]$ ]] || exit 1
fi

echo "Step 1: Updating version numbers..."

# Update Cargo.toml versions
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" crates/ds-cli/Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" crates/ds-core/Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" crates/ds-plugin-sdk/Cargo.toml

# Update Homebrew formula version
sed -i '' "s/version \".*\"/version \"${VERSION}\"/" homebrew/deftshell.rb

echo "Step 2: Running tests..."
cargo test --workspace
echo "All tests passed."

echo "Step 3: Building release binaries..."
cargo build --release
echo "Release binary built."

echo "Step 4: Generating man page..."
if command -v help2man &>/dev/null; then
    help2man ./target/release/ds > docs/man/ds.1 2>/dev/null || true
fi

echo "Step 5: Creating git commit and tag..."
git add -A
git commit -m "release: v${VERSION}"
git tag -a "v${VERSION}" -m "DeftShell v${VERSION}"

echo
echo "=== Release v${VERSION} prepared ==="
echo
echo "Next steps:"
echo "  1. Review changes:  git log --oneline -5"
echo "  2. Push to remote:  git push origin main --tags"
echo "  3. GitHub Actions will automatically create the release"
echo "  4. Update Homebrew:  Update SHA256 hashes in homebrew/deftshell.rb"
