use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Deserialize;

use keymouse_monitor::config::DatabaseConfig;

#[derive(Deserialize)]
struct Config {
    database: DatabaseConfig,
}

fn check_sqlite(cfg: &keymouse_monitor::config::SqliteConfig) -> bool {
    print!("\n  file:  {}", cfg.path);
    let start = Instant::now();

    match rusqlite::Connection::open(&cfg.path) {
        Ok(conn) => {
            let dur = start.elapsed();
            println!("  \x1b[32m✓ opened\x1b[0m  ({:.1}ms)", dur.as_secs_f64() * 1000.0);
            match conn.query_row("SELECT 1", [], |_| Ok(())) {
                Ok(_) => {
                    let dur = start.elapsed();
                    println!("  query: \x1b[32m✓ SELECT 1\x1b[0m  ({:.1}ms)", dur.as_secs_f64() * 1000.0);
                    true
                }
                Err(e) => {
                    println!("  query: \x1b[31m✗ failed\x1b[0m  {e}");
                    false
                }
            }
        }
        Err(e) => {
            let dur = start.elapsed();
            println!("  \x1b[31m✗ failed\x1b[0m  ({:.1}ms)  {e}", dur.as_secs_f64() * 1000.0);
            false
        }
    }
}

fn redacted_uri(uri: &str) -> String {
    keymouse_monitor::database::redact_credentials(uri)
}

fn extract_hostport(uri: &str) -> Option<String> {
    let after_scheme = if let Some(pos) = uri.find("://") {
        &uri[pos + 3..]
    } else {
        uri
    };
    let after_auth = if let Some(pos) = after_scheme.find('@') {
        &after_scheme[pos + 1..]
    } else {
        after_scheme
    };
    let host_part = after_auth
        .split(|c| c == '/' || c == '?')
        .next()
        .unwrap_or(after_auth);
    if host_part.is_empty() { None } else { Some(host_part.to_string()) }
}

fn check_mongodb(cfg: &keymouse_monitor::config::MongoConfig) -> bool {
    let uri = keymouse_monitor::database::build_uri(cfg);

    println!("\n  uri:   {}", redacted_uri(&uri));
    let start = Instant::now();

    if let Some(hostport) = extract_hostport(&uri) {
        print!("  dns/tcp: resolving {} ... ", hostport);
        let _ = std::io::stdout().flush();
        match format!("{}:27017", hostport.trim_end_matches(":27017")).to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    println!("\x1b[32m{} ✓\x1b[0m", addr.ip());
                    print!("  tcp:    connecting ... ");
                    let _ = std::io::stdout().flush();
                    match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
                        Ok(_) => println!("\x1b[32m✓\x1b[0m"),
                        Err(e) => println!("\x1b[33m⚠ {}\x1b[0m (atlas may still work via SRV)", e),
                    }
                } else {
                    println!("\x1b[31m✗ no addresses\x1b[0m");
                }
            }
            Err(e) => {
                println!("\x1b[33m⚠ {e}\x1b[0m (atlas SRV records may still resolve within driver)");
            }
        }
    }

    let client = match mongodb::sync::Client::with_uri_str(&uri) {
        Ok(c) => {
            let dur = start.elapsed();
            println!("  client: \x1b[32m✓ created\x1b[0m  ({:.1}ms)", dur.as_secs_f64() * 1000.0);
            c
        }
        Err(e) => {
            let dur = start.elapsed();
            println!("  client: \x1b[31m✗ failed\x1b[0m  ({:.1}ms)  {e}", dur.as_secs_f64() * 1000.0);
            return false;
        }
    };

    let db = client.database(&cfg.database);
    match db.run_command(mongodb::bson::doc! { "ping": 1 }).run() {
        Ok(_) => {
            let dur = start.elapsed();
            println!("  ping:   \x1b[32m✓ success\x1b[0m  ({:.1}ms)", dur.as_secs_f64() * 1000.0);
            true
        }
        Err(e) => {
            let dur = start.elapsed();
            println!("  ping:   \x1b[31m✗ failed\x1b[0m  ({:.1}ms)  {e}", dur.as_secs_f64() * 1000.0);
            false
        }
    }
}

fn main() {
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.json".to_string());
    let path = Path::new(&config_path);

    if !path.exists() {
        eprintln!("\x1b[31m✗ config file not found: {}\x1b[0m", config_path);
        std::process::exit(1);
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31m✗ failed to read config: {e}\x1b[0m");
            std::process::exit(1);
        }
    };

    let cfg: Config = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31m✗ failed to parse config: {e}\x1b[0m");
            std::process::exit(1);
        }
    };

    println!("database connection check");
    println!("{}", "-".repeat(48));
    println!("  backend: {}", cfg.database.backend);

    let ok = match cfg.database.backend.to_lowercase().as_str() {
        "sqlite" => check_sqlite(&cfg.database.sqlite),
        "mongodb" | "mongo" => check_mongodb(&cfg.database.mongodb),
        other => {
            eprintln!("\x1b[31m✗ unknown backend: {other}\x1b[0m");
            false
        }
    };

    println!("{}", "-".repeat(48));
    if ok {
        println!(" \x1b[32m✓ connection OK\x1b[0m");
        std::process::exit(0);
    } else {
        println!(" \x1b[31m✗ connection FAILED\x1b[0m");
        std::process::exit(1);
    }
}
