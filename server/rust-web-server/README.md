# Rust Web Server

This project is a high-performance web server built in Rust that serves data from a CSV file to web browsers. It utilizes the Actix web framework for handling HTTP requests and responses.

## Project Structure

```
rust-web-server
├── src
│   ├── main.rs          # Entry point of the application
│   ├── handlers         # Contains request handler functions
│   │   └── mod.rs
│   ├── models           # Defines data structures
│   │   └── mod.rs
│   ├── routes           # Sets up application routes
│   │   └── mod.rs
│   └── utils            # Utility functions for CSV handling
│       └── mod.rs
├── data
│   └── data.csv        # CSV data file
├── Cargo.toml          # Project configuration file
└── README.md           # Project documentation
```

## Setup Instructions

1. **Clone the repository:**
   ```
   git clone <repository-url>
   cd rust-web-server
   ```

2. **Install Rust:**
   Ensure you have Rust installed on your machine. You can install it from [rust-lang.org](https://www.rust-lang.org/).

3. **Build the project:**
   ```
   cargo build
   ```

4. **Run the server:**
   ```
   cargo run
   ```

5. **Access the server:**
   Open your web browser and navigate to `http://localhost:8000` to view the served data.

## Usage

The web server provides endpoints to access the data from the CSV file. You can extend the functionality by adding more routes and handlers as needed.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.