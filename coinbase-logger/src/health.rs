use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::fs;
use warp::{http::StatusCode, Filter, Rejection, Reply};

const HEALTH_CHECK_PORT: u16 = 8080;
const MAX_FILE_AGE_SECONDS: u64 = 60; // Files should be updated within 60 seconds

pub async fn start_health_server() {
    let health_route = warp::path("health")
        .and_then(health_check)
        .with(warp::cors().allow_any_origin());

    println!("Starting health check server on port {}", HEALTH_CHECK_PORT);

    warp::serve(health_route)
        .run(([0, 0, 0, 0], HEALTH_CHECK_PORT))
        .await;
}

async fn health_check() -> Result<impl Reply, Rejection> {
    // Check if any data files have been written to recently
    let (message, status) = match check_recent_file_writes().await {
        Ok(true) => (
            "OK: Data files are being written".to_string(),
            StatusCode::OK,
        ),
        Ok(false) => (
            "UNHEALTHY: No recent file writes detected".to_string(),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        Err(e) => {
            eprintln!("Health check error: {}", e);
            (format!("ERROR: {}", e), StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    Ok(warp::reply::with_status(message, status))
}

async fn check_recent_file_writes() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let data_dir = "/mnt/md/data";
    let now = SystemTime::now();
    let max_age = Duration::from_secs(MAX_FILE_AGE_SECONDS);

    // Check if the data directory exists
    if !Path::new(data_dir).exists() {
        return Ok(false);
    }

    // Read the data directory
    let mut entries = fs::read_dir(data_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Only check directories (symbol directories)
        if path.is_dir() {
            let md_path = path.join("MD");

            if md_path.exists() {
                // Check for recent .bin files in the MD directory
                let mut md_entries = fs::read_dir(&md_path).await?;

                while let Some(md_entry) = md_entries.next_entry().await? {
                    let file_path = md_entry.path();

                    // Check if it's a .bin file
                    if file_path.extension().and_then(|s| s.to_str()) == Some("bin") {
                        // Get file metadata
                        let metadata = fs::metadata(&file_path).await?;

                        // Check last modified time
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                if age <= max_age {
                                    // Found a recently modified file
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // No recent file writes found
    Ok(false)
}
