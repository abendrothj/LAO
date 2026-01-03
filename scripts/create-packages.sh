#!/usr/bin/env bash
set -euo pipefail
# Disable job control globally to prevent fg/bg issues in CI environments
set +m

# Cross-platform packaging script for LAO
# Creates platform-specific packages for distribution

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/target"
DIST_DIR="$ROOT_DIR/dist"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/' || echo "0.1.0")
APP_NAME="lao"
APP_DESCRIPTION="Local AI Workflow Orchestrator by Jake Abendroth"

# Detect current platform
OS=$(uname -s 2>/dev/null || echo "Unknown")
case "$OS" in
    Linux*)   PLATFORM="linux" ;;
    Darwin*)  PLATFORM="macos" ;;
    MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
    *)        PLATFORM="unknown" ;;
esac

echo "📦 Creating packages for LAO v$VERSION on $PLATFORM"
echo "📁 Root directory: $ROOT_DIR"
echo "📁 Target directory: $TARGET_DIR"
echo "📁 Distribution directory: $DIST_DIR"

# Create distribution directory
mkdir -p "$DIST_DIR"

# Function to build release binaries
build_release() {
    echo "🔨 Building release binaries..."
    cargo build --release --bin lao-cli
    cargo build --release --bin lao-ui
    
    # Build plugins
    if [ -f "scripts/build-plugins.sh" ]; then
        bash scripts/build-plugins.sh
    fi
    
    echo "✅ Release build complete"
}

# Function to create Linux packages
create_linux_packages() {
    echo "🐧 Creating Linux packages..."
    
    # Create AppImage
    create_appimage
    
    # Create Debian package
    create_deb_package
    
    # Create RPM package
    create_rpm_package
    
    # Create tar.gz archive
    create_tar_archive
}

