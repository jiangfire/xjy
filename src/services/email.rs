use crate::config::email::EmailConfig;
use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

#[derive(Clone)]
pub struct EmailService {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from_address: Option<String>,
    frontend_url: String,
}

impl EmailService {
    /// Build from environment variables. If SMTP is not configured, email
    /// sending is silently skipped (graceful degradation).
    pub fn from_env() -> Self {
        match EmailConfig::from_env() {
            Some(cfg) => {
                let creds = Credentials::new(cfg.smtp_username.clone(), cfg.smtp_password.clone());
                let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)
                    .map(|builder| builder.port(cfg.smtp_port).credentials(creds).build());

                match transport {
                    Ok(t) => Self {
                        transport: Some(t),
                        from_address: Some(cfg.from_address),
                        frontend_url: cfg.frontend_url,
                    },
                    Err(e) => {
                        tracing::warn!("Failed to build SMTP transport: {e}");
                        Self {
                            transport: None,
                            from_address: None,
                            frontend_url: cfg.frontend_url,
                        }
                    }
                }
            }
            None => {
                let frontend_url = std::env::var("FRONTEND_URL")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string());
                Self {
                    transport: None,
                    from_address: None,
                    frontend_url,
                }
            }
        }
    }

    /// Returns true if SMTP is configured and available.
    pub fn is_configured(&self) -> bool {
        self.transport.is_some()
    }

    /// Send a verification email. Silently succeeds if SMTP is not configured.
    pub async fn send_verification_email(&self, to: &str, token: &str) -> Result<()> {
        let link = format!("{}/verify-email?token={}", self.frontend_url, token);
        let body = format!(
            "Welcome! Please verify your email by clicking the link below:\n\n{}\n\nThis link expires in 24 hours.",
            link
        );

        self.send_email(to, "Verify your email", &body).await
    }

    /// Send a password reset email. Silently succeeds if SMTP is not configured.
    pub async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<()> {
        let link = format!("{}/reset-password?token={}", self.frontend_url, token);
        let body = format!(
            "A password reset was requested for your account.\n\nClick the link below to reset your password:\n\n{}\n\nThis link expires in 1 hour. If you did not request this, you can safely ignore this email.",
            link
        );

        self.send_email(to, "Reset your password", &body).await
    }

    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        let transport = match &self.transport {
            Some(t) => t,
            None => {
                tracing::debug!("SMTP not configured, skipping email to {to}");
                return Ok(());
            }
        };
        let from_address = match &self.from_address {
            Some(f) => f,
            None => return Ok(()),
        };

        let from_mailbox: Mailbox =
            from_address
                .parse()
                .map_err(|e: lettre::address::AddressError| {
                    anyhow::anyhow!("Invalid from address '{}': {}", from_address, e)
                })?;
        let to_mailbox: Mailbox = to.parse().map_err(|e: lettre::address::AddressError| {
            anyhow::anyhow!("Invalid to address '{}': {}", to, e)
        })?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())?;

        transport.send(email).await?;
        tracing::info!("Email sent to {to}: {subject}");
        Ok(())
    }
}
