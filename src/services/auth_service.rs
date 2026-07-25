// src/services/auth_service.rs
use crate::models::user::{
    AuthResponse, LoginRequest, RegisterRequest, SessionInfo, SessionListResponse, UserProfile,
    UserResponse,
};
use crate::utils::{
    errors::{AppError, Result},
    jwt::JwtManager,
    validation,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;
use uuid::Uuid;
use crate::config::Config;
use serde_json::json;

#[derive(Debug, FromRow)]
struct User {
    id: Uuid,
    email: String,
    password_hash: String,
    display_name: String,
    avatar_url: Option<String>,
    theme: Option<String>,
    preferences: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
}

pub struct AuthService {
    pool: PgPool,
    jwt_manager: Arc<JwtManager>,
}

impl AuthService {
    pub fn new(pool: PgPool, jwt_manager: Arc<JwtManager>) -> Self {
        Self { pool, jwt_manager }
    }

    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse> {
        validation::validate_email(&req.email)?;
        validation::validate_password(&req.password)?;

        let email = validation::sanitize_string(&req.email).to_lowercase();
        let display_name = validation::sanitize_string(&req.display_name);

        // Check if email already exists
        let existing = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict("Email already registered".to_string()));
        }

        // Hash password
        let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::InternalError(format!("Password hashing failed: {}", e)))?;

        // Create user - use query_as with explicit type binding
        let user = sqlx::query_as::<_, User>(
            r#"INSERT INTO users (email, password_hash, display_name)
               VALUES ($1, $2, $3)
               RETURNING 
                   id, 
                   email, 
                   password_hash, 
                   display_name, 
                   avatar_url,
                   theme,
                   preferences,
                   created_at, 
                   updated_at,
                   last_login_at"#,
        )
        .bind(&email)
        .bind(&password_hash)
        .bind(&display_name)
        .fetch_one(&self.pool)
        .await?;

        // Generate tokens
        let access_token = self
            .jwt_manager
            .generate_access_token(user.id, user.email.clone())?;
        let refresh_token = self
            .jwt_manager
            .generate_refresh_token(user.id, user.email.clone())?;

        Ok(AuthResponse {
            user: UserResponse {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                created_at: user.created_at,
            },
            access_token,
            refresh_token,
        })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse> {
        let email = validation::sanitize_string(&req.email).to_lowercase();

        // Fetch user - use query_as for dynamic queries
        let user = sqlx::query_as::<_, User>(
            r#"SELECT 
                id, 
                email, 
                password_hash, 
                display_name, 
                avatar_url,
                theme,
                preferences,
                created_at, 
                updated_at,
                last_login_at
            FROM users 
            WHERE email = $1"#,
        )
        .bind(&email)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::AuthenticationError("Invalid credentials".to_string()))?;

        // Verify password
        let password_valid = bcrypt::verify(&req.password, &user.password_hash)
            .map_err(|e| AppError::InternalError(format!("Password verification failed: {}", e)))?;

        if !password_valid {
            return Err(AppError::AuthenticationError(
                "Invalid credentials".to_string(),
            ));
        }

        // Update last login timestamp
        sqlx::query!(
            "UPDATE users SET last_login_at = NOW() WHERE id = $1",
            user.id
        )
        .execute(&self.pool)
        .await?;

        // Generate tokens
        let access_token = self
            .jwt_manager
            .generate_access_token(user.id, user.email.clone())?;
        let refresh_token = self
            .jwt_manager
            .generate_refresh_token(user.id, user.email.clone())?;

        Ok(AuthResponse {
            user: UserResponse {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                created_at: user.created_at,
            },
            access_token,
            refresh_token,
        })
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<(String, String)> {
        // Verify refresh token
        let claims = self.jwt_manager.verify_refresh_token(refresh_token)?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::AuthenticationError("Invalid user ID".to_string()))?;

        // Revoke the old refresh token (rotation — prevents replay attacks)
        let token_hash = self.hash_token(refresh_token);
        sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET revoked = true, revoked_at = NOW()
            WHERE token_hash = $1 AND NOT revoked
            "#,
            token_hash
        )
        .execute(&self.pool)
        .await?;

        // Fetch user
        let user = sqlx::query!("SELECT email FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::AuthenticationError("User not found".to_string()))?;

        // Generate new tokens
        let new_access = self
            .jwt_manager
            .generate_access_token(user_id, user.email.clone())?;
        let new_refresh = self
            .jwt_manager
            .generate_refresh_token(user_id, user.email)?;

        tracing::info!("Refresh token rotated for user {}", user_id);

        Ok((new_access, new_refresh))
    }

    /// Get current authenticated user profile
    pub async fn get_current_user(&self, user_id: Uuid) -> Result<UserProfile> {
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

    /// Store refresh token in database
    pub async fn store_refresh_token(
        &self,
        user_id: Uuid,
        token: &str,
        expires_at: DateTime<Utc>,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Result<()> {
        let token_hash = self.hash_token(token);

        // Convert IP address string to IpNetwork
        let ip_network = ip_address.and_then(|ip_str| {
            ip_str
                .parse::<std::net::IpAddr>()
                .ok()
                .and_then(|ip| IpNetwork::new(ip, if ip.is_ipv4() { 32 } else { 128 }).ok())
        });

        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at, user_agent, ip_address)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            user_id,
            token_hash,
            expires_at,
            user_agent,
            ip_network as Option<IpNetwork>,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Revoke a refresh token
    pub async fn revoke_refresh_token(&self, token: &str) -> Result<()> {
        let token_hash = self.hash_token(token);

        let result = sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET revoked = true, revoked_at = NOW()
            WHERE token_hash = $1 AND NOT revoked
            "#,
            token_hash
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::AuthenticationError(
                "Token not found or already revoked".into(),
            ));
        }

        tracing::info!("Refresh token revoked");
        Ok(())
    }

    /// Verify refresh token is not revoked
    pub async fn verify_refresh_token_not_revoked(&self, token: &str) -> Result<()> {
        let token_hash = self.hash_token(token);

        let is_valid = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM refresh_tokens
                WHERE token_hash = $1
                    AND NOT revoked
                    AND expires_at > NOW()
            )
            "#,
            token_hash
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);

        if !is_valid {
            return Err(AppError::AuthenticationError(
                "Refresh token is invalid or revoked".into(),
            ));
        }

        Ok(())
    }
    
    /// Send password reset email via Brevo
    async fn send_reset_email(&self, email: &str, token: &str, config: &Config) -> Result<()> {
        if config.brevo_api_key.is_empty() {
            tracing::warn!("BREVO_API_KEY not set, skipping password reset email to {}", email);
            return Ok(());
        }

        let reset_url = format!("{}/reset-password?token={}", config.app_url, token);

        let client = reqwest::Client::new();
        let payload = json!({
            "sender": { "email": config.email_from, "name": "NoteFlow" },
            "to": [{ "email": email }],
            "subject": "Reset your NoteFlow password",
            "htmlContent": format!(
                r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:20px;background-color:#1a1a1f;color:#e1e1e6;border-radius:12px;">
                    <h1 style="color:#b8a4d4;font-size:24px;">NoteFlow</h1>
                    <p style="font-size:16px;line-height:1.5;">You requested to reset your password. Click the button below to set a new one. This link will expire in 1 hour.</p>
                    <div style="margin:30px 0;">
                        <a href="{}" style="background-color:#b8a4d4;color:#1a1a1f;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:bold;display:inline-block;">Reset Password</a>
                    </div>
                    <p style="font-size:14px;color:#a1a1aa;">If you didn't request this, you can safely ignore this email.</p>
                    <hr style="border:0;border-top:1px solid #3a3a44;margin:20px 0;">
                    <p style="font-size:12px;color:#71717a;">Sent by NoteFlow</p>
                </div>"#,
                reset_url
            )
        });

        let response = client
            .post("https://api.brevo.com/v3/smtp/email")
            .header("api-key", &config.brevo_api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to send email: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("Brevo API error: {}", error_text);
            return Err(AppError::InternalError("Failed to send reset email".into()));
        }

        Ok(())
    }

    /// List active sessions for a user
    pub async fn list_sessions(&self, user_id: Uuid, current_token_hash: &str) -> Result<SessionListResponse> {
        let sessions = sqlx::query!(
            r#"
            SELECT id, user_agent, ip_address, created_at, expires_at, token_hash
            FROM refresh_tokens
            WHERE user_id = $1 AND NOT revoked AND expires_at > NOW()
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let total = sessions.len() as i64;

        let session_list: Vec<SessionInfo> = sessions
            .into_iter()
            .map(|s| SessionInfo {
                id: s.id,
                user_agent: s.user_agent,
                ip_address: s.ip_address.map(|ip| ip.to_string()),
                created_at: s.created_at,
                expires_at: s.expires_at,
                is_current: s.token_hash == current_token_hash,
            })
            .collect();

        Ok(SessionListResponse {
            sessions: session_list,
            total,
        })
    }

    /// Revoke a specific session by ID
    pub async fn revoke_session(&self, session_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET revoked = true, revoked_at = NOW()
            WHERE id = $1 AND user_id = $2 AND NOT revoked
            "#,
            session_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Session not found or already revoked".into()));
        }

        tracing::info!("Session {} revoked for user {}", session_id, user_id);
        Ok(())
    }

    /// Create password reset token
    pub async fn create_password_reset_token(&self, email: &str) -> Result<String> {
        let email = validation::sanitize_string(email).to_lowercase();

        // Check if user exists
        let user = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
            .fetch_optional(&self.pool)
            .await?;

        // Always return success to prevent email enumeration
        if user.is_none() {
            tracing::warn!("Password reset requested for non-existent email: {}", email);
            return Ok("If that email exists, a reset link has been sent".into());
        }

        let user_id = user.unwrap().id;

        // Generate secure random token
        let token = self.generate_reset_token();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        sqlx::query!(
            r#"
            UPDATE users
            SET reset_token = $1, reset_token_expires = $2
            WHERE id = $3
            "#,
            token,
            expires_at,
            user_id
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("Password reset token created for user {}", user_id);
        
        // Load config for Resend
        let config = Config::from_env().map_err(|e| AppError::InternalError(format!("Config error: {}", e)))?;
        
        // Send actual email via Resend
        self.send_reset_email(&email, &token, &config).await?;

        Ok("If that email exists, a reset link has been sent".into())
    }

    /// Reset password using token
    pub async fn reset_password(&self, token: &str, new_password: &str) -> Result<()> {
        validation::validate_password(new_password)?;

        // Find user with valid token
        let user = sqlx::query!(
            r#"
            SELECT id, email FROM users
            WHERE reset_token = $1
                AND reset_token_expires > NOW()
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::AuthenticationError("Invalid or expired reset token".into()))?;

        // Hash new password
        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::InternalError(format!("Password hashing failed: {}", e)))?;

        // Update password and clear token
        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $1,
                reset_token = NULL,
                reset_token_expires = NULL,
                updated_at = NOW()
            WHERE id = $2
            "#,
            password_hash,
            user.id
        )
        .execute(&self.pool)
        .await?;

        // Revoke all refresh tokens for security
        sqlx::query!(
            "UPDATE refresh_tokens SET revoked = true, revoked_at = NOW() WHERE user_id = $1 AND NOT revoked",
            user.id
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("Password reset successful for user {}", user.id);
        Ok(())
    }

    // Helper methods

    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn generate_reset_token(&self) -> String {
        let mut rng = rand::thread_rng();
        let token: String = (0..32)
            .map(|_| format!("{:02x}", rng.gen::<u8>()))
            .collect();
        token
    }
}
