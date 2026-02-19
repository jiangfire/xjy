use std::env;

#[derive(Debug, Clone, Copy)]
pub struct AuthConfig {
    pub require_email_verification: bool,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let require_email_verification = env::var("REQUIRE_EMAIL_VERIFICATION")
            .ok()
            .and_then(|v| {
                let v = v.trim().to_ascii_lowercase();
                match v.as_str() {
                    "1" | "true" | "yes" | "y" | "on" => Some(true),
                    "0" | "false" | "no" | "n" | "off" => Some(false),
                    _ => None,
                }
            })
            .unwrap_or(false);

        Self {
            require_email_verification,
        }
    }
}

