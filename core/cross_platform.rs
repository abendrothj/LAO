// Cross-platform utilities for LAO
// Provides platform detection, path handling, and OS-specific functionality

use std::env;
use std::path::{Path, PathBuf};

/// Platform detection utilities
pub struct Platform;

impl Platform {
    /// Get the current operating system
    pub fn os() -> &'static str {
        env::consts::OS
    }

    /// Get the current architecture
    pub fn arch() -> &'static str {
        env::consts::ARCH
    }

    /// Get the current family (unix, windows)
    pub fn family() -> &'static str {
        env::consts::FAMILY
    }

    /// Check if running on Linux
    pub fn is_linux() -> bool {
        Self::os() == "linux"
    }

    /// Check if running on macOS
    pub fn is_macos() -> bool {
        Self::os() == "macos"
    }

    /// Check if running on Windows
    pub fn is_windows() -> bool {
        Self::os() == "windows"
    }

    /// Get the shared library extension for current platform
    pub fn shared_lib_extension() -> &'static str {
        match Self::os() {
            "linux" => "so",
            "macos" => "dylib",
            "windows" => "dll",
            _ => "so", // Default to Linux extension
        }
    }

    /// Get the shared library prefix for current platform
    pub fn shared_lib_prefix() -> &'static str {
        match Self::os() {
            "windows" => "",
            _ => "lib",
        }
    }

    /// Check if a file extension is a shared library for current platform
    pub fn is_shared_lib_extension(ext: &str) -> bool {
        ext == Self::shared_lib_extension()
    }

    /// Check if a file is a shared library for current platform
    pub fn is_shared_lib_file(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            Self::is_shared_lib_extension(ext)
        } else {
            false
        }
    }

    /// Get the executable extension for current platform
    pub fn exe_extension() -> &'static str {
        match Self::os() {
            "windows" => "exe",
            _ => "",
        }
    }

    /// Get the home directory for current platform
    pub fn home_dir() -> Option<PathBuf> {
        env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE")) // Windows fallback
            .map(PathBuf::from)
    }

    /// Get the config directory for current platform
    pub fn config_dir() -> Option<PathBuf> {
        match Self::os() {
            "windows" => env::var_os("APPDATA").map(PathBuf::from),
            _ => Self::home_dir().map(|home| home.join(".config")),
        }
    }

    /// Get the cache directory for current platform
    pub fn cache_dir() -> Option<PathBuf> {
        match Self::os() {
            "windows" => env::var_os("LOCALAPPDATA").map(|path| PathBuf::from(path).join("cache")),
            "macos" => Self::home_dir().map(|home| home.join("Library").join("Caches")),
            _ => Self::home_dir().map(|home| home.join(".cache")),
        }
    }

    /// Get the data directory for current platform
    pub fn data_dir() -> Option<PathBuf> {
        match Self::os() {
            "windows" => env::var_os("LOCALAPPDATA").map(PathBuf::from),
            "macos" => {
                Self::home_dir().map(|home| home.join("Library").join("Application Support"))
            }
            _ => Self::home_dir().map(|home| home.join(".local").join("share")),
        }
    }
}

/// Path utilities for cross-platform compatibility
pub struct PathUtils;

impl PathUtils {
    /// Normalize a path for the current platform
    pub fn normalize(path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    /// Join paths in a cross-platform way
    pub fn join<P: AsRef<Path>>(base: &Path, path: P) -> PathBuf {
        base.join(path)
    }

    /// Get the LAO plugin directory
    pub fn plugin_dir() -> PathBuf {
        // Try environment variable first
        if let Ok(plugin_dir) = env::var("LAO_PLUGIN_DIR") {
            return PathBuf::from(plugin_dir);
        }

        // Get current directory
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Check if we're in a subdirectory (like core/) and plugins/ exists in parent
        let plugins_in_current = current_dir.join("plugins");
        let plugins_in_parent = current_dir.parent().map(|p| p.join("plugins"));

        if plugins_in_current.exists() {
            plugins_in_current
        } else if let Some(parent_plugins) = plugins_in_parent {
            if parent_plugins.exists() {
                parent_plugins
            } else {
                plugins_in_current
            }
        } else {
            plugins_in_current
        }
    }

    /// Get the LAO cache directory
    pub fn cache_dir() -> PathBuf {
        // Try environment variable first
        if let Ok(cache_dir) = env::var("LAO_CACHE_DIR") {
            return PathBuf::from(cache_dir);
        }

        // Use platform-specific cache directory
        Platform::cache_dir()
            .unwrap_or_else(|| PathBuf::from("cache"))
            .join("lao")
    }

    /// Get the LAO config directory
    pub fn config_dir() -> PathBuf {
        // Try environment variable first
        if let Ok(config_dir) = env::var("LAO_CONFIG_DIR") {
            return PathBuf::from(config_dir);
        }

        // Use platform-specific config directory
        Platform::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("lao")
    }
}

/// Environment utilities for cross-platform compatibility
pub struct EnvUtils;

impl EnvUtils {
    /// Get an environment variable with platform-specific fallbacks
    pub fn get_with_fallback(key: &str, fallbacks: &[&str]) -> Option<String> {
        // Try primary key first
        if let Ok(value) = env::var(key) {
            return Some(value);
        }

        // Try fallback keys
        for fallback in fallbacks {
            if let Ok(value) = env::var(fallback) {
                return Some(value);
            }
        }

        None
    }

    /// Get the PATH environment variable
    pub fn path() -> Option<String> {
        env::var("PATH").ok()
    }

    /// Add a directory to PATH (for current process)
    pub fn add_to_path(dir: &Path) -> Result<(), String> {
        let current_path = env::var("PATH").unwrap_or_default();
        let new_path = if current_path.is_empty() {
            dir.to_string_lossy().to_string()
        } else {
            format!("{}:{}", dir.to_string_lossy(), current_path)
        };

        env::set_var("PATH", new_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let os = Platform::os();
        assert!(!os.is_empty());

        let arch = Platform::arch();
        assert!(!arch.is_empty());

        let family = Platform::family();
        assert!(family == "unix" || family == "windows");
    }

    #[test]
    fn test_shared_lib_extension() {
        let ext = Platform::shared_lib_extension();
        assert!(!ext.is_empty());

        assert!(Platform::is_shared_lib_extension(ext));
    }

    #[test]
    fn test_path_utils() {
        let plugin_dir = PathUtils::plugin_dir();
        assert!(plugin_dir.is_absolute() || plugin_dir.starts_with("plugins"));

        let cache_dir = PathUtils::cache_dir();
        assert!(!cache_dir.to_string_lossy().is_empty());
    }
}
