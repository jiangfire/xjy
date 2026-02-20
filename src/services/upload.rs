use crate::error::{AppError, AppResult};
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

#[derive(Clone)]
pub struct UploadConfig {
    pub upload_dir: String,
}

const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5 MB
const ALLOWED_CONTENT_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

/// Validate file magic bytes match the declared content type.
fn validate_magic_bytes(data: &[u8], content_type: &str) -> bool {
    match content_type {
        "image/jpeg" => data.len() >= 3 && data[..3] == [0xFF, 0xD8, 0xFF],
        "image/png" => data.len() >= 4 && data[..4] == [0x89, 0x50, 0x4E, 0x47],
        "image/gif" => data.len() >= 4 && data[..4] == [0x47, 0x49, 0x46, 0x38],
        "image/webp" => {
            data.len() >= 12
                && data[..4] == [0x52, 0x49, 0x46, 0x46]
                && data[8..12] == [0x57, 0x45, 0x42, 0x50]
        }
        _ => false,
    }
}

pub struct UploadService;

impl UploadService {
    /// Save an uploaded file to disk.
    /// Returns the public URL path (e.g., `/uploads/avatars/uuid.jpg`).
    pub async fn save_file(
        config: &UploadConfig,
        data: &[u8],
        content_type: &str,
        subdirectory: &str,
    ) -> AppResult<String> {
        // Validate size
        if data.len() > MAX_FILE_SIZE {
            return Err(AppError::PayloadTooLarge);
        }

        // Validate content type
        if !ALLOWED_CONTENT_TYPES.contains(&content_type) {
            return Err(AppError::Validation(format!(
                "Unsupported file type: {}. Allowed: jpeg, png, gif, webp",
                content_type
            )));
        }

        // Validate magic bytes match content type
        if !validate_magic_bytes(data, content_type) {
            return Err(AppError::Validation(
                "File content does not match declared content type".to_string(),
            ));
        }

        let ext = match content_type {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => return Err(AppError::Validation("Unsupported file type".to_string())),
        };

        let filename = format!("{}.{}", Uuid::new_v4(), ext);
        let dir = Path::new(&config.upload_dir).join(subdirectory);

        fs::create_dir_all(&dir).await.map_err(|e| {
            AppError::Validation(format!("Failed to create upload directory: {}", e))
        })?;

        let file_path = dir.join(&filename);
        fs::write(&file_path, data)
            .await
            .map_err(|e| AppError::Validation(format!("Failed to write file: {}", e)))?;

        Ok(format!("/uploads/{}/{}", subdirectory, filename))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_magic_bytes_valid() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert!(validate_magic_bytes(&data, "image/jpeg"));
    }

    #[test]
    fn png_magic_bytes_valid() {
        let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        assert!(validate_magic_bytes(&data, "image/png"));
    }

    #[test]
    fn gif_magic_bytes_valid() {
        let data = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
        assert!(validate_magic_bytes(&data, "image/gif"));
    }

    #[test]
    fn webp_magic_bytes_valid() {
        let data = [
            0x52, 0x49, 0x46, 0x46, // RIFF
            0x00, 0x00, 0x00, 0x00, // size
            0x57, 0x45, 0x42, 0x50, // WEBP
        ];
        assert!(validate_magic_bytes(&data, "image/webp"));
    }

    #[test]
    fn wrong_magic_bytes_rejected() {
        let png_data = [0x89, 0x50, 0x4E, 0x47];
        assert!(!validate_magic_bytes(&png_data, "image/jpeg"));
    }

    #[test]
    fn empty_data_rejected() {
        assert!(!validate_magic_bytes(&[], "image/jpeg"));
        assert!(!validate_magic_bytes(&[], "image/png"));
    }

    #[test]
    fn unknown_content_type_rejected() {
        let data = [0xFF, 0xD8, 0xFF];
        assert!(!validate_magic_bytes(&data, "application/pdf"));
    }

    #[test]
    fn too_short_data_rejected() {
        assert!(!validate_magic_bytes(&[0xFF, 0xD8], "image/jpeg"));
        assert!(!validate_magic_bytes(&[0x89, 0x50, 0x4E], "image/png"));
    }
}
