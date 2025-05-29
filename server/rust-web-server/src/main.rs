#![feature(portable_simd)]
use axum::{body, extract::Path, http::StatusCode, response::IntoResponse, routing::get, Router};
use file::serve_file;
use futures::Stream;
use hyper::server::conn::Http;
use hyper::{Body, Request, Response};
use memmap2::{Mmap, MmapOptions};
use rustls::{Certificate, PrivateKey, ServerConfig, ServerConnection};
use std::time::Instant;
use std::{fs::File, io, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tokio::signal;
use tokio_rustls::{TlsAcceptor, TlsStream};
use tokio_stream::StreamExt;
use tower::Service;
use tower_http::services::ServeDir;

use std::io::BufReader;
mod file;

fn load_certs(path: &str) -> Vec<Certificate> {
    let certfile = File::open(path).expect("cannot open certificate file {:?}",path);
    let mut reader = BufReader::new(certfile);

    rustls_pemfile::certs(&mut reader)
        .expect("cannot read certificate file")
        .into_iter()
        .map(Certificate)
        .collect()
}

fn load_private_key(path: &str) -> PrivateKey {
    let keyfile = File::open(path).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);

    if let Ok(mut keys) = rustls_pemfile::pkcs8_private_keys(&mut reader) {
        if !keys.is_empty() {
            return PrivateKey(keys.remove(0));
        }
    }

    if let Ok(mut keys) = rustls_pemfile::rsa_private_keys(&mut reader) {
        if !keys.is_empty() {
            return PrivateKey(keys.remove(0));
        }
    }

    panic!("no keys found in {path}");
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .route("/data/:filename", get(serve_file))
        .nest_service(
            "/",
            ServeDir::new("../../").append_index_html_on_directories(true),
        );

    let make_svc = router.into_make_service();

    let addr: SocketAddr = "127.0.0.1:3001".parse().unwrap();
    println!("Listening on https://{addr} (HTTP/2)");

    let certs = load_certs("/home/xander/projects/md-server/localhost.crt");
    let key = load_private_key("localhost.key");

    let mut tls_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("bad certificate/private key?");

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let listener = TcpListener::bind(addr).await?;
    println!("Server running. Press Ctrl+C to stop.");

    tokio::spawn(async {
        shutdown_signal().await;
        println!("Shutdown signal received.");
        std::process::exit(0);
    });

    loop {
        let (socket, remote_addr) = listener.accept().await?;
        println!("Accepted connection from: {remote_addr}");

        let acceptor = tls_acceptor.clone();
        let mut make_svc = make_svc.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(socket).await {
                Ok(stream) => stream,
                Err(err) => {
                    eprintln!("TLS error: {err:?}");
                    return;
                }
            };

            let mut http = Http::new();
            http.http2_only(false);

            let svc = match make_svc.call(&()).await {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("MakeService error: {err:?}");
                    return;
                }
            };

            if let Err(err) = http.serve_connection(tls_stream, svc).await {
                eprintln!("Error serving connection: {err:?}");
            }
        });
    }
}

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}
