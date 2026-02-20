use axum::http::{header, HeaderMap};
use std::{env, sync::OnceLock};

pub const ACCESS_TOKEN_COOKIE: &str = "access_token";
pub const REFRESH_TOKEN_COOKIE: &str = "refresh_token";

#[derive(Debug, Clone)]
struct AuthCookieConfig {
    secure: bool,
    same_site: &'static str,
    domain: Option<String>,
}

impl AuthCookieConfig {
    fn from_env() -> Self {
        let same_site = parse_same_site(
            &env::var("AUTH_COOKIE_SAMESITE").unwrap_or_else(|_| "Lax".to_string()),
        );
        let mut secure = parse_bool_env("AUTH_COOKIE_SECURE", false);
        let domain = env::var("AUTH_COOKIE_DOMAIN")
            .ok()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        // Browsers require SameSite=None cookies to also be Secure.
        if same_site == "None" {
            secure = true;
        }

        Self {
            secure,
            same_site,
            domain,
        }
    }
}

fn auth_cookie_config() -> &'static AuthCookieConfig {
    static CONFIG: OnceLock<AuthCookieConfig> = OnceLock::new();
    CONFIG.get_or_init(AuthCookieConfig::from_env)
}

fn parse_bool_env(var_name: &str, default: bool) -> bool {
    env::var(var_name)
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" | "on" => Some(true),
            "0" | "false" | "no" | "n" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn parse_same_site(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "strict" => "Strict",
        "none" => "None",
        _ => "Lax",
    }
}

pub fn build_auth_cookie(name: &str, value: &str, max_age_seconds: u64) -> String {
    let config = auth_cookie_config();
    let mut cookie = format!(
        "{name}={value}; Path=/; Max-Age={max_age_seconds}; HttpOnly; SameSite={}",
        config.same_site
    );

    if config.secure {
        cookie.push_str("; Secure");
    }

    if let Some(domain) = &config.domain {
        cookie.push_str("; Domain=");
        cookie.push_str(domain);
    }

    cookie
}

pub fn build_clear_cookie(name: &str) -> String {
    let config = auth_cookie_config();
    let mut cookie = format!(
        "{name}=; Path=/; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; SameSite={}",
        config.same_site
    );

    if config.secure {
        cookie.push_str("; Secure");
    }

    if let Some(domain) = &config.domain {
        cookie.push_str("; Domain=");
        cookie.push_str(domain);
    }

    cookie
}

pub fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|cookie_header| {
            cookie_header.split(';').find_map(|cookie| {
                let mut parts = cookie.trim().splitn(2, '=');
                let key = parts.next()?.trim();
                let value = parts.next()?.trim();
                if key == name {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
}
