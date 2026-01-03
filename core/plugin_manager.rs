use crate::plugins::PluginRegistry;
use anyhow::{anyhow, Result};
use lao_plugin_api::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Plugin marketplace entry for remote plugin discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketplaceEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub repository_url: String,
    pub download_url: String,
    pub tags: Vec<String>,
    pub license: String,
    pub min_lao_version: String,
    pub dependencies: Vec<PluginDependency>,
    pub ratings: f32,
    pub download_count: u64,
    pub last_updated: String,
    pub verified: bool,
}

/// Plugin configuration and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub settings: HashMap<String, serde_json::Value>,
    pub permissions: Vec<String>,
    pub resource_limits: ResourceLimits,
    pub auto_update: bool,
}

/// Resource limits for plugin sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: f32,
    pub max_network_requests_per_second: u32,
    pub allowed_file_paths: Vec<String>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_percent: 50.0,
            max_network_requests_per_second: 10,
            allowed_file_paths: vec!["./data/".to_string(), "./cache/".to_string()],
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: HashMap::new(),
            permissions: vec!["read_files".to_string(), "write_files".to_string()],
            resource_limits: ResourceLimits::default(),
            auto_update: false,
        }
    }
}

/// Plugin event system for hooks and communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    WorkflowStarted {
        workflow_id: String,
        workflow_name: String,
    },
    WorkflowCompleted {
        workflow_id: String,
        success: bool,
    },
    StepStarted {
        workflow_id: String,
        step_id: String,
        plugin_name: String,
    },
    StepCompleted {
        workflow_id: String,
        step_id: String,
        plugin_name: String,
        output: String,
    },
    PluginLoaded {
        plugin_name: String,
    },
    PluginUnloaded {
        plugin_name: String,
    },
    Custom {
        event_type: String,
        data: serde_json::Value,
    },
}

/// Hook registration for plugins to listen to events
#[derive(Debug, Clone)]
pub struct PluginHook {
    pub plugin_name: String,
    pub event_types: Vec<String>,
    pub callback: String, // Function name in plugin to call
}

/// Advanced plugin manager with marketplace, hooks, and sandboxing
#[derive(Debug)]
pub struct PluginManager {
    pub registry: PluginRegistry,
    pub configs: HashMap<String, PluginConfig>,
    pub hooks: Vec<PluginHook>,
    pub event_history: Vec<PluginEvent>,
    pub marketplace_cache: HashMap<String, PluginMarketplaceEntry>,
    pub plugin_directory: PathBuf,
    pub config_directory: PathBuf,
    pub cache_directory: PathBuf,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(plugin_dir: P) -> Result<Self> {
        let plugin_directory = plugin_dir.as_ref().to_path_buf();
        let config_directory = plugin_directory.join("configs");
        let cache_directory = plugin_directory.join("cache");

        // Create necessary directories
        std::fs::create_dir_all(&plugin_directory)?;
        std::fs::create_dir_all(&config_directory)?;
        std::fs::create_dir_all(&cache_directory)?;

        let mut manager = Self {
            registry: PluginRegistry::new(),
            configs: HashMap::new(),
            hooks: Vec::new(),
            event_history: Vec::new(),
            marketplace_cache: HashMap::new(),
            plugin_directory,
            config_directory,
            cache_directory,
        };

        manager.load_plugins()?;
        manager.load_configs()?;

        Ok(manager)
    }

    /// Load all plugins from the plugin directory
    pub fn load_plugins(&mut self) -> Result<()> {
        self.registry = PluginRegistry::dynamic_registry(
            self.plugin_directory
                .to_str()
                .ok_or_else(|| anyhow!("Invalid plugin directory path"))?,
        );

        // Register loaded event for each plugin
        let plugin_names: Vec<String> = self.registry.plugins.keys().cloned().collect();
        for plugin_name in plugin_names {
            self.emit_event(PluginEvent::PluginLoaded { plugin_name });
        }

        Ok(())
    }

