use std::{net::IpAddr, path::PathBuf};

use anyhow::{Context, bail};

/// Runtime settings, read from the environment.
#[derive(Clone, Debug)]
pub struct Config {
    /// Postgres connection string (`DATABASE_URL`, required).
    pub database_url: String,
    /// Pool size (`DATABASE_MAX_CONNECTIONS`, default 10).
    pub database_max_connections: u32,
    /// Address to listen on (`BIND_ADDR`, default 127.0.0.1).
    pub bind_addr: IpAddr,
    /// Port to listen on (`PORT`, default 3000).
    pub port: u16,
    /// Built frontend to serve for non-API paths (`STATIC_DIR`, optional).
    pub static_dir: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let Some(database_url) = get("DATABASE_URL") else {
            bail!("DATABASE_URL is not set (see .env.example)");
        };
        let database_max_connections = parse_or(&get, "DATABASE_MAX_CONNECTIONS", 10)?;
        let bind_addr = parse_or(&get, "BIND_ADDR", IpAddr::from([127, 0, 0, 1]))?;
        let port = parse_or(&get, "PORT", 3000)?;
        let static_dir = get("STATIC_DIR")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        Ok(Self {
            database_url,
            database_max_connections,
            bind_addr,
            port,
            static_dir,
        })
    }
}

fn parse_or<T>(get: &impl Fn(&str) -> Option<String>, key: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match get(key) {
        Some(value) if !value.is_empty() => value
            .parse()
            .with_context(|| format!("{key} has an invalid value: {value:?}")),
        _ => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config(vars: &[(&str, &str)]) -> anyhow::Result<Config> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|key| vars.get(key).cloned())
    }

    #[test]
    fn requires_database_url() {
        assert!(config(&[]).is_err());
    }

    #[test]
    fn applies_defaults() {
        let config = config(&[("DATABASE_URL", "postgres://localhost/kenning")]).unwrap();
        assert_eq!(config.port, 3000);
        assert_eq!(config.bind_addr.to_string(), "127.0.0.1");
        assert_eq!(config.database_max_connections, 10);
        assert!(config.static_dir.is_none());
    }

    #[test]
    fn rejects_bad_port() {
        let err = config(&[("DATABASE_URL", "postgres://x"), ("PORT", "eighty")]).unwrap_err();
        assert!(err.to_string().contains("PORT"));
    }
}
