use crate::utils::errors::{AppError, Result};

pub fn validate_email(email: &str) -> Result<()> {
    if !email.contains('@') || email.len() < 5 || email.len() > 255 {
        return Err(AppError::ValidationError(
            "Invalid email format".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(AppError::ValidationError(
            "Password must be at least 8 characters".to_string(),
        ));
    }
    if password.len() > 128 {
        return Err(AppError::ValidationError(
            "Password must be less than 128 characters".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_note_content(content: &str, max_size: usize) -> Result<()> {
    if content.len() > max_size {
        return Err(AppError::ValidationError(format!(
            "Note content exceeds maximum size of {} bytes",
            max_size
        )));
    }
    Ok(())
}

pub fn validate_note_title(title: &str) -> Result<()> {
    if title.is_empty() {
        return Err(AppError::ValidationError(
            "Note title cannot be empty".to_string(),
        ));
    }
    if title.len() > 255 {
        return Err(AppError::ValidationError(
            "Note title must be less than 255 characters".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_tag_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 50 {
        return Err(AppError::ValidationError(
            "Tag name must be between 1 and 50 characters".to_string(),
        ));
    }
    Ok(())
}

pub const VALID_BLOCK_TYPES: [&str; 11] = [
    "paragraph",
    "heading",
    "bullet_list",
    "numbered_list",
    "todo_list",
    "quote",
    "code",
    "divider",
    "table",
    "image",
    "chart",
];

pub fn validate_block_type(block_type: &str) -> Result<()> {
    if !VALID_BLOCK_TYPES.contains(&block_type) {
        return Err(AppError::ValidationError(format!(
            "Unsupported block type: {}",
            block_type
        )));
    }
    Ok(())
}

pub fn validate_block_position(position: i32) -> Result<()> {
    if position < 0 {
        return Err(AppError::ValidationError(
            "Block position cannot be negative".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_block_data(data: Option<&serde_json::Value>, max_size: usize) -> Result<()> {
    if let Some(data) = data {
        let size = serde_json::to_vec(data).map(|v| v.len()).unwrap_or(0);
        if size > max_size {
            return Err(AppError::ValidationError(format!(
                "Block data exceeds maximum size of {} bytes",
                max_size
            )));
        }
    }
    Ok(())
}

pub fn sanitize_string(input: &str) -> String {
    input.trim().to_string()
}