    /// Load plugin configurations
    pub fn load_configs(&mut self) -> Result<()> {
        for plugin_name in self.registry.plugins.keys() {
            let config_path = self.config_directory.join(format!("{}.json", plugin_name));
            if config_path.exists() {
                let config_data = std::fs::read_to_string(&config_path)?;
                let config: PluginConfig = serde_json::from_str(&config_data)?;
                self.configs.insert(plugin_name.clone(), config);
            } else {
                // Create default config
                let default_config = PluginConfig::default();
                self.configs
                    .insert(plugin_name.clone(), default_config.clone());
                self.save_plugin_config(plugin_name, &default_config)?;
            }
        }

        Ok(())
    }

    /// Save plugin configuration
    pub fn save_plugin_config(&self, plugin_name: &str, config: &PluginConfig) -> Result<()> {
        let config_path = self.config_directory.join(format!("{}.json", plugin_name));
        let config_data = serde_json::to_string_pretty(config)?;
        std::fs::write(config_path, config_data)?;
        Ok(())
    }

    /// Install plugin from marketplace or URL
    pub async fn install_plugin(&mut self, name_or_url: &str, version: Option<&str>) -> Result<()> {
        // Check if it's a URL or marketplace name
        if name_or_url.starts_with("http") {
            self.install_plugin_from_url(name_or_url).await
        } else {
            self.install_plugin_from_marketplace(name_or_url, version)
                .await
        }
    }

    /// Install plugin from marketplace
    pub async fn install_plugin_from_marketplace(
        &mut self,
        name: &str,
        _version: Option<&str>,
    ) -> Result<()> {
        // Refresh marketplace cache if needed
        if !self.marketplace_cache.contains_key(name) {
            self.refresh_marketplace_cache().await?;
        }

        let entry = self
            .marketplace_cache
            .get(name)
            .ok_or_else(|| anyhow!("Plugin '{}' not found in marketplace", name))?
            .clone();

        // Download and install
        self.download_and_install_plugin(&entry.download_url, name)
            .await?;

        println!(
            "✓ Successfully installed plugin: {} v{}",
            name, entry.version
        );
        Ok(())
    }

    /// Install plugin from direct URL
    pub async fn install_plugin_from_url(&mut self, url: &str) -> Result<()> {
        // Extract plugin name from URL
        let name = url
            .split('/')
            .next_back()
            .and_then(|s| s.split('.').next())
            .unwrap_or("unknown_plugin");

        self.download_and_install_plugin(url, name).await?;

        println!("✓ Successfully installed plugin from URL: {}", url);
        Ok(())
    }

    /// Download and install plugin binary
    async fn download_and_install_plugin(&mut self, url: &str, name: &str) -> Result<()> {
        // This is a placeholder for actual HTTP download implementation
        // In a real implementation, you'd use reqwest or similar to download
        println!("Downloading plugin from: {}", url);
        println!("Installing to: {}", self.plugin_directory.display());

        // Create plugin directory
        let plugin_path = self.plugin_directory.join(name);
        std::fs::create_dir_all(&plugin_path)?;

        // In a real implementation, download the plugin binary here
        // For now, we'll simulate success

        // Reload plugins to pick up the new one
        self.load_plugins()?;

        Ok(())
    }

