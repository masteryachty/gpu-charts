use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Determine build profile (development or production)
    let profile = env::var("CARGO_BUILD_PROFILE").unwrap_or_else(|_| {
        if env::var("CARGO_CFG_DEBUG_ASSERTIONS").is_ok() {
            "development".to_string()
        } else {
            "production".to_string()
        }
    });

    // Read config file
    let config_path = Path::new("config.toml");
    let config_content = fs::read_to_string(config_path).expect("Failed to read config.toml");

    // Parse TOML and extract data path for current profile
    let config: toml::Value = config_content.parse().expect("Failed to parse config.toml");

    let data_path = config[&profile]["data_path"]
        .as_str()
        .unwrap_or("/mnt/md/data");

    let port = config[&profile]["port"].as_integer().unwrap_or(8443);

    // Make configuration available to the compiled binary
    println!("cargo:rustc-env=GRAPH_DATA_PATH={data_path}");
    println!("cargo:rustc-env=GRAPH_PORT={port}");
    println!("cargo:rustc-env=GRAPH_PROFILE={profile}");

    // Re-run build script if config file changes
    println!("cargo:rerun-if-changed=config.toml");
}
