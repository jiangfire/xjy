use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitRule {
    pub per_second: u64,
    pub burst_size: u32,
}

impl RateLimitRule {
    const fn new(per_second: u64, burst_size: u32) -> Self {
        Self {
            per_second,
            burst_size,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub auth: RateLimitRule,
    pub public_read: RateLimitRule,
    pub protected: RateLimitRule,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auth: RateLimitRule::new(5, 10),
            public_read: RateLimitRule::new(30, 60),
            protected: RateLimitRule::new(10, 20),
        }
    }
}

impl RateLimitConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.enabled = parse_bool_env("RATE_LIMIT_ENABLED", cfg.enabled);

        if let Ok(raw) = env::var("RATE_LIMIT_CONFIG") {
            match parse_rate_limit_config(&raw) {
                Ok(parsed) => cfg = cfg.apply_partial(parsed),
                Err(err) => {
                    tracing::warn!("Invalid RATE_LIMIT_CONFIG '{}': {}", raw, err);
                }
            }
        }

        cfg
    }

    fn apply_partial(mut self, parsed: PartialRateLimitConfig) -> Self {
        if let Some(rule) = parsed.global {
            self.auth = rule;
            self.public_read = rule;
            self.protected = rule;
        }
        if let Some(rule) = parsed.auth {
            self.auth = rule;
        }
        if let Some(rule) = parsed.public_read {
            self.public_read = rule;
        }
        if let Some(rule) = parsed.protected {
            self.protected = rule;
        }
        self
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct PartialRateLimitConfig {
    global: Option<RateLimitRule>,
    auth: Option<RateLimitRule>,
    public_read: Option<RateLimitRule>,
    protected: Option<RateLimitRule>,
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

fn parse_rate_limit_config(raw: &str) -> Result<PartialRateLimitConfig, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty value".to_string());
    }

    // Global format: "10:20" -> apply to all groups.
    if !trimmed.contains('=') {
        let rule = parse_rule(trimmed)?;
        return Ok(PartialRateLimitConfig {
            global: Some(rule),
            ..Default::default()
        });
    }

    // Grouped format: "auth=5:10,public=30:60,protected=10:20"
    let mut parsed = PartialRateLimitConfig::default();
    for item in trimmed.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let (name, raw_rule) = item
            .split_once('=')
            .ok_or_else(|| format!("invalid item '{}', expected name=per:burst", item))?;
        let rule = parse_rule(raw_rule.trim())?;
        match normalize_group_name(name.trim()) {
            Some("auth") => parsed.auth = Some(rule),
            Some("public_read") => parsed.public_read = Some(rule),
            Some("protected") => parsed.protected = Some(rule),
            _ => {
                return Err(format!(
                    "unknown group '{}', expected auth/public/protected",
                    name.trim()
                ));
            }
        }
    }

    Ok(parsed)
}

fn normalize_group_name(name: &str) -> Option<&'static str> {
    match name.to_ascii_lowercase().as_str() {
        "auth" => Some("auth"),
        "public" | "public_read" | "public-read" => Some("public_read"),
        "protected" => Some("protected"),
        _ => None,
    }
}

fn parse_rule(raw: &str) -> Result<RateLimitRule, String> {
    let (per_second_raw, burst_raw) = raw
        .split_once(':')
        .ok_or_else(|| format!("invalid rule '{}', expected per:burst", raw))?;

    let per_second: u64 = per_second_raw
        .trim()
        .parse()
        .map_err(|_| format!("invalid per_second '{}'", per_second_raw.trim()))?;
    let burst_size: u32 = burst_raw
        .trim()
        .parse()
        .map_err(|_| format!("invalid burst_size '{}'", burst_raw.trim()))?;

    if per_second == 0 {
        return Err("per_second must be > 0".to_string());
    }
    if burst_size == 0 {
        return Err("burst_size must be > 0".to_string());
    }

    Ok(RateLimitRule::new(per_second, burst_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_global_rule() {
        let parsed = parse_rate_limit_config("12:24").unwrap();
        assert_eq!(parsed.global, Some(RateLimitRule::new(12, 24)));
        assert_eq!(parsed.auth, None);
    }

    #[test]
    fn parse_grouped_rules() {
        let parsed = parse_rate_limit_config("auth=1:2,public=3:4,protected=5:6").unwrap();
        assert_eq!(parsed.auth, Some(RateLimitRule::new(1, 2)));
        assert_eq!(parsed.public_read, Some(RateLimitRule::new(3, 4)));
        assert_eq!(parsed.protected, Some(RateLimitRule::new(5, 6)));
    }

    #[test]
    fn parse_group_alias() {
        let parsed = parse_rate_limit_config("public-read=8:16").unwrap();
        assert_eq!(parsed.public_read, Some(RateLimitRule::new(8, 16)));
    }

    #[test]
    fn parse_invalid_rule() {
        let err = parse_rate_limit_config("auth=abc").unwrap_err();
        assert!(err.contains("invalid rule"));
    }
}