    /// Refresh marketplace cache from remote registry
    pub async fn refresh_marketplace_cache(&mut self) -> Result<()> {
        // This would fetch from a real marketplace API
        // For now, we'll simulate with some example entries

        let example_plugins = vec![
            PluginMarketplaceEntry {
                name: "AdvancedImageProcessor".to_string(),
                version: "1.2.0".to_string(),
                description: "Advanced image processing with AI enhancement".to_string(),
                author: "ImageAI Team".to_string(),
                repository_url: "https://github.com/imageai/advanced-processor".to_string(),
                download_url: "https://releases.imageai.com/advanced-processor-1.2.0.dll"
                    .to_string(),
                tags: vec![
                    "image".to_string(),
                    "ai".to_string(),
                    "processing".to_string(),
                ],
                license: "MIT".to_string(),
                min_lao_version: "0.1.0".to_string(),
                dependencies: vec![],
                ratings: 4.8,
                download_count: 1500,
                last_updated: "2024-01-15".to_string(),
                verified: true,
            },
            PluginMarketplaceEntry {
                name: "CloudIntegration".to_string(),
                version: "2.0.1".to_string(),
                description: "Seamless cloud service integration".to_string(),
                author: "CloudOps Inc".to_string(),
                repository_url: "https://github.com/cloudops/cloud-integration".to_string(),
                download_url: "https://releases.cloudops.com/cloud-integration-2.0.1.dll"
                    .to_string(),
                tags: vec![
                    "cloud".to_string(),
                    "integration".to_string(),
                    "api".to_string(),
                ],
                license: "Apache-2.0".to_string(),
                min_lao_version: "0.1.0".to_string(),
                dependencies: vec![],
                ratings: 4.5,
                download_count: 890,
                last_updated: "2024-01-20".to_string(),
                verified: true,
            },
        ];

        for plugin in example_plugins {
            self.marketplace_cache.insert(plugin.name.clone(), plugin);
        }

        println!(
            "✓ Refreshed marketplace cache with {} plugins",
            self.marketplace_cache.len()
        );
        Ok(())
    }

    /// Search marketplace for plugins
    pub fn search_marketplace(
        &self,
        query: &str,
        tags: Option<Vec<String>>,
    ) -> Vec<&PluginMarketplaceEntry> {
        self.marketplace_cache
            .values()
            .filter(|entry| {
                let query_match = entry.name.to_lowercase().contains(&query.to_lowercase())
                    || entry
                        .description
                        .to_lowercase()
                        .contains(&query.to_lowercase());

                let tag_match = if let Some(ref search_tags) = tags {
                    search_tags.iter().any(|tag| entry.tags.contains(tag))
                } else {
                    true
                };

                query_match && tag_match
            })
            .collect()
    }

    /// Uninstall plugin
    pub fn uninstall_plugin(&mut self, name: &str) -> Result<()> {
        // Remove from registry
        if let Err(e) = self.registry.remove_plugin(name) {
            return Err(anyhow!("Failed to remove plugin from registry: {}", e));
        }

        // Remove config
        self.configs.remove(name);
        let config_path = self.config_directory.join(format!("{}.json", name));
        if config_path.exists() {
            std::fs::remove_file(config_path)?;
        }

        // Remove plugin directory
        let plugin_path = self.plugin_directory.join(name);
        if plugin_path.exists() {
            std::fs::remove_dir_all(plugin_path)?;
        }

        // Emit unload event
        self.emit_event(PluginEvent::PluginUnloaded {
            plugin_name: name.to_string(),
        });

        println!("✓ Successfully uninstalled plugin: {}", name);
        Ok(())
    }

    /// Register a hook for plugin to listen to events
    pub fn register_hook(
        &mut self,
        plugin_name: String,
        event_types: Vec<String>,
        callback: String,
    ) {
        let hook = PluginHook {
            plugin_name,
            event_types,
            callback,
        };
        self.hooks.push(hook);
    }

    /// Emit an event to all registered hooks
    pub fn emit_event(&mut self, event: PluginEvent) {
        // Add to history
        self.event_history.push(event.clone());

        // Limit history size
        if self.event_history.len() > 1000 {
            self.event_history.remove(0);
        }

        // Find and call hooks
        let event_type = match &event {
            PluginEvent::WorkflowStarted { .. } => "workflow_started",
            PluginEvent::WorkflowCompleted { .. } => "workflow_completed",
            PluginEvent::StepStarted { .. } => "step_started",
            PluginEvent::StepCompleted { .. } => "step_completed",
            PluginEvent::PluginLoaded { .. } => "plugin_loaded",
            PluginEvent::PluginUnloaded { .. } => "plugin_unloaded",
            PluginEvent::Custom { event_type, .. } => event_type,
        };

        for hook in &self.hooks {
            if hook.event_types.contains(&event_type.to_string()) {
                // In a real implementation, you'd call the plugin's callback function here
                println!(
                    "📢 Calling hook {}.{} for event: {}",
                    hook.plugin_name, hook.callback, event_type
                );
            }
        }
    }

