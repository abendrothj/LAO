use clap::{Parser, Subcommand};
use lao_orchestrator_core::cross_platform::PathUtils;
use lao_orchestrator_core::plugins::PluginRegistry;
use lao_orchestrator_core::{load_workflow_yaml, run_workflow_yaml};

#[derive(Parser)]
#[command(name = "lao")]
#[command(about = "Local AI Orchestrator CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a workflow YAML file
    Run {
        path: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate a workflow YAML file (type & plugin availability)
    Validate { path: String },
    /// List available plugins
    PluginList,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run { path, dry_run } => {
            if dry_run {
                match load_workflow_yaml(&path) {
                    Ok(workflow) => {
                        let plugin_dir = PathUtils::plugin_dir();
                        let plugin_registry = PluginRegistry::dynamic_registry(
                            plugin_dir.to_str().unwrap_or("plugins"),
                        );
                        println!("[DRY RUN] Workflow: {}", workflow.workflow);
                        for (i, step) in workflow.steps.iter().enumerate() {
                            let plugin = plugin_registry.get(&step.run);
                            println!("Step {}: {}", i + 1, step.run);
                            match plugin {
                                Some(_p) => {
                                    println!("  [OK] Plugin '{}' loaded.", step.run);
                                }
                                None => {
                                    println!("  [ERROR] Plugin '{}' not found!", step.run);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[DRY RUN] Failed to load workflow: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                match run_workflow_yaml(&path) {
                    Ok(results) => {
                        println!("Workflow executed successfully. Step outputs:");
                        for (i, output) in results.iter().enumerate() {
                            println!("Step {}: {:?}", i + 1, output);
                        }
                    }
                    Err(e) => {
                        eprintln!("Workflow execution failed: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Validate { path } => match load_workflow_yaml(&path) {
            Ok(workflow) => {
                let plugin_dir = PathUtils::plugin_dir();
                let plugin_registry =
                    PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));
                let dag = lao_orchestrator_core::build_dag(&workflow.steps).unwrap();
                let errors = lao_orchestrator_core::validate_workflow_types(&dag, &plugin_registry);
                if errors.is_empty() {
                    println!("Validation passed: all steps and plugins available.");
                } else {
                    for (step, msg) in errors {
                        println!("Step {step}: {msg}");
                    }
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Failed to load workflow: {e}");
                std::process::exit(1);
            }
        },
        Commands::PluginList => {
            let plugin_dir = PathUtils::plugin_dir();
            let plugin_registry =
                PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));
            println!("Available plugins:");
            for name in plugin_registry.plugins.keys() {
                println!("- {name}");
            }
        }
    }
}
