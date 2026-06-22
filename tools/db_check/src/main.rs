use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Deserialize;

// ── config structs (mirror of main config) ─────────────────────────

#[derive(Deserialize)]
struct SqliteConfig {
    path: String,
}

#[derive(Deserialize)]
struct MongoConfig {
    uri: String,
    database: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    auth_source: Option<String>,
}

#[derive(Deserialize)]
struct DatabaseConfig {
    backend: String,
    sqlite: SqliteConfig,
    mongodb: MongoConfig,
}

#[derive(Deserialize)]
struct Config {
    database: DatabaseConfig,
}

// ── helpers ────────────────────────────────────────────────────────

fn check_sqlite(cfg: &SqliteConfig) -> bool {
    print!("\n  file:  {}", cfg.path);
    let start = Instant::now();

    match rusqlite::Connection::open(&cfg.path) {
        Ok(conn) => {
            let dur = start.elapsed();
            println!("  \x1b[32m✓ opened\x1b[0m  ({:.1}ms)", dur.as_secs_f64() * 1000.0);

            // run a quick sanity query
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

fn ensure_scheme(uri: &str) -> String {
    if uri.contains("://") {
        return uri.to_string();
    }
    if uri.contains("mongodb.net") || uri.contains("mongodb-dev.net") {
        format!("mongodb+srv://{}", uri)
    } else {
        format!("mongodb://{}", uri)
    }
}

fn build_uri(cfg: &MongoConfig) -> String {
    let uri = ensure_scheme(&cfg.uri);
    if uri.contains('@') {
        return uri;
    }
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) {
        if !u.is_empty() && !p.is_empty() {
            let scheme_end = uri.find("://").map(|i| i + 3).unwrap_or(0);
            let host = &uri[scheme_end..];
            let scheme = &uri[..scheme_end];
            let encoded_user = url_encode(u);
            let encoded_pass = url_encode(p);
            if let Some(src) = &cfg.auth_source {
                if !src.is_empty() {
                    return format!("{}{}:{}@{}/{}?authSource={}", scheme, encoded_user, encoded_pass, host, cfg.database, src);
                }
            }
            return format!("{}{}:{}@{}/{}", scheme, encoded_user, encoded_pass, host, cfg.database);
        }
    }
    let trimmed = uri.trim_end_matches('/');
    if trimmed.contains('/') {
        trimmed.to_string()
    } else {
        format!("{}/{}", trimmed, cfg.database)
    }
}

fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            '@' => "%40".to_string(),
            '#' => "%23".to_string(),
            '?' => "%3F".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            ' ' => "%20".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

fn redacted_uri(uri: &str) -> String {
    let uri = ensure_scheme(uri);
    if let Some(at) = uri.find('@') {
        let scheme_end = uri.find("://").map(|i| i + 3).unwrap_or(0);
        format!("{}<credentials>@{}", &uri[..scheme_end], &uri[at + 1..])
    } else {
        uri
    }
}

/// Extract host:port from a mongodb connection URI for TCP pre-check.
fn extract_hostport(uri: &str) -> Option<String> {
    let after_scheme = if let Some(pos) = uri.find("://") {
        &uri[pos + 3..]
    } else {
        uri
    };
    // strip credentials user:pass@
    let after_auth = if let Some(pos) = after_scheme.find('@') {
        &after_scheme[pos + 1..]
    } else {
        after_scheme
    };
    // take up to '/' or '?' or end
    let host_part = after_auth
        .split(|c| c == '/' || c == '?')
        .next()
        .unwrap_or(after_auth);
    if host_part.is_empty() { None } else { Some(host_part.to_string()) }
}

fn check_mongodb(cfg: &MongoConfig) -> bool {
    let raw_uri = build_uri(cfg);
    // Add a connect timeout so the tool doesn't hang forever.
    let timeout_ms = 5000;
    let uri = if raw_uri.contains('?') {
        format!("{}", raw_uri)
    } else {
        format!("{}", raw_uri)
    };

    println!("\n  uri:   {}", redacted_uri(&uri));
    let start = Instant::now();

    // ── DNS / TCP pre‑check ────────────────────────────────────
    if let Some(hostport) = extract_hostport(&uri) {
        print!("  dns/tcp: resolving {} ... ", hostport);
        let _ = std::io::stdout().flush();
        match format!("{}:27017", hostport.trim_end_matches(":27017")).to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    println!("\x1b[32m{} ✓\x1b[0m", addr.ip());
                    // TCP connect pre-check
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

    // ── MongoDB client ─────────────────────────────────────────
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

    // ── Ping ───────────────────────────────────────────────────
    let db = client.database(&cfg.database);
    match db.run_command(mongodb::bson::doc! { "ping": 1 }, None) {
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

// ── main ───────────────────────────────────────────────────────────

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
