// main.rs

use std::{convert::Infallible, fs::File, io::BufReader, net::SocketAddr, sync::Arc};

use hyper::{
    body::Body, server::conn::Http, service::service_fn, Method, Request, Response, StatusCode,
};
use rustls::{Certificate, PrivateKey, ServerConfig};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

mod data;
mod metrics_server;
mod status;
mod symbols;

use data::handle_data_request;
use status::handle_status_request;
use symbols::{handle_symbols_request, handle_symbol_search_request, initialize_search_service};

/// Middleware to track HTTP metrics
struct MetricsMiddleware {
    method: String,
    endpoint: String,
    start: std::time::Instant,
}

impl MetricsMiddleware {
    fn new(method: &str, endpoint: &str) -> Self {
        Self {
            method: method.to_string(),
            endpoint: endpoint.to_string(),
            start: std::time::Instant::now(),
        }
    }

    fn complete(self, status: u16) {
        let duration = self.start.elapsed().as_secs_f64();
        metrics_server::record_http_request(&self.method, &self.endpoint, status, duration);
    }
}

/// Our top–level service function. It dispatches GET requests on "/api/data" to our handler.
async fn service_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let metrics_middleware = MetricsMiddleware::new(&method, &path);

    // Handle preflight OPTIONS requests.
    if req.method() == Method::OPTIONS {
        let response = Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type")
            .body(Body::empty())
            .unwrap();
        metrics_middleware.complete(200);
        return Ok(response);
    }

    // For GET requests on /api/data, call our handler.
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/api/data") => {
            let mut response = handle_data_request(req).await?;
            // Attach CORS header to the response.
            response
                .headers_mut()
                .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
            response
        }
        (&Method::GET, "/api/symbols") => {
            let mut response = handle_symbols_request(req).await?;
            response
                .headers_mut()
                .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
            response
        }
        (&Method::GET, "/api/status") => {
            let mut response = handle_status_request(req).await?;
            response
                .headers_mut()
                .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
            response
        }
        (&Method::GET, "/api/symbol-search") => {
            let mut response = handle_symbol_search_request(req).await?;
            response
                .headers_mut()
                .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
            response
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::from("Not Found"))
            .unwrap(),
    };
    
    // Record metrics
    let status = response.status().as_u16();
    metrics_middleware.complete(status);
    
    Ok(response)
}

/// Load the TLS certificate and private key from files.
fn load_tls_config() -> Result<ServerConfig, Box<dyn std::error::Error>> {
    // Get certificate paths from environment or use defaults
    let cert_path = std::env::var("SSL_CERT_PATH").unwrap_or_else(|_| "localhost.crt".to_string());
    let key_path =
        std::env::var("SSL_PRIVATE_FILE").unwrap_or_else(|_| "localhost.key".to_string());

    let certs = load_certs(&cert_path)?;
    let key = load_private_key(&key_path)?;
    let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    // Advertise both HTTP/2 and HTTP/1.1.
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

/// Load certificates from a PEM file.
fn load_certs(path: &str) -> Result<Vec<Certificate>, Box<dyn std::error::Error>> {
    let certfile = File::open(path)?;
    let mut reader = BufReader::new(certfile);
    let certs = rustls_pemfile::certs(&mut reader)?
        .into_iter()
        .map(Certificate)
        .collect();
    Ok(certs)
}

/// Load a private key from a PEM file.
fn load_private_key(path: &str) -> Result<PrivateKey, Box<dyn std::error::Error>> {
    let keyfile = File::open(path)?;
    let mut reader = BufReader::new(keyfile);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;
    if keys.is_empty() {
        return Err("No private keys found".into());
    }
    Ok(PrivateKey(keys[0].clone()))
}

//
// ----- Main: Set up TLS, bind a TCP listener, and serve connections -----
//

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load symbol registry at startup
    symbols::load_symbols_at_startup().await?;
    
    // Initialize the symbol search service
    symbols::initialize_search_service().await?;
    
    // Start metrics server on separate port
    let metrics_port = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9091".to_string())
        .parse::<u16>()
        .unwrap_or(9091);
    
    tokio::spawn(async move {
        metrics_server::start_metrics_server(metrics_port).await;
    });
    
    println!("Metrics server started on port {}", metrics_port);
    println!("Prometheus can scrape metrics from http://localhost:{}/metrics", metrics_port);
    
    // Check if we should use TLS or plain HTTP
    let use_tls = std::env::var("USE_TLS")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true);

    // Bind a TCP listener (default to 8443 for both modes)
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8443".to_string())
        .parse::<u16>()
        .unwrap_or(8443);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(&addr).await?;

    if use_tls {
        println!("Listening on https://{addr} (TLS enabled)");

        // Load TLS configuration
        let tls_config = load_tls_config()?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

        loop {
            let (stream, _peer_addr) = listener.accept().await?;
            stream.set_nodelay(true).ok();

            let acceptor = tls_acceptor.clone();
            tokio::spawn(async move {
                let tls_stream = match acceptor.accept(stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("TLS accept error: {e:?}");
                        return;
                    }
                };
                if let Err(e) = Http::new()
                    .http2_only(true)
                    .serve_connection(tls_stream, service_fn(service_handler))
                    .await
                {
                    eprintln!("Error serving connection: {e:?}");
                }
            });
        }
    } else {
        println!("Listening on http://{addr} (HTTP/1.1 for Cloudflare Tunnel)");

        loop {
            let (stream, _peer_addr) = listener.accept().await?;
            stream.set_nodelay(true).ok();

            tokio::spawn(async move {
                // Serve HTTP/1.1 only for Cloudflare Tunnel
                if let Err(e) = Http::new()
                    .http1_only(true)
                    .serve_connection(stream, service_fn(service_handler))
                    .await
                {
                    eprintln!("Error serving connection: {e:?}");
                }
            });
        }
    }
}