# Function to create AppImage
create_appimage() {
    echo "📱 Creating AppImage..."
    
    local appimage_dir="$DIST_DIR/AppImage"
    mkdir -p "$appimage_dir"
    
    # Create AppDir structure
    local appdir="$appimage_dir/LAO.AppDir"
    mkdir -p "$appdir/usr/bin"
    mkdir -p "$appdir/usr/share/applications"
    mkdir -p "$appdir/usr/share/icons"
    mkdir -p "$appdir/usr/share/metainfo"
    
    # Copy binaries
    cp "$TARGET_DIR/release/lao-cli" "$appdir/usr/bin/"
    cp "$TARGET_DIR/release/lao-ui" "$appdir/usr/bin/"
    
    # Copy plugins
    mkdir -p "$appdir/usr/lib/lao/plugins"
    cp plugins/*.so "$appdir/usr/lib/lao/plugins/" 2>/dev/null || true
    
    # Create desktop file
    cat > "$appdir/usr/share/applications/lao.desktop" << EOF
[Desktop Entry]
Name=LAO Orchestrator
Comment=$APP_DESCRIPTION
Exec=lao-ui
Icon=lao
Type=Application
Categories=Development;Utility;
StartupNotify=true
EOF
    
    # Create AppRun
    cat > "$appdir/AppRun" << 'EOF'
#!/bin/bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
exec "${HERE}/usr/bin/lao-ui" "$@"
EOF
    chmod +x "$appdir/AppRun"
    
    # Create metainfo
    cat > "$appdir/usr/share/metainfo/lao.appdata.xml" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>lao</id>
  <name>LAO Orchestrator</name>
  <summary>$APP_DESCRIPTION</summary>
  <description>
    <p>LAO is a cross-platform desktop tool for chaining local AI models and plugins into powerful, agentic workflows.</p>
  </description>
  <launchable type="desktop-id">lao.desktop</launchable>
  <url type="homepage">https://github.com/abendrothj/lao</url>
  <provides>
    <binary>lao-cli</binary>
    <binary>lao-ui</binary>
  </provides>
</component>
EOF
    
    echo "✅ AppImage structure created at $appdir"
    echo "💡 Use appimagetool to create final AppImage:"
    echo "   appimagetool $appdir $DIST_DIR/LAO-$VERSION-x86_64.AppImage"
}

# Function to create Debian package
create_deb_package() {
    echo "📦 Creating Debian package..."
    
    local deb_dir="$DIST_DIR/deb"
    local package_dir="$deb_dir/lao_${VERSION}_amd64"
    mkdir -p "$package_dir/DEBIAN"
    mkdir -p "$package_dir/usr/bin"
    mkdir -p "$package_dir/usr/lib/lao/plugins"
    mkdir -p "$package_dir/usr/share/applications"
    mkdir -p "$package_dir/usr/share/doc/lao"
    
    # Copy binaries
    cp "$TARGET_DIR/release/lao-cli" "$package_dir/usr/bin/"
    cp "$TARGET_DIR/release/lao-ui" "$package_dir/usr/bin/"
    
    # Copy plugins
    cp plugins/*.so "$package_dir/usr/lib/lao/plugins/" 2>/dev/null || true
    
    # Create control file
    cat > "$package_dir/DEBIAN/control" << EOF
Package: lao
Version: $VERSION
Section: devel
Priority: optional
Architecture: amd64
Depends: libc6, libgtk-3-0, libwebkit2gtk-4.0-37
Maintainer: Jake Abendroth <contact@jakea.net>
Description: $APP_DESCRIPTION
 LAO is a cross-platform desktop tool for chaining local AI models
 and plugins into powerful, agentic workflows. It supports prompt-driven
 orchestration, visual DAG editing, and full offline execution.
 .
 Features:
  * Modular plugin system (Rust, local-first)
  * Offline DAG engine (retries, caching, lifecycle hooks)
  * Prompt-driven agentic workflows (LLM-powered)
  * Visual workflow builder (UI, YAML export)
  * CLI interface for automation
EOF
    
    # Create desktop file
    cat > "$package_dir/usr/share/applications/lao.desktop" << EOF
[Desktop Entry]
Name=LAO Orchestrator
Comment=$APP_DESCRIPTION
Exec=lao-ui
Icon=lao
Type=Application
Categories=Development;Utility;
StartupNotify=true
EOF
    
    # Create copyright file
    cat > "$package_dir/usr/share/doc/lao/copyright" << EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: lao
Source: https://github.com/abendrothj/lao

Files: *
Copyright: 2024 Jake Abendroth
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files (the "Software"), to deal
 in the Software without restriction, including without limitation the rights
 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 copies of the Software, and to permit persons to whom the Software is
 furnished to do so, subject to the following conditions:
 .
 The above copyright notice and this permission notice shall be included in all
 copies or substantial portions of the Software.
EOF
    
    # Create changelog
    cat > "$package_dir/usr/share/doc/lao/changelog.Debian" << EOF
lao ($VERSION) unstable; urgency=medium

  * Initial release of LAO Orchestrator
  * Cross-platform AI workflow orchestration
  * Plugin system with dynamic loading
  * Visual workflow builder
  * CLI interface

 -- Jake Abendroth <contact@jakea.net>  $(date -R)
EOF
    gzip -9 "$package_dir/usr/share/doc/lao/changelog.Debian"
    
    # Set permissions
    chmod 755 "$package_dir/usr/bin"/*
    chmod 644 "$package_dir/usr/share/applications/lao.desktop"
    
    # Build package
    dpkg-deb --build "$package_dir" "$DIST_DIR/lao_${VERSION}_amd64.deb"
    
    echo "✅ Debian package created: $DIST_DIR/lao_${VERSION}_amd64.deb"
}

# Function to create RPM package
create_rpm_package() {
    echo "📦 Creating RPM package..."
    
    local rpm_dir="$DIST_DIR/rpm"
    local spec_dir="$rpm_dir/SPECS"
    local build_dir="$rpm_dir/BUILD"
    local rpmbuild_dir="$rpm_dir/RPMS"
    local sources_dir="$rpm_dir/SOURCES"
    
    mkdir -p "$spec_dir" "$build_dir" "$rpmbuild_dir" "$sources_dir"
    
    # Create spec file
    cat > "$spec_dir/lao.spec" << EOF
Name:           lao
Version:        $VERSION
Release:        1%{?dist}
Summary:        $APP_DESCRIPTION

License:        MIT
URL:            https://github.com/abendrothj/lao
Source0:        lao-%{version}.tar.gz

BuildRequires:  gcc
BuildRequires:  make
# Note: cargo and rust are typically installed via rustup in CI environments
# If these packages are not available, the build will use pre-built binaries
Requires:       gtk3
Requires:       webkit2gtk3

%description
LAO is a cross-platform desktop tool for chaining local AI models and plugins
into powerful, agentic workflows. It supports prompt-driven orchestration,
visual DAG editing, and full offline execution.

%prep
%setup -q

%build
# Use pre-built binaries instead of building from source
# This avoids dependency issues in CI environments

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/lib/lao/plugins
mkdir -p %{buildroot}/usr/share/applications

# Install pre-built binaries from the tarball
install -m 755 target/release/lao-cli %{buildroot}/usr/bin/
install -m 755 target/release/lao-ui %{buildroot}/usr/bin/
install -m 644 plugins/*.so %{buildroot}/usr/lib/lao/plugins/ 2>/dev/null || true

cat > %{buildroot}/usr/share/applications/lao.desktop << 'DESKTOP_EOF'
[Desktop Entry]
Name=LAO Orchestrator
Comment=$APP_DESCRIPTION
Exec=lao-ui
Icon=lao
Type=Application
Categories=Development;Utility;
StartupNotify=true
DESKTOP_EOF

%files
/usr/bin/lao-cli
/usr/bin/lao-ui
/usr/lib/lao/plugins/*.so
/usr/share/applications/lao.desktop

%changelog
* $(date '+%a %b %d %Y') Jake Abendroth <contact@jakea.net> - $VERSION-1
- Initial release of LAO Orchestrator
EOF
    
    # Create source tarball with proper directory structure
    # RPM expects the tarball to extract to a directory named lao-VERSION
    local temp_dir="/tmp/lao-tarball-$$"
    mkdir -p "$temp_dir/lao-$VERSION"
    
    # Copy source files to the temp directory
    cp -r "$ROOT_DIR"/* "$temp_dir/lao-$VERSION/" 2>/dev/null || true
    cp -r "$ROOT_DIR"/.[^.]* "$temp_dir/lao-$VERSION/" 2>/dev/null || true
    
    # Remove excluded directories
    rm -rf "$temp_dir/lao-$VERSION/target" 2>/dev/null || true
    rm -rf "$temp_dir/lao-$VERSION/.git" 2>/dev/null || true
    rm -rf "$temp_dir/lao-$VERSION/dist" 2>/dev/null || true
    
    # Copy pre-built binaries to the tarball for RPM installation
    mkdir -p "$temp_dir/lao-$VERSION/target/release"
    cp "$ROOT_DIR/target/release/lao-cli" "$temp_dir/lao-$VERSION/target/release/" 2>/dev/null || true
    cp "$ROOT_DIR/target/release/lao-ui" "$temp_dir/lao-$VERSION/target/release/" 2>/dev/null || true
    
    # Create tarball from the temp directory
    tar -czf "$sources_dir/lao-$VERSION.tar.gz" \
        -C "$temp_dir" "lao-$VERSION"
    
    # Clean up temp directory
    rm -rf "$temp_dir"
    
    # Check if rpmbuild is available
    if ! command -v rpmbuild >/dev/null 2>&1; then
        echo "⚠️  rpmbuild not found, skipping RPM package creation"
        echo "💡 Install rpm-build package to enable RPM creation"
        return 0
    fi
    
    # Build RPM
    echo "🔨 Building RPM package..."
    
    # Try a different approach: Use rpmbuild with specific options to avoid job control
    echo "Debug: Running rpmbuild with specific options..."
    echo "Debug: RPM directories:"
    echo "  spec_dir: $spec_dir"
    echo "  rpm_dir: $rpm_dir"
    echo "  rpmbuild_dir: $rpmbuild_dir"
    echo "  sources_dir: $sources_dir"
    
    # List contents of key directories for debugging
    echo "Debug: Contents of sources directory:"
    ls -la "$sources_dir/" || echo "Sources directory not found"
    
    echo "Debug: Contents of spec directory:"
    ls -la "$spec_dir/" || echo "Spec directory not found"
    
    # Try building with --quiet and --nodeps to minimize interaction
    echo "Attempting RPM build with --quiet --nodeps..."
    if rpmbuild --define "_topdir $rpm_dir" -bb "$spec_dir/lao.spec" --quiet --nodeps; then
        echo "RPM build succeeded with --quiet --nodeps"
        echo "Debug: Contents of RPM build directory after build:"
        ls -la "$rpmbuild_dir/" || echo "RPM build directory not found"
        ls -la "$rpmbuild_dir/x86_64/" || echo "x86_64 directory not found"
        # Check if RPM file was actually created
        if ls "$rpmbuild_dir/x86_64/lao-$VERSION-1."*.rpm 1> /dev/null 2>&1; then
            cp "$rpmbuild_dir/x86_64/lao-$VERSION-1."*.rpm "$DIST_DIR/"
            echo "✅ RPM package created: $DIST_DIR/lao-$VERSION-1.*.rpm"
        else
            echo "❌ RPM build succeeded but no package file found"
            return 1
        fi
    else
        echo "❌ RPM build failed with --quiet --nodeps, trying without quiet mode..."
        # Try without --quiet flag
        if rpmbuild --define "_topdir $rpm_dir" -bb "$spec_dir/lao.spec" --nodeps; then
            echo "RPM build succeeded with --nodeps"
            # Check if RPM file was actually created
            if ls "$rpmbuild_dir/x86_64/lao-$VERSION-1."*.rpm 1> /dev/null 2>&1; then
                cp "$rpmbuild_dir/x86_64/lao-$VERSION-1."*.rpm "$DIST_DIR/"
                echo "✅ RPM package created: $DIST_DIR/lao-$VERSION-1.*.rpm"
            else
                echo "❌ RPM build succeeded but no package file found"
                return 1
            fi
        else
            echo "❌ RPM build failed with --nodeps, trying with minimal options..."
            # Try with minimal options
            if rpmbuild --define "_topdir $rpm_dir" -bb "$spec_dir/lao.spec"; then
                echo "RPM build succeeded with minimal options"
                # Check if RPM file was actually created
                if ls "$rpmbuild_dir/x86_64/lao-$VERSION-1."*.rpm 1> /dev/null 2>&1; then
                    cp "$rpmbuild_dir/x86_64/lao-$VERSION-1."*.rpm "$DIST_DIR/"
                    echo "✅ RPM package created: $DIST_DIR/lao-$VERSION-1.*.rpm"
                else
                    echo "❌ RPM build succeeded but no package file found"
                    return 1
                fi
            else
                echo "❌ RPM build failed - this may be due to missing dependencies"
                echo "💡 Try installing: sudo apt-get install rpm-build"
                return 1
            fi
        fi
    fi
}

# Function to create tar archive
create_tar_archive() {
    echo "📦 Creating tar archive..."
    
    local archive_dir="$DIST_DIR/tar"
    local package_dir="$archive_dir/lao-$VERSION"
    mkdir -p "$package_dir"
    
    # Copy binaries
    cp "$TARGET_DIR/release/lao-cli" "$package_dir/"
    cp "$TARGET_DIR/release/lao-ui" "$package_dir/"
    
    # Copy plugins
    mkdir -p "$package_dir/plugins"
    cp plugins/*.so "$package_dir/plugins/" 2>/dev/null || true
    
    # Copy documentation
    cp README.md "$package_dir/"
    cp LICENSE "$package_dir/" 2>/dev/null || true
    
    # Create install script
    cat > "$package_dir/install.sh" << 'EOF'
#!/bin/bash
set -e

echo "Installing LAO Orchestrator..."

# Create directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /usr/local/lib/lao/plugins

# Install binaries
sudo cp lao-cli /usr/local/bin/
sudo cp lao-ui /usr/local/bin/
sudo chmod +x /usr/local/bin/lao-*

# Install plugins
sudo cp plugins/*.so /usr/local/lib/lao/plugins/ 2>/dev/null || true

# Create desktop file
sudo mkdir -p /usr/share/applications
sudo tee /usr/share/applications/lao.desktop > /dev/null << 'DESKTOP'
[Desktop Entry]
Name=LAO Orchestrator
Comment=Local AI Workflow Orchestrator
Exec=lao-ui
Icon=lao
Type=Application
Categories=Development;Utility;
StartupNotify=true
DESKTOP

echo "✅ LAO installed successfully!"
echo "Run 'lao-ui' to start the GUI or 'lao-cli --help' for CLI options"
EOF
    chmod +x "$package_dir/install.sh"
    
    # Create archive
    tar -czf "$DIST_DIR/lao-$VERSION-linux-x86_64.tar.gz" -C "$archive_dir" "lao-$VERSION"
    
    echo "✅ Tar archive created: $DIST_DIR/lao-$VERSION-linux-x86_64.tar.gz"
}

# Function to create macOS packages
create_macos_packages() {
    echo "🍎 Creating macOS packages..."
    
    # Create DMG
    create_dmg_package
    
    # Create tar.gz archive
    create_macos_tar_archive
}

# Function to create DMG package
create_dmg_package() {
    echo "💿 Creating DMG package..."
    
    local dmg_dir="$DIST_DIR/dmg"
    local app_dir="$dmg_dir/LAO.app"
    mkdir -p "$app_dir/Contents/MacOS"
    mkdir -p "$app_dir/Contents/Resources"
    mkdir -p "$app_dir/Contents/PlugIns"
    
    # Copy binaries
    cp "$TARGET_DIR/release/lao-cli" "$app_dir/Contents/MacOS/"
    cp "$TARGET_DIR/release/lao-ui" "$app_dir/Contents/MacOS/"
    
    # Copy plugins
    mkdir -p "$app_dir/Contents/PlugIns/plugins"
    cp plugins/*.dylib "$app_dir/Contents/PlugIns/plugins/" 2>/dev/null || true
    
    # Set proper permissions
    chmod +x "$app_dir/Contents/MacOS/lao-cli"
    chmod +x "$app_dir/Contents/MacOS/lao-ui"
    
    # Create Info.plist
    cat > "$app_dir/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>lao-ui</string>
    <key>CFBundleIdentifier</key>
    <string>dev.lao.orchestrator</string>
    <key>CFBundleName</key>
    <string>LAO Orchestrator</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF
    
    # Create DMG with proper settings to avoid corruption
    echo "Creating DMG with hdiutil..."
    echo "Debug: DMG directory contents:"
    ls -la "$dmg_dir/"
    
    # First, create a temporary DMG
    local temp_dmg="$DIST_DIR/LAO-$VERSION-temp.dmg"
    local final_dmg="$DIST_DIR/LAO-$VERSION.dmg"
    
    # Calculate size needed (add some padding)
    local size_mb=$(du -sm "$dmg_dir" | cut -f1)
    local size_mb=$((size_mb + 20))  # Add 20MB padding for safety
    echo "Debug: Calculated DMG size: ${size_mb}MB"
    
    # Create temporary DMG
    echo "Creating temporary DMG..."
    if ! hdiutil create -size "${size_mb}m" -fs HFS+ -volname "LAO $VERSION" "$temp_dmg"; then
        echo "❌ Failed to create temporary DMG"
        return 1
    fi
    
    # Mount the DMG at a fixed, known mount point to avoid parsing issues with spaces
    echo "Mounting DMG..."
    local mount_point="$DIST_DIR/LAO-$VERSION-mnt"
    mkdir -p "$mount_point"
    if ! hdiutil attach "$temp_dmg" -mountpoint "$mount_point" -nobrowse -quiet; then
        echo "❌ Failed to mount DMG"
        rmdir "$mount_point" 2>/dev/null || true
        return 1
    fi
    
    # Copy contents to mounted DMG
    echo "Copying contents to DMG..."
    if ! cp -R "$dmg_dir"/* "$mount_point/"; then
        echo "❌ Failed to copy contents to DMG"
        hdiutil detach "$mount_point" 2>/dev/null || true
        return 1
    fi
    
    # Unmount the DMG
    echo "Unmounting DMG..."
    if ! hdiutil detach "$mount_point" -quiet; then
        echo "❌ Failed to unmount DMG"
        return 1
    fi
    rmdir "$mount_point" 2>/dev/null || true
    sync
    sleep 1
    
    # Convert to compressed format (overwrite if exists)
    echo "Converting to compressed format..."
    if [ -f "$final_dmg" ]; then rm -f "$final_dmg"; fi
    if ! hdiutil convert "$temp_dmg" -format UDZO -o "$final_dmg" -ov; then
        echo "❌ Failed to convert DMG to compressed format"
        rm -f "$temp_dmg"
        return 1
    fi
    
    # Clean up temporary file
    rm -f "$temp_dmg"
    
    # Verify the DMG
    echo "Verifying DMG..."
    if hdiutil verify "$final_dmg"; then
        echo "✅ DMG package created and verified: $final_dmg"
        echo "Debug: DMG file size: $(ls -lh "$final_dmg" | awk '{print $5}')"
    else
        echo "❌ DMG verification failed"
        return 1
    fi
}

# Function to create macOS tar archive
create_macos_tar_archive() {
    echo "📦 Creating macOS tar archive..."
    
    local archive_dir="$DIST_DIR/tar"
    local package_dir="$archive_dir/lao-$VERSION-macos"
    mkdir -p "$package_dir"
    
    # Copy binaries
    cp "$TARGET_DIR/release/lao-cli" "$package_dir/"
    cp "$TARGET_DIR/release/lao-ui" "$package_dir/"
    
    # Copy plugins
    mkdir -p "$package_dir/plugins"
    cp plugins/*.dylib "$package_dir/plugins/" 2>/dev/null || true
    
    # Copy documentation
    cp README.md "$package_dir/"
    cp LICENSE "$package_dir/" 2>/dev/null || true
    
    # Create install script
    cat > "$package_dir/install.sh" << 'EOF'
#!/bin/bash
set -e

echo "Installing LAO Orchestrator on macOS..."

# Create directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /usr/local/lib/lao/plugins

# Install binaries
sudo cp lao-cli /usr/local/bin/
sudo cp lao-ui /usr/local/bin/
sudo chmod +x /usr/local/bin/lao-*

# Install plugins
sudo cp plugins/*.dylib /usr/local/lib/lao/plugins/ 2>/dev/null || true

echo "✅ LAO installed successfully!"
echo "Run 'lao-ui' to start the GUI or 'lao-cli --help' for CLI options"
EOF
    chmod +x "$package_dir/install.sh"
    
    # Create archive
    tar -czf "$DIST_DIR/lao-$VERSION-macos-x86_64.tar.gz" -C "$archive_dir" "lao-$VERSION-macos"
    
    echo "✅ macOS tar archive created: $DIST_DIR/lao-$VERSION-macos-x86_64.tar.gz"
}

# Function to create Windows packages
create_windows_packages() {
    echo "🪟 Creating Windows packages..."
    
    # Create MSI installer
    create_msi_package
    
    # Create ZIP archive
    create_windows_zip_archive
}

# Function to create MSI package
create_msi_package() {
    echo "📦 Creating MSI package..."
    
    local msi_dir="$DIST_DIR/msi"
    mkdir -p "$msi_dir"
    
    # Create WiX configuration
    cat > "$msi_dir/lao.wxs" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*" Name="LAO Orchestrator" Language="1033" Version="$VERSION" Manufacturer="Jake Abendroth" UpgradeCode="581ACBB9-CC79-4763-BD13-ED19A5CBB960">
    <Package InstallerVersion="200" Compressed="yes" InstallScope="perMachine" />
    
    <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
    <MediaTemplate />
    
    <Feature Id="ProductFeature" Title="LAO Orchestrator" Level="1">
      <ComponentGroupRef Id="ProductComponents" />
    </Feature>
    
    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFilesFolder">
        <Directory Id="INSTALLFOLDER" Name="LAO">
          <Component Id="LaoCliCmp" Guid="*">
            <File Id="LaoCliExe" Source="target/release/lao-cli.exe" KeyPath="yes" />
          </Component>
          <Component Id="LaoUiCmp" Guid="*">
            <File Id="LaoUiExe" Source="target/release/lao-ui.exe" KeyPath="yes" />
          </Component>
          <Directory Id="PluginsFolder" Name="plugins">
            <Component Id="EchoPluginCmp" Guid="*">
              <File Id="EchoPluginDll" Source="plugins/echo_plugin.dll" KeyPath="yes" />
            </Component>
            <Component Id="OllamaPluginCmp" Guid="*">
              <File Id="OllamaPluginDll" Source="plugins/ollama_plugin.dll" KeyPath="yes" />
            </Component>
            <Component Id="WhisperPluginCmp" Guid="*">
              <File Id="WhisperPluginDll" Source="plugins/whisper_plugin.dll" KeyPath="yes" />
            </Component>
          </Directory>
        </Directory>
      </Directory>
    </Directory>
    
    <ComponentGroup Id="ProductComponents" Directory="INSTALLFOLDER">
      <ComponentRef Id="LaoCliCmp" />
      <ComponentRef Id="LaoUiCmp" />
      <ComponentRef Id="EchoPluginCmp" />
      <ComponentRef Id="OllamaPluginCmp" />
      <ComponentRef Id="WhisperPluginCmp" />
    </ComponentGroup>
  </Product>
</Wix>
EOF
    
    echo "✅ WiX configuration created: $msi_dir/lao.wxs"
    echo "🔧 Building MSI with WiX Toolset..."
    local ps_script="$msi_dir/build-wix.ps1"
    cat > "$ps_script" <<'EOF'
param(
    [Parameter(Mandatory=$true)][string]$Version,
    [Parameter(Mandatory=$true)][string]$MsiDir,
    [Parameter(Mandatory=$true)][string]$DistDir
)

trap { Write-Error $_; exit 1 }

function Has-Command {
    param([string]$Name)
    return (Get-Command $Name -ErrorAction SilentlyContinue)
}

if (-not (Has-Command 'candle.exe')) {
    Write-Host 'Installing WiX Toolset via Chocolatey...'
    if (-not (Has-Command 'choco.exe')) {
        Write-Host 'Chocolatey not found; installing Chocolatey...'
        Set-ExecutionPolicy Bypass -Scope Process -Force
        [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
        Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    }
    choco install wixtoolset -y --no-progress
}

$wixBin = (Get-Command candle.exe | Select-Object -First 1).Source | Split-Path
$env:Path = $wixBin + ';' + $env:Path
$msiPath = (Resolve-Path $MsiDir).Path
$distPath = (Resolve-Path $DistDir).Path
$wxs = Join-Path $msiPath 'lao.wxs'
$wixobj = Join-Path $msiPath 'lao.wixobj'
$msiOut = Join-Path $distPath ("lao-$Version-x86_64.msi")

Write-Host ("Using WiX bin: $wixBin")

& candle.exe -nologo -o $wixobj $wxs
if ($LASTEXITCODE -ne 0) { throw 'candle.exe failed' }

& light.exe -nologo -o $msiOut $wixobj
if ($LASTEXITCODE -ne 0) { throw 'light.exe failed' }
EOF

    powershell -NoProfile -ExecutionPolicy Bypass -File "$ps_script" -Version "$VERSION" -MsiDir "$msi_dir" -DistDir "$DIST_DIR"
    rm -f "$ps_script"
    echo "✅ MSI created: $DIST_DIR/lao-$VERSION-x86_64.msi"
}

# Function to create Windows ZIP archive
create_windows_zip_archive() {
    echo "📦 Creating Windows ZIP archive..."
    
    local archive_dir="$DIST_DIR/zip"
    local package_dir="$archive_dir/lao-$VERSION-windows"
    mkdir -p "$package_dir"
    
    # Copy binaries
    cp "$TARGET_DIR/release/lao-cli.exe" "$package_dir/" 2>/dev/null || true
    cp "$TARGET_DIR/release/lao-ui.exe" "$package_dir/" 2>/dev/null || true
    
    # Copy plugins
    mkdir -p "$package_dir/plugins"
    cp plugins/*.dll "$package_dir/plugins/" 2>/dev/null || true
    
    # Copy documentation
    cp README.md "$package_dir/"
    cp LICENSE "$package_dir/" 2>/dev/null || true
    
    # Create install script
    cat > "$package_dir/install.bat" << 'EOF'
@echo off
echo Installing LAO Orchestrator on Windows...

REM Create directories
mkdir "%ProgramFiles%\LAO" 2>nul
mkdir "%ProgramFiles%\LAO\plugins" 2>nul

REM Install binaries
copy "lao-cli.exe" "%ProgramFiles%\LAO\" >nul
copy "lao-ui.exe" "%ProgramFiles%\LAO\" >nul

REM Install plugins
copy "plugins\*.dll" "%ProgramFiles%\LAO\plugins\" >nul

REM Add to PATH
setx PATH "%PATH%;%ProgramFiles%\LAO" /M >nul

echo LAO installed successfully!
echo Run 'lao-ui.exe' to start the GUI or 'lao-cli.exe --help' for CLI options
pause
EOF
    
    # Create archive
    cd "$archive_dir"
    
    # Check if zip command is available
    if command -v zip >/dev/null 2>&1; then
        zip -r "$DIST_DIR/lao-$VERSION-windows-x86_64.zip" "lao-$VERSION-windows"
        echo "✅ Windows ZIP archive created: $DIST_DIR/lao-$VERSION-windows-x86_64.zip"
    else
        echo "⚠️  zip command not found, creating tar.gz instead"
        tar -czf "$DIST_DIR/lao-$VERSION-windows-x86_64.tar.gz" "lao-$VERSION-windows"
        echo "✅ Windows tar.gz archive created: $DIST_DIR/lao-$VERSION-windows-x86_64.tar.gz"
    fi
    
    cd "$ROOT_DIR"
}

# Main packaging function
main() {
    echo "🚀 Starting LAO packaging process..."
    
    # Build release binaries
    build_release
    
    # Create platform-specific packages
    case "$PLATFORM" in
        "linux")
            create_linux_packages
            ;;
        "macos")
            create_macos_packages
            ;;
        "windows")
            create_windows_packages
            ;;
        *)
            echo "❌ Unsupported platform: $PLATFORM"
            exit 1
            ;;
    esac
    
    echo "🎉 Packaging complete!"
    echo "📦 Packages created in: $DIST_DIR"
    echo "📋 Available packages:"
    ls -la "$DIST_DIR"/*.{deb,rpm,dmg,msi,zip,tar.gz} 2>/dev/null || echo "  No packages found"
}

# Run main function
main "$@"
