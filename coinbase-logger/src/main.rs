use coinbase_logger::{connection::ConnectionHandler, health::start_health_server, websocket::get_all_products};

type Error = Box<dyn std::error::Error + Send + Sync>;

const CONNECTIONS_COUNT: usize = 10;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Error> {
    println!("Starting coinbase-logger...");
    
    // Start health check server in background
    tokio::spawn(async {
        start_health_server().await;
    });

    // Fetch all available products
    println!("Calling get_all_products...");
    let products = match get_all_products().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error fetching products: {e}");
            return Err(e);
        }
    };
    println!("Found {} products", products.len());

    // Calculate symbols per connection
    let symbols_per_connection = products.len().div_ceil(CONNECTIONS_COUNT);

    // Create connection handlers
    let mut tasks = vec![];

    for i in 0..CONNECTIONS_COUNT {
        let start_idx = i * symbols_per_connection;
        let end_idx = std::cmp::min((i + 1) * symbols_per_connection, products.len());

        if start_idx >= products.len() {
            break;
        }

        let connection_symbols = products[start_idx..end_idx].to_vec();

        println!(
            "Connection {}: Handling {} symbols",
            i,
            connection_symbols.len()
        );

        let task = tokio::spawn(async move {
            let mut handler = match ConnectionHandler::new(i, connection_symbols).await {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Failed to create connection handler {i}: {e}");
                    return;
                }
            };

            handler.run().await;
        });

        tasks.push(task);

        // No rate limiting - launch connections concurrently
    }

    // Wait for all tasks (they run forever)
    for task in tasks {
        let _ = task.await;
    }

    Ok(())
}
