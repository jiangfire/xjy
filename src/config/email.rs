use std::env;

#[derive(Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub frontend_url: String,
}

impl EmailConfig {
    /// Read email config from environment variables.
    /// Returns None if SMTP is not configured (graceful degradation).
    pub fn from_env() -> Option<Self> {
        let smtp_host = env::var("SMTP_HOST").ok()?;
        let smtp_port = env::var("SMTP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(587);
        let smtp_username = env::var("SMTP_USERNAME").ok()?;
        let smtp_password = env::var("SMTP_PASSWORD").ok()?;
        let from_address = env::var("SMTP_FROM")
            .unwrap_or_else(|_| format!("Forum <{}>", smtp_username.clone()));
        let frontend_url =
            env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        Some(Self {
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            from_address,
            frontend_url,
        })
    }
}
