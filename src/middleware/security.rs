use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use std::{env, sync::OnceLock};

const DEFAULT_CSP_POLICY: &str = "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self' 'unsafe-inline'; worker-src 'self' blob:; child-src 'self' blob:; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' ws: wss:";
const HSTS_VALUE: &str = "max-age=31536000; includeSubDomains";

#[derive(Debug, Clone)]
struct SecurityHeadersConfig {
    csp: HeaderValue,
    enable_hsts: bool,
}

impl SecurityHeadersConfig {
    fn from_env() -> Self {
        let raw_csp = env::var("CSP_POLICY").unwrap_or_else(|_| DEFAULT_CSP_POLICY.to_string());
        let csp = HeaderValue::from_str(&raw_csp).unwrap_or_else(|err| {
            tracing::warn!(
                "Invalid CSP_POLICY value ({}), falling back to default policy",
                err
            );
            HeaderValue::from_static(DEFAULT_CSP_POLICY)
        });

        let enable_hsts = parse_bool_env("ENABLE_HSTS", true);

        Self { csp, enable_hsts }
    }
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

fn security_headers_config() -> &'static SecurityHeadersConfig {
    static CONFIG: OnceLock<SecurityHeadersConfig> = OnceLock::new();
    CONFIG.get_or_init(SecurityHeadersConfig::from_env)
}

pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let config = security_headers_config();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert("content-security-policy", config.csp.clone());
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );
    headers.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );

    if config.enable_hsts {
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static(HSTS_VALUE),
        );
    }

    response
}
