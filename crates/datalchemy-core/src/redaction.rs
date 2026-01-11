use serde::{Deserialize, Serialize};

/// Connection metadata with secrets redacted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedConnection {
    pub engine: Option<String>,
    pub user: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub redacted: String,
}

/// Redact secrets from a connection string while extracting non-sensitive metadata.
pub fn redact_connection_string(conn: &str) -> RedactedConnection {
    let mut redacted = conn.to_string();
    let mut engine = None;
    let mut user = None;
    let mut host = None;
    let mut port = None;
    let mut database = None;

    if let Some(scheme_end) = conn.find("://") {
        engine = Some(conn[..scheme_end].to_string());
        let after_scheme = &conn[scheme_end + 3..];
        let (auth_part, host_part) = if let Some(at_idx) = after_scheme.find('@') {
            (&after_scheme[..at_idx], &after_scheme[at_idx + 1..])
        } else {
            ("", after_scheme)
        };

        if !auth_part.is_empty() {
            if let Some(colon_idx) = auth_part.find(':') {
                user = Some(auth_part[..colon_idx].to_string());
                let password_start = scheme_end + 3 + colon_idx + 1;
                let password_end = scheme_end + 3 + auth_part.len();
                if password_start <= redacted.len() && password_end <= redacted.len() {
                    redacted.replace_range(password_start..password_end, "***");
                }
            } else {
                user = Some(auth_part.to_string());
            }
        }

        let host_part = host_part
            .splitn(2, '?')
            .next()
            .unwrap_or("");
        let (host_port, path) = if let Some(slash_idx) = host_part.find('/') {
            (&host_part[..slash_idx], &host_part[slash_idx + 1..])
        } else {
            (host_part, "")
        };

        if !host_port.is_empty() {
            if let Some(colon_idx) = host_port.rfind(':') {
                host = Some(host_port[..colon_idx].to_string());
                if let Ok(parsed) = host_port[colon_idx + 1..].parse::<u16>() {
                    port = Some(parsed);
                }
            } else {
                host = Some(host_port.to_string());
            }
        }

        if !path.is_empty() {
            let db_name = path.split('?').next().unwrap_or("");
            if !db_name.is_empty() {
                database = Some(db_name.to_string());
            }
        }
    }

    redacted = redact_query_params(&redacted);

    RedactedConnection {
        engine,
        user,
        host,
        port,
        database,
        redacted,
    }
}

fn redact_query_params(conn: &str) -> String {
    let Some(query_start) = conn.find('?') else {
        return conn.to_string();
    };

    let (base, query) = conn.split_at(query_start + 1);
    let mut redacted_params = Vec::new();

    for pair in query.split('&') {
        let mut iter = pair.splitn(2, '=');
        let key = iter.next().unwrap_or("");
        let value = iter.next().unwrap_or("");
        if is_sensitive_key(key) {
            redacted_params.push(format!("{key}=***"));
        } else if value.is_empty() {
            redacted_params.push(key.to_string());
        } else {
            redacted_params.push(format!("{key}={value}"));
        }
    }

    format!("{base}{}", redacted_params.join("&"))
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(
        key.to_lowercase().as_str(),
        "password" | "pass" | "token" | "api_key" | "apikey"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_password_in_authority() {
        let conn = "postgres://user:secret@localhost:5432/db";
        let redacted = redact_connection_string(conn);
        assert!(redacted.redacted.contains("***@"));
        assert!(!redacted.redacted.contains("secret"));
        assert_eq!(redacted.user.as_deref(), Some("user"));
        assert_eq!(redacted.host.as_deref(), Some("localhost"));
        assert_eq!(redacted.port, Some(5432));
        assert_eq!(redacted.database.as_deref(), Some("db"));
    }

    #[test]
    fn redacts_query_passwords() {
        let conn = "postgres://user@localhost/db?password=secret&sslmode=require";
        let redacted = redact_connection_string(conn);
        assert!(redacted.redacted.contains("password=***"));
        assert!(redacted.redacted.contains("sslmode=require"));
    }
}
