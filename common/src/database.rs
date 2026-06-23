use crate::config::MongoConfig;

pub fn build_uri(cfg: &MongoConfig) -> String {
    let mut result = format!("{}://", cfg.protocol);

    if let (Some(username), Some(password)) = (&cfg.username, &cfg.password) {
        if !username.is_empty() && !password.is_empty() {
            let encoded_user = url_encode(username);
            let encoded_pass = url_encode(password);
            result.push_str(&format!("{}:{}@", encoded_user, encoded_pass));
        }
    }

    if let Some(hosts) = &cfg.hosts {
        if !hosts.is_empty() {
            result.push_str(&hosts.join(","));
        }
    }

    let db = &cfg.database;
    if !db.is_empty() {
        result.push_str(&format!("/{}", db));
    }

    let mut params = Vec::new();

    if cfg.ssl {
        params.push("ssl=true".to_string());
    }

    if let Some(replica_set) = &cfg.replica_set {
        if !replica_set.is_empty() {
            params.push(format!("replicaSet={}", replica_set));
        }
    }

    if !cfg.auth_source.is_empty() {
        params.push(format!("authSource={}", cfg.auth_source));
    }

    if let Some(app_name) = &cfg.app_name {
        if !app_name.is_empty() {
            params.push(format!("appName={}", app_name));
        }
    }

    if !params.is_empty() {
        result.push_str(&format!("?{}", params.join("&")));
    }

    if !result.contains("connectTimeoutMS") {
        let sep = if result.contains('?') { "&" } else { "?" };
        result.push_str(&format!("{}connectTimeoutMS={}", sep, cfg.connect_timeout_ms));
    }
    if !result.contains("serverSelectionTimeoutMS") {
        let sep = if result.contains('?') { "&" } else { "?" };
        result.push_str(&format!("{}serverSelectionTimeoutMS={}", sep, cfg.server_selection_timeout_ms));
    }

    result
}

pub fn redact_credentials(uri: &str) -> String {
    if let Some(at) = uri.find('@') {
        let scheme_end = uri.find("://").map(|i| i + 3).unwrap_or(0);
        format!("{}<credentials>@{}", &uri[..scheme_end], &uri[at + 1..])
    } else {
        uri.to_string()
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
