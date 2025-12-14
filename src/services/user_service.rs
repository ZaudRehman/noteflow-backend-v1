// src/services/user_service.rs
use crate::models::user::*;
use crate::utils::{
    errors::{AppError, Result},
    validation,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get user profile with all details
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

    /// Update user profile
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<UserProfile> {
        // Validate display name if provided
        if let Some(ref name) = req.display_name {
            let sanitized = validation::sanitize_string(name);
            if sanitized.is_empty() || sanitized.len() > 100 {
                return Err(AppError::ValidationError(
                    "Display name must be 1-100 characters".into(),
                ));
            }
        }

        // Validate avatar URL if provided
        if let Some(ref url) = req.avatar_url {
            if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(AppError::ValidationError(
                    "Avatar URL must be a valid HTTP(S) URL".into(),
                ));
            }
        }

        // Build dynamic update query
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

    /// Update user preferences (theme + custom preferences JSON)
    pub async fn update_preferences(
        &self,
        user_id: Uuid,
        req: UpdatePreferencesRequest,
    ) -> Result<()> {
        // Validate theme if provided
        if let Some(ref theme) = req.theme {
            if !["light", "dark", "auto"].contains(&theme.as_str()) {
                return Err(AppError::ValidationError(
                    "Theme must be 'light', 'dark', or 'auto'".into(),
                ));
            }
        }

        // Validate preferences is an object if provided
        if let Some(ref prefs) = req.preferences {
            if !prefs.is_object() {
                return Err(AppError::ValidationError(
                    "Preferences must be a JSON object".into(),
                ));
            }
        }

        let mut query = String::from("UPDATE users SET updated_at = NOW()");
        let mut param_count = 1;

        if req.theme.is_some() {
            param_count += 1;
            query.push_str(&format!(", theme = ${}", param_count));
        }

        if req.preferences.is_some() {
            param_count += 1;
            query.push_str(&format!(", preferences = ${}", param_count));
        }

        query.push_str(" WHERE id = $1");

        let mut sql_query = sqlx::query(&query).bind(user_id);

        if let Some(theme) = req.theme {
            sql_query = sql_query.bind(theme);
        }

        if let Some(prefs) = req.preferences {
            sql_query = sql_query.bind(prefs);
        }

        sql_query.execute(&self.pool).await?;

        tracing::info!("User {} updated preferences", user_id);
        Ok(())
    }

    /// Update last login timestamp
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
