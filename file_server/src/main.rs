use actix_files as fs;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // The folder you want to serve (e.g. `./public`)
    let static_folder = "./..";

    // Bind to localhost at port 8080 (adjust to your needs)
    HttpServer::new(move || {
        App::new()
            // Serve the static files at the root URL path (`/`)
            .service(fs::Files::new("/", static_folder).index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
