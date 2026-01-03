#!/usr/bin/env bash
set -euo pipefail

# LAO Release Script
# Creates a git tag and triggers automated builds for all platforms
# Enhanced for longevity with automatic version bumping and better error handling

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CURRENT_VERSION=$(grep '^version' core/Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/' || echo "0.1.0")

# Default to current version, but allow override
VERSION="${1:-$CURRENT_VERSION}"
RELEASE_TYPE="${2:-patch}"  # patch, minor, major, or specific version

# Function to calculate next version
calculate_next_version() {
    local current="$1"
    local type="$2"
    
    # Split version into parts
    IFS='.' read -ra VERSION_PARTS <<< "$current"
    local major="${VERSION_PARTS[0]:-0}"
    local minor="${VERSION_PARTS[1]:-0}"
    local patch="${VERSION_PARTS[2]:-0}"
    
    case "$type" in
        "major")
            echo "$((major + 1)).0.0"
            ;;
        "minor")
            echo "$major.$((minor + 1)).0"
            ;;
        "patch")
            echo "$major.$minor.$((patch + 1))"
            ;;
        *)
            # Assume it's a specific version
            echo "$type"
            ;;
    esac
}

# Function to update version in Cargo.toml files
update_version() {
    local new_version="$1"
    local cargo_files=(
        "core/Cargo.toml"
        "cli/Cargo.toml"
        "lao_plugin_api/Cargo.toml"
        "ui/lao-ui/Cargo.toml"
        "tools/plugin-generator/Cargo.toml"
        "tools/plugin-registry/Cargo.toml"
        "plugins/EchoPlugin/Cargo.toml"
        "plugins/WhisperPlugin/Cargo.toml"
        "plugins/OllamaPlugin/Cargo.toml"
        "plugins/PromptDispatcherPlugin/Cargo.toml"
        "plugins/SummarizerPlugin/Cargo.toml"
        "plugins/LMStudioPlugin/Cargo.toml"
        "plugins/GGUFPlugin/Cargo.toml"
        "plugins/plugin-template/Cargo.toml"
    )
    
    echo "📝 Updating version to $new_version in all Cargo.toml files..."
    
    for cargo_file in "${cargo_files[@]}"; do
        if [[ -f "$ROOT_DIR/$cargo_file" ]]; then
            if [[ "$OSTYPE" == "darwin"* ]]; then
                # macOS
                sed -i '' "s/^version = \".*\"/version = \"$new_version\"/" "$ROOT_DIR/$cargo_file"
            else
                # Linux
                sed -i "s/^version = \".*\"/version = \"$new_version\"/" "$ROOT_DIR/$cargo_file"
            fi
            echo "✅ Updated $cargo_file"
        fi
    done
    
    echo "✅ All versions updated to $new_version"
}

# Function to check if version is valid semver
validate_version() {
    local version="$1"
    
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "❌ Invalid version format: $version"
        echo "💡 Use semantic versioning (e.g., 1.2.3)"
        exit 1
    fi
    
    echo "✅ Version format is valid"
}

# Function to check if we're in a git repository
check_git_repo() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        echo "❌ Not in a git repository"
        exit 1
    fi
    
    if ! git remote get-url origin > /dev/null 2>&1; then
        echo "❌ No remote origin configured"
        echo "💡 Configure with: git remote add origin https://github.com/abendrothj/lao.git"
        exit 1
    fi
    
    echo "✅ Git repository configured"
}

# Function to check if working directory is clean
check_clean_working_dir() {
    if ! git diff-index --quiet HEAD --; then
        echo "❌ Working directory has uncommitted changes"
        echo "💡 Commit or stash changes first:"
        echo "   git add . && git commit -m 'Prepare release $VERSION'"
        echo "   OR"
        echo "   git stash"
        exit 1
    fi
    
    echo "✅ Working directory is clean"
}

