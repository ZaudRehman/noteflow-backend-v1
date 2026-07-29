use uuid::Uuid;

use crate::models::user::*;
use crate::utils::{
    errors::{AppError, Result},
    validation,
};
use crate::Config;
use sqlx::PgPool;

pub struct UserService {
    pool: PgPool,
    config: Config,
}

impl UserService {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }

    pub async fn get_profile(&self, user_id: Uuid) -> Result<UserProfile> {
        let user = sqlx::query!(
            r#"
            SELECT 
                id, email, display_name, avatar_url, theme, 
                preferences, created_at, updated_at, last_login_at
            FROM users 
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

        Ok(UserProfile {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            theme: user.theme.unwrap_or_else(|| "light".to_string()),
            preferences: user.preferences.unwrap_or_else(|| serde_json::json!({})),
            created_at: user.created_at,
            last_login_at: user.last_login_at,
        })
    }

    pub async fn update_profile(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<UserProfile> {
        if let Some(ref name) = req.display_name {
            let sanitized = validation::sanitize_string(name);
            if sanitized.is_empty() || sanitized.len() > 100 {
                return Err(AppError::ValidationError(
                    "Display name must be 1-100 characters".into(),
                ));
            }
        }

        let mut query = String::from("UPDATE users SET updated_at = NOW()");
        let mut param_count = 1;

        if req.display_name.is_some() {
            param_count += 1;
            query.push_str(&format!(", display_name = ${}", param_count));
        }

        if req.avatar_url.is_some() {
            param_count += 1;
            query.push_str(&format!(", avatar_url = ${}", param_count));
        }

        query.push_str(" WHERE id = $1");

        let mut sql_query = sqlx::query(&query).bind(user_id);

        if let Some(name) = req.display_name {
            sql_query = sql_query.bind(validation::sanitize_string(&name));
        }

        if let Some(url) = req.avatar_url {
            sql_query = sql_query.bind(url);
        }

        sql_query.execute(&self.pool).await?;

        self.get_profile(user_id).await
    }

    pub async fn update_preferences(
        &self,
        user_id: Uuid,
        req: UpdatePreferencesRequest,
    ) -> Result<UserProfile> {
        let theme = req.theme.map(|t| {
            if t == "system" {
                "auto".to_string()
            } else {
                t
            }
        });

        if let Some(ref t) = theme {
            if !["light", "dark", "auto"].contains(&t.as_str()) {
                return Err(AppError::ValidationError(
                    "Theme must be 'light', 'dark', or 'auto'".into(),
                ));
            }
        }

        if let Some(ref prefs) = req.preferences {
            if !prefs.is_object() {
                return Err(AppError::ValidationError(
                    "Preferences must be a JSON object".into(),
                ));
            }
        }

        let mut query = String::from("UPDATE users SET updated_at = NOW()");
        let mut param_count = 1;

        if theme.is_some() {
            param_count += 1;
            query.push_str(&format!(", theme = ${}", param_count));
        }

        if req.preferences.is_some() {
            param_count += 1;
            query.push_str(&format!(", preferences = ${}", param_count));
        }

        query.push_str(" WHERE id = $1");

        let mut sql_query = sqlx::query(&query).bind(user_id);

        if let Some(t) = theme {
            sql_query = sql_query.bind(t);
        }

        if let Some(prefs) = req.preferences {
            sql_query = sql_query.bind(prefs);
        }

        sql_query.execute(&self.pool).await?;

        tracing::info!("User {} updated preferences", user_id);
        self.get_profile(user_id).await
    }

    pub async fn upload_avatar(&self, image_bytes: &[u8], content_type: &str, user_id: &Uuid) -> Result<String> {
        if self.config.imagekit_private_key.is_empty() {
            return Err(AppError::InternalError(
                "Avatar upload not configured (IMAGEKIT_PRIVATE_KEY missing)".into(),
            ));
        }

        if image_bytes.len() > 5_000_000 {
            return Err(AppError::ValidationError("Image too large (max 5MB)".into()));
        }

        // Delete existing avatar first if present
        let existing = sqlx::query_scalar::<_, Option<String>>(
            "SELECT avatar_url FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        if let Some(ref url) = existing {
            if url.contains("imagekit.io") {
                self.delete_avatar_from_cdn(url).await.ok();
            }
        }

        let ext = match content_type {
            c if c.contains("png") => "png",
            c if c.contains("gif") => "gif",
            c if c.contains("webp") => "webp",
            _ => "jpg",
        };

        let file_path = format!("avatars/{}/avatar.{}", user_id, ext);

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        let encoded = b64.encode(image_bytes);

        let client = reqwest::Client::new();
        let form = reqwest::multipart::Form::new()
            .text("file", encoded)
            .text("fileName", file_path)
            .text("useUniqueFileName", "false".to_string());

        let response = client
            .post("https://upload.imagekit.io/api/v1/files/upload")
            .basic_auth(&self.config.imagekit_private_key, None::<&str>)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to upload avatar: {}", e)))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            tracing::error!("ImageKit upload failed: {}", text);
            return Err(AppError::InternalError("Avatar upload failed".into()));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            AppError::InternalError(format!("Failed to parse ImageKit response: {}", e))
        })?;

        let url = json["url"]
            .as_str()
            .ok_or_else(|| AppError::InternalError("Failed to get URL from ImageKit response".into()))?
            .to_string();

        if url.is_empty() {
            return Err(AppError::InternalError("ImageKit returned empty URL".into()));
        }

        sqlx::query!(
            "UPDATE users SET avatar_url = $1, updated_at = NOW() WHERE id = $2",
            url,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("Avatar uploaded to ImageKit for user {}: {}", user_id, url);
        Ok(url)
    }

    pub async fn delete_avatar(&self, user_id: Uuid) -> Result<()> {
        let avatar_url = sqlx::query_scalar::<_, Option<String>>(
            "SELECT avatar_url FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?
        .ok_or_else(|| AppError::NotFound("No avatar set".into()))?;

        self.delete_avatar_from_cdn(&avatar_url).await?;

        sqlx::query!(
            "UPDATE users SET avatar_url = NULL, updated_at = NOW() WHERE id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("Avatar deleted for user {}", user_id);
        Ok(())
    }

    pub async fn delete_account(&self, user_id: Uuid, password: &str) -> Result<()> {
        let user = sqlx::query!(
            "SELECT password_hash, avatar_url FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

        let valid = bcrypt::verify(password, &user.password_hash)
            .map_err(|e| AppError::InternalError(format!("Password verification failed: {}", e)))?;

        if !valid {
            return Err(AppError::AuthenticationError("Invalid password".into()));
        }

        if let Some(ref url) = user.avatar_url {
            if url.contains("imagekit.io") {
                self.delete_avatar_from_cdn(url).await.ok();
            }
        }

        sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(&self.pool)
            .await?;

        tracing::info!("User {} deleted their account", user_id);
        Ok(())
    }

    async fn delete_avatar_from_cdn(&self, avatar_url: &str) -> Result<()> {
        if self.config.imagekit_private_key.is_empty() {
            return Ok(());
        }

        // Extract the ImageKit file path from the URL
        // URL format: https://ik.imagekit.io/{env_id}/avatars/{user_id}/avatar.{ext}
        let parts: Vec<&str> = avatar_url.split("/ik.imagekit.io/").collect();
        if parts.len() < 2 {
            tracing::warn!("Cannot parse ImageKit URL: {}", avatar_url);
            return Ok(());
        }

        let file_path = parts[1].split('/').skip(1).collect::<Vec<&str>>().join("/");
        if file_path.is_empty() {
            return Ok(());
        }

        let client = reqwest::Client::new();
        let response = client
            .delete(&format!(
                "https://api.imagekit.io/v1/files/by-path/{}",
                file_path
            ))
            .basic_auth(&self.config.imagekit_private_key, None::<&str>)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("Avatar deleted from ImageKit: {}", avatar_url);
            }
            Ok(resp) => {
                tracing::warn!("ImageKit delete returned {} for {}: {}", resp.status(), file_path, resp.text().await.unwrap_or_default());
            }
            Err(e) => {
                tracing::warn!("Failed to delete avatar from ImageKit: {}", e);
            }
        }

        Ok(())
    }

    pub async fn update_last_login(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE users SET last_login_at = NOW() WHERE id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}