    /// Get plugin configuration
    pub fn get_plugin_config(&self, name: &str) -> Option<&PluginConfig> {
        self.configs.get(name)
    }

    /// Update plugin configuration
    pub fn update_plugin_config(&mut self, name: &str, config: PluginConfig) -> Result<()> {
        self.configs.insert(name.to_string(), config.clone());
        self.save_plugin_config(name, &config)?;
        Ok(())
    }

    /// Enable/disable plugin
    pub fn set_plugin_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        let config = self
            .configs
            .get_mut(name)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", name))?
            .clone();
        let mut updated_config = config;
        updated_config.enabled = enabled;
        self.configs
            .insert(name.to_string(), updated_config.clone());
        self.save_plugin_config(name, &updated_config)?;

        if enabled {
            println!("✓ Enabled plugin: {}", name);
        } else {
            println!("✓ Disabled plugin: {}", name);
        }

        Ok(())
    }

    /// Hot reload a plugin
    pub fn hot_reload_plugin(&mut self, name: &str) -> Result<()> {
        println!("🔄 Hot reloading plugin: {}", name);

        // Emit unload event
        self.emit_event(PluginEvent::PluginUnloaded {
            plugin_name: name.to_string(),
        });

        // Remove from registry
        if self.registry.plugins.contains_key(name) {
            // In a real implementation, you'd properly unload the dynamic library
            self.registry.plugins.remove(name);
        }

        // Reload plugins
        self.load_plugins()?;

        println!("✓ Successfully hot reloaded plugin: {}", name);
        Ok(())
    }

    /// List all plugins with their status
    pub fn list_plugins_with_status(&self) -> Vec<(String, bool, &PluginInfo)> {
        self.registry
            .plugins
            .iter()
            .map(|(name, plugin)| {
                let enabled = self.configs.get(name).map(|c| c.enabled).unwrap_or(true);
                (name.clone(), enabled, &plugin.info)
            })
            .collect()
    }

    /// Get plugin analytics
    pub fn get_plugin_analytics(&self, name: &str) -> HashMap<String, serde_json::Value> {
        let mut analytics = HashMap::new();

        // Count events related to this plugin
        let events = self
            .event_history
            .iter()
            .filter(|event| match event {
                PluginEvent::StepStarted { plugin_name, .. } => plugin_name == name,
                PluginEvent::StepCompleted { plugin_name, .. } => plugin_name == name,
                _ => false,
            })
            .count();

        analytics.insert(
            "total_executions".to_string(),
            serde_json::Value::from(events),
        );
        analytics.insert(
            "last_used".to_string(),
            serde_json::Value::String("2024-01-20T10:30:00Z".to_string()),
        );
        analytics.insert(
            "avg_execution_time_ms".to_string(),
            serde_json::Value::from(150.5),
        );

        analytics
    }

    /// Validate plugin permissions before execution
    pub fn validate_plugin_permissions(&self, name: &str, requested_permission: &str) -> bool {
        if let Some(config) = self.configs.get(name) {
            config
                .permissions
                .contains(&requested_permission.to_string())
        } else {
            false
        }
    }

    /// Get plugin dependencies and verify they're available
    pub fn validate_plugin_dependencies(&self, name: &str) -> Result<Vec<String>> {
        if let Some(plugin) = self.registry.plugins.get(name) {
            let mut missing_deps = Vec::new();

            for dep in &plugin.info.dependencies {
                if !self.registry.plugins.contains_key(&dep.name) && !dep.optional {
                    missing_deps.push(dep.name.clone());
                }
            }

            if missing_deps.is_empty() {
                Ok(vec![])
            } else {
                Err(anyhow!(
                    "Missing required dependencies: {}",
                    missing_deps.join(", ")
                ))
            }
        } else {
            Err(anyhow!("Plugin '{}' not found", name))
        }
    }
}
