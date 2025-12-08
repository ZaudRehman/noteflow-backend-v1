use sqlx::PgPool;
use uuid::Uuid;
use crate::models::user::{User, RegisterRequest, LoginRequest, AuthResponse};
use crate::utils::{jwt::JwtManager, errors::{AppError, Result}, validation};
use std::sync::Arc;

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

        // Create user
        let user = sqlx::query_as!(
            User,
            r#"INSERT INTO users (email, password_hash, display_name)
               VALUES ($1, $2, $3)
               RETURNING id, email, password_hash, display_name, created_at, updated_at"#,
            email, password_hash, display_name
        )
        .fetch_one(&self.pool)
        .await?;

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(user.id, user.email.clone())?;
        let refresh_token = self.jwt_manager.generate_refresh_token(user.id, user.email.clone())?;

        Ok(AuthResponse {
            user: user.into(),
            access_token,
            refresh_token,
        })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse> {
        let email = validation::sanitize_string(&req.email).to_lowercase();

        // Fetch user
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::AuthenticationError("Invalid credentials".to_string()))?;

        // Verify password
        let password_valid = bcrypt::verify(&req.password, &user.password_hash)
            .map_err(|e| AppError::InternalError(format!("Password verification failed: {}", e)))?;

        if !password_valid {
            return Err(AppError::AuthenticationError("Invalid credentials".to_string()));
        }

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(user.id, user.email.clone())?;
        let refresh_token = self.jwt_manager.generate_refresh_token(user.id, user.email.clone())?;

        Ok(AuthResponse {
            user: user.into(),
            access_token,
            refresh_token,
        })
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<(String, String)> {
        // Verify refresh token
        let claims = self.jwt_manager.verify_refresh_token(refresh_token)?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::AuthenticationError("Invalid user ID".to_string()))?;

        // Fetch user
        let user = sqlx::query!("SELECT email FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::AuthenticationError("User not found".to_string()))?;

        // Generate new tokens
        let new_access = self.jwt_manager.generate_access_token(user_id, user.email.clone())?;
        let new_refresh = self.jwt_manager.generate_refresh_token(user_id, user.email)?;

        Ok((new_access, new_refresh))
    }
}