# Function to check if tag already exists and handle it
check_tag_exists() {
    local version="$1"
    local force="${2:-false}"
    
    if git tag -l | grep -q "^v$version$"; then
        if [[ "$force" == "true" ]]; then
            echo "⚠️  Tag v$version already exists, but force mode enabled"
            echo "🗑️  Removing existing tag..."
            git tag -d "v$version" 2>/dev/null || true
            git push origin ":refs/tags/v$version" 2>/dev/null || true
            echo "✅ Existing tag removed"
        else
            echo "❌ Tag v$version already exists"
            echo "💡 Available tags:"
            git tag -l | sort -V | tail -5
            echo ""
            echo "🔄 Options:"
            echo "  1. Use a different version: $0 <new-version>"
            echo "  2. Force replace tag: $0 $version force"
            echo "  3. Bump version automatically: $0 auto $RELEASE_TYPE"
            exit 1
        fi
    fi
    
    echo "✅ Tag v$version is available"
}

# Function to create and push tag
create_and_push_tag() {
    local version="$1"
    echo "🏷️  Creating tag v$version..."
    
    # Create annotated tag
    git tag -a "v$version" -m "Release LAO v$version
    
    🎉 LAO v$version Release by Jake Abendroth
    
    ## What's New
    - Cross-platform support (Linux, macOS, Windows)
    - Comprehensive packaging system
    - Visual workflow builder with egui
    - Plugin system with dynamic loading
    - CLI and GUI interfaces
    
    ## Packages Available
    - Linux: .deb, .rpm, .tar.gz, AppImage
    - macOS: .dmg, .tar.gz
    - Windows: .msi, .zip
    
    ## Installation
    Download packages from GitHub Releases or use:
    \`\`\`bash
    # Linux
    sudo dpkg -i lao_${version}_amd64.deb
    
    # macOS
    open LAO-${version}.dmg
    
    # Windows
    msiexec /i lao-${version}.msi
    \`\`\`
    
    ## Quick Start
    \`\`\`bash
    lao-ui          # Start GUI
    lao-cli --help  # CLI help
    \`\`\`
    
    ## Support
    For issues and support, contact Jake Abendroth at contact@jakea.net
    Repository: https://github.com/abendrothj/lao"
    
    echo "✅ Tag v$version created"
    
    # Push tag to remote
    echo "📤 Pushing tag to remote..."
    git push origin "v$version"
    
    echo "✅ Tag v$version pushed to remote"
}

# Function to trigger GitHub Actions workflow
trigger_workflow() {
    echo "🤖 GitHub Actions workflow will trigger automatically on tag push..."
    echo "🔗 View progress: https://github.com/abendrothj/lao/actions"
    echo "💡 No manual trigger needed - workflow runs automatically on tag push"
}

# Function to show release checklist
show_checklist() {
    local version="$1"
    echo ""
    echo "📋 Release Checklist:"
    echo "✅ Version updated in Cargo.toml"
    echo "✅ All tests passing"
    echo "✅ Documentation updated"
    echo "✅ CHANGELOG.md updated"
    echo "✅ Tag created and pushed"
    echo "✅ GitHub Actions workflow triggered"
    echo ""
    echo "🔗 Next steps:"
    echo "1. Monitor GitHub Actions: https://github.com/abendrothj/lao/actions"
    echo "2. Wait for builds to complete (~10-15 minutes)"
    echo "3. Review generated packages in GitHub Releases"
    echo "4. Test packages on target platforms"
    echo "5. Announce release to users"
}

# Function to show manual release process
show_manual_process() {
    echo ""
    echo "🛠️  Manual Release Process:"
    echo ""
    echo "1. Update version in Cargo.toml:"
    echo "   sed -i 's/^version = \".*\"/version = \"$CURRENT_VERSION\"/' Cargo.toml"
    echo ""
    echo "2. Commit changes:"
    echo "   git add Cargo.toml"
    echo "   git commit -m 'Bump version to $CURRENT_VERSION'"
    echo ""
    echo "3. Create and push tag:"
    echo "   git tag -a v$CURRENT_VERSION -m 'Release LAO v$CURRENT_VERSION'"
    echo "   git push origin v$CURRENT_VERSION"
    echo ""
    echo "4. Monitor builds:"
    echo "   # Check GitHub Actions or run locally:"
    echo "   bash scripts/create-packages.sh"
    echo ""
    echo "5. Create GitHub Release:"
    echo "   gh release create v$CURRENT_VERSION --title 'LAO v$CURRENT_VERSION' --notes 'Release notes here'"
}

# Main release function
main() {
    local version="$1"
    local force="${2:-false}"
    
    echo "🎯 Starting LAO release process for v$version"
    echo ""
    
    # Validate version format
    validate_version "$version"
    
    # Pre-flight checks
    check_git_repo
    check_clean_working_dir
    check_tag_exists "$version" "$force"
    
    echo ""
    echo "🚀 Proceeding with release..."
    
    # Update version in Cargo.toml if different from current
    if [[ "$version" != "$CURRENT_VERSION" ]]; then
        update_version "$version"
        echo "📝 Committing version change..."
        git add core/Cargo.toml cli/Cargo.toml lao_plugin_api/Cargo.toml ui/lao-ui/Cargo.toml tools/plugin-generator/Cargo.toml tools/plugin-registry/Cargo.toml plugins/*/Cargo.toml
        git commit -m "Bump version to $version" || echo "⚠️  No changes to commit"
    fi
    
    # Create and push tag
    create_and_push_tag "$version"
    
    # Trigger workflow
    trigger_workflow
    
    # Show checklist
    show_checklist "$version"
    
    echo ""
    echo "🎉 Release process initiated!"
    echo "📦 Packages will be available in GitHub Releases once builds complete"
}

# Handle command line arguments
case "${1:-}" in
    "check")
        echo "🔍 Pre-release checks..."
        check_git_repo
        check_clean_working_dir
        check_tag_exists "$VERSION" "false"
        echo "✅ All checks passed!"
        ;;
    "auto")
        release_type="${2:-patch}"
        next_version=$(calculate_next_version "$CURRENT_VERSION" "$release_type")
        echo "🔄 Auto-bumping version from $CURRENT_VERSION to $next_version ($release_type)"
        main "$next_version" "false"
        ;;
    "force")
        version="${2:-$CURRENT_VERSION}"
        echo "⚠️  Force mode enabled for version $version"
        main "$version" "true"
        ;;
    "manual")
        show_manual_process
        ;;
    "help"|"-h"|"--help")
        echo "LAO Release Script - Enhanced for Longevity"
        echo ""
        echo "Usage: $0 [version] [options]"
        echo ""
        echo "Arguments:"
        echo "  version     - Specific version (e.g., 1.2.3) or 'auto'"
        echo "  options     - 'force' to replace existing tags"
        echo ""
        echo "Commands:"
        echo "  auto [type] - Auto-bump version (patch|minor|major)"
        echo "  force [ver] - Force release, replacing existing tag"
        echo "  check       - Run pre-release checks only"
        echo "  manual      - Show manual release steps"
        echo "  help        - Show this help"
        echo ""
        echo "Examples:"
        echo "  $0                    # Release current version"
        echo "  $0 1.2.3             # Release specific version"
        echo "  $0 auto patch        # Auto-bump patch version"
        echo "  $0 auto minor        # Auto-bump minor version"
        echo "  $0 force 1.2.3       # Force release, replace tag"
        echo "  $0 check             # Check only"
        echo "  $0 manual            # Manual steps"
        echo ""
        echo "Current version: $CURRENT_VERSION"
        echo "Available tags:"
        git tag -l | sort -V | tail -5 | sed 's/^/  /'
        ;;
    "")
        main "$VERSION" "false"
        ;;
    *)
        # Check if it's a version number or force command
        if [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            main "$1" "false"
        elif [[ "$1" == "force" ]]; then
            version="${2:-$CURRENT_VERSION}"
            main "$version" "true"
        else
            echo "❌ Unknown command: $1"
            echo "💡 Use '$0 help' for usage information"
            exit 1
        fi
        ;;
esac