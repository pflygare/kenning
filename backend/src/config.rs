use std::{net::IpAddr, path::PathBuf};

use anyhow::{Context, bail};
use url::Url;

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
    /// Where users reach the app, used in emailed links and the Google
    /// redirect (`PUBLIC_URL`, default http://localhost:3000). An https URL
    /// also marks the session cookie Secure.
    pub public_url: Url,
    /// Google sign-in (`GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`);
    /// the button is hidden when unset.
    pub google: Option<GoogleConfig>,
    /// Outgoing mail server as a URL, such as
    /// `smtps://user:pass@smtp.example.com:465` (`SMTP_URL`). When unset,
    /// emails are written to the log instead of sent.
    pub smtp_url: Option<String>,
    /// Sender of outgoing email (`MAIL_FROM`, default `Kenning <no-reply@localhost>`).
    pub mail_from: String,
    /// The testing page at `/dev`: an outbox of every email sent and
    /// shortcuts such as confirming your email without one (`DEV_TOOLS`,
    /// default false). It lets anyone read every email, password resets
    /// included, so never turn it on where real people sign up.
    pub dev_tools: bool,
}

#[derive(Clone, Debug)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
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
        let static_dir = non_empty(&get, "STATIC_DIR").map(PathBuf::from);
        let public_url = parse_or(&get, "PUBLIC_URL", Url::parse("http://localhost:3000")?)?;
        let google = match (
            non_empty(&get, "GOOGLE_CLIENT_ID"),
            non_empty(&get, "GOOGLE_CLIENT_SECRET"),
        ) {
            (Some(client_id), Some(client_secret)) => Some(GoogleConfig {
                client_id,
                client_secret,
            }),
            (None, None) => None,
            _ => bail!("set both GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET, or neither"),
        };
        let smtp_url = non_empty(&get, "SMTP_URL");
        let mail_from = non_empty(&get, "MAIL_FROM")
            .unwrap_or_else(|| "Kenning <no-reply@localhost>".to_string());
        let dev_tools = parse_or(&get, "DEV_TOOLS", false)?;

        Ok(Self {
            database_url,
            database_max_connections,
            bind_addr,
            port,
            static_dir,
            public_url,
            google,
            smtp_url,
            mail_from,
            dev_tools,
        })
    }

    /// An absolute URL for `path` (which starts with `/`) on the public site.
    pub fn link(&self, path: &str) -> String {
        let base = self.public_url.as_str().trim_end_matches('/');
        format!("{base}{path}")
    }

    pub fn secure_cookies(&self) -> bool {
        self.public_url.scheme() == "https"
    }

    /// Settings for tests: a dummy database URL and every optional feature off.
    pub fn for_tests() -> Self {
        Self::from_lookup(|key| (key == "DATABASE_URL").then(|| "postgres://test".to_string()))
            .expect("test config")
    }
}

fn non_empty(get: &impl Fn(&str) -> Option<String>, key: &str) -> Option<String> {
    get(key).filter(|value| !value.is_empty())
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
        assert!(!config.dev_tools);
    }

    #[test]
    fn google_needs_both_halves() {
        assert!(config(&[("DATABASE_URL", "x"), ("GOOGLE_CLIENT_ID", "id")]).is_err());
        let both = config(&[
            ("DATABASE_URL", "x"),
            ("GOOGLE_CLIENT_ID", "id"),
            ("GOOGLE_CLIENT_SECRET", "secret"),
        ])
        .unwrap();
        assert_eq!(both.google.unwrap().client_id, "id");
    }

    #[test]
    fn builds_links_from_public_url() {
        let config = config(&[
            ("DATABASE_URL", "x"),
            ("PUBLIC_URL", "https://wiki.example.com/"),
        ])
        .unwrap();
        assert_eq!(
            config.link("/invite/abc"),
            "https://wiki.example.com/invite/abc"
        );
        assert!(config.secure_cookies());
    }

    #[test]
    fn rejects_bad_port() {
        let err = config(&[("DATABASE_URL", "postgres://x"), ("PORT", "eighty")]).unwrap_err();
        assert!(err.to_string().contains("PORT"));
    }
}
