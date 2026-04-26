use anyhow::Result;
use clap::{Parser, Subcommand};
use env_parse::env_parse;
use gsm_cron::register_job;
use gsm_instance::Instance;
use gsm_instance::config::{InstanceConfig, LaunchMode};
use gsm_instance::update::update_server;
use gsm_monitor::{LogRules, start_instance_log_monitor};
use gsm_windrose::json_config;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use std::fs; // Added for file system operations
use serde_json::Value; // Added for JSON parsing

fn get_instance_config() -> InstanceConfig {
    InstanceConfig {
        app_id: env_parse!("APP_ID", 4129620_u32, u32),
        working_dir: PathBuf::from(env_parse!(
            "INSTALL_PATH",
            "/home/steam/windrose".to_string(),
            String
        )),
        command: "/home/steam/windrose/R5/Binaries/Win64/WindroseServer-Win64-Shipping.exe"
            .to_string(),
        name: env_parse!("NAME", "Windrose Dedicated Server".to_string(), String),
        launch_mode: LaunchMode::Proton,
        force_windows: true,
        launch_args: env_parse!("WINDROSE_LAUNCH_ARGS", "".to_string(), String)
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        install_args: vec![format!(
            "+force_install_dir {}",
            env_parse!("INSTALL_PATH", "/home/steam/windrose".to_string(), String)
        )],
        ..Default::default()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO) // Default to INFO level
        .with_env_filter(EnvFilter::from_default_env()) // Allow RUST_LOG to override
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();

    match &cli.command {
        Commands::Install => {
            info!("Executing Install command...");
            let config = get_instance_config();
            let instance = Instance::new(config);
            instance.install()?;
            info!("Install command finished successfully.");
            Ok(())
        }
        Commands::Start => {
            info!("Executing Start command...");
            let config = get_instance_config();
            info!("Configuration loaded: {:?}", config);

            // Define the mapping from environment variables to JSON keys
            let mut mapping_table = HashMap::new();
            mapping_table.insert(
                "WINDROSE_SERVER_NAME".to_string(),
                "ServerDescription_Persistent.ServerName".to_string(),
            );
            mapping_table.insert(
                "WINDROSE_MAX_PLAYERS".to_string(),
                "ServerDescription_Persistent.MaxPlayerCount".to_string(),
            );
            mapping_table.insert(
                "WINDROSE_REGION".to_string(),
                "ServerDescription_Persistent.UserSelectedRegion".to_string(),
            );
            mapping_table.insert(
                "WINDROSE_DIRECT_CONNECTION".to_string(),
                "ServerDescription_Persistent.UseDirectConnection".to_string(),
            );
            mapping_table.insert(
                "WINDROSE_DIRECT_PORT".to_string(),
                "ServerDescription_Persistent.DirectConnectionServerPort".to_string(),
            );
            // Add password related environment variables
            mapping_table.insert(
                "WINDROSE_PASSWORD".to_string(),
                "ServerDescription_Persistent.Password".to_string(),
            );

            let mut env_vars: HashMap<String, String> = env::vars().collect();
            
            // Determine IsPasswordProtected based on WINDROSE_PASSWORD
            let is_password_protected = env_vars.get("WINDROSE_PASSWORD")
                                                .map_or(false, |p| !p.is_empty());
            env_vars.insert("WINDROSE_IS_PASSWORD_PROTECTED".to_string(), is_password_protected.to_string());
            mapping_table.insert(
                "WINDROSE_IS_PASSWORD_PROTECTED".to_string(),
                "ServerDescription_Persistent.IsPasswordProtected".to_string(),
            );


            let json_file_path = config.working_dir.join("ServerDescription.json");

            info!("Applying environment variables to JSON config...");
            json_config::apply_env_to_json(&env_vars, &json_file_path, &mapping_table)?;
            info!("JSON config updated successfully.");

            let instance = Instance::new(config);
            instance.start()?;
            info!("Start command finished successfully.");
            Ok(())
        }
        Commands::Stop => {
            info!("Executing Stop command...");
            let config = get_instance_config();
            let instance = Instance::new(config);
            instance.stop()?;
            info!("Stop command finished successfully.");
            Ok(())
        }
        Commands::Restart => {
            info!("Executing Restart command...");
            let config = get_instance_config();
            let instance = Instance::new(config);
            instance.stop()?;
            info!("Server stopped, restarting in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
            instance.start()?;
            info!("Restart command finished successfully.");
            Ok(())
        }
        Commands::Update => {
            info!("Executing Update command...");
            let config = get_instance_config();
            update_server(
                config.app_id,
                &config.working_dir,
                config.force_windows,
                &config.install_args,
            )?;
            info!("Update command finished successfully.");
            Ok(())
        }
        Commands::Monitor => {
            info!("Executing Monitor command...");
            let config = get_instance_config();

            // Construct the path to ServerDescription.json
            // This is relative to the working_dir (INSTALL_PATH), which is /home/steam/windrose
            // And the actual file is at /home/steam/windrose/R5/ServerDescription.json
            // So we need to join R5/ServerDescription.json to working_dir
            let json_file_path = config.working_dir.join("R5").join("ServerDescription.json");

            // Read and parse ServerDescription.json to extract the InviteCode
            match fs::read_to_string(&json_file_path) {
                Ok(content) => {
                    match serde_json::from_str::<Value>(&content) {
                        Ok(json_value) => {
                            info!("--- Server Metadata ---");

                            if let Some(deployment_id) = json_value["DeploymentId"].as_str() {
                                info!("Deployment ID: {}", deployment_id);
                            }
                            if let Some(server_desc_persistent) = json_value["ServerDescription_Persistent"].as_object() {
                                if let Some(persistent_server_id) = server_desc_persistent["PersistentServerId"].as_str() {
                                    info!("Persistent Server ID: {}", persistent_server_id);
                                }
                                if let Some(invite_code) = server_desc_persistent["InviteCode"].as_str() {
                                    info!("Server Join Code: {}", invite_code);
                                }
                                if let Some(is_password_protected) = server_desc_persistent["IsPasswordProtected"].as_bool() {
                                    info!("Password Protected: {}", is_password_protected);
                                }
                                if let Some(server_name) = server_desc_persistent["ServerName"].as_str() {
                                    if !server_name.is_empty() {
                                        info!("Server Name: {}", server_name);
                                    }
                                }
                                if let Some(max_players) = server_desc_persistent["MaxPlayerCount"].as_i64() {
                                    info!("Max Players: {}", max_players);
                                }
                                if let Some(region) = server_desc_persistent["UserSelectedRegion"].as_str() {
                                    if !region.is_empty() {
                                        info!("Region: {}", region);
                                    }
                                }
                                if let Some(use_direct_connection) = server_desc_persistent["UseDirectConnection"].as_bool() {
                                    if use_direct_connection {
                                        let address = server_desc_persistent["DirectConnectionServerAddress"].as_str().unwrap_or("N/A");
                                        let port = server_desc_persistent["DirectConnectionServerPort"].as_i64().unwrap_or(-1);
                                        info!("Direct Connection: Enabled ({}:{})", address, port);
                                    } else {
                                        info!("Direct Connection: Disabled");
                                    }
                                }
                            }
                            info!("-----------------------");
                        },
                        Err(e) => {
                            info!("Failed to parse ServerDescription.json (path: {}): {}", json_file_path.display(), e);
                        }
                    }
                },
                Err(e) => {
                    info!("Failed to read ServerDescription.json (path: {}): {}", json_file_path.display(), e);
                }
            }

            // Ensure log directory exists
            let log_dir = config.working_dir.join("logs");
            if !log_dir.exists() {
                info!("Creating log directory: {}", log_dir.display());
                fs::create_dir_all(&log_dir)?;
            }

            let rules = LogRules::new();
            start_instance_log_monitor(config.working_dir.clone(), rules);

            if env_parse!("AUTO_UPDATE", false, bool) {
                let cron_schedule = env_parse!("AUTO_UPDATE_CRON", "0 0 * * *".to_string(), String);
                let update_config = config.clone();
                register_job("Auto-update", &cron_schedule, move || {
                    info!("Executing scheduled update...");
                    match update_server(
                        update_config.app_id,
                        &update_config.working_dir,
                        update_config.force_windows,
                        &update_config.install_args,
                    ) {
                        Ok(_) => info!("Scheduled update completed successfully."),
                        Err(e) => info!("Scheduled update failed: {}", e),
                    }
                });
            }

            info!("Monitor is running. Press Ctrl+C to exit.");
            loop {
                sleep(Duration::from_secs(60)).await;
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Installs the Windrose game server
    Install,
    /// Starts the Windrose game server
    Start,
    /// Stops the Windrose game server
    Stop,
    /// Restarts the Windrose game server
    Restart,
    /// Updates the Windrose game server
    Update,
    /// Monitors the Windrose game server process and logs
    Monitor,
}
