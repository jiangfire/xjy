use anyhow::{Context, Result};

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).context("Failed to hash password")
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    bcrypt::verify(password, hash).context("Failed to verify password")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails() {
        let hash = hash_password("correct_password").unwrap();
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn empty_password_hashes() {
        let hash = hash_password("").unwrap();
        assert!(verify_password("", &hash).unwrap());
        assert!(!verify_password("notempty", &hash).unwrap());
    }

    #[test]
    fn different_hashes_for_same_password() {
        let hash1 = hash_password("same_password").unwrap();
        let hash2 = hash_password("same_password").unwrap();
        // bcrypt uses random salt, so hashes should differ
        assert_ne!(hash1, hash2);
        // But both should verify
        assert!(verify_password("same_password", &hash1).unwrap());
        assert!(verify_password("same_password", &hash2).unwrap());
    }
}
