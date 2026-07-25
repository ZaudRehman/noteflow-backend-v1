use crate::config::Config;
use crate::utils::errors::{AppError, Result};
use crate::utils::web_push;
use base64::Engine;
use chrono::Utc;
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use rand::rngs::OsRng;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

pub struct NotificationService {
    pool: PgPool,
    config: Arc<Config>,
}

#[derive(Debug, serde::Serialize)]
pub struct PushSubscriptionInfo {
    pub id: Uuid,
    pub endpoint: String,
    pub user_agent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl NotificationService {
    pub fn new(pool: PgPool, config: Arc<Config>) -> Self {
        Self { pool, config }
    }

    // ── VAPID helpers ──

    fn get_or_init_vapid(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let config = &self.config;

        if !config.vapid_public_key.is_empty() && !config.vapid_private_key.is_empty() {
            let pub_bytes =
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(&config.vapid_public_key)
                    .map_err(|e| AppError::InternalError(format!("Invalid VAPID_PUBLIC_KEY: {}", e)))?;
            let priv_bytes =
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(&config.vapid_private_key)
                    .map_err(|e| AppError::InternalError(format!("Invalid VAPID_PRIVATE_KEY: {}", e)))?;
            return Ok((pub_bytes, priv_bytes));
        }

        let sk = p256::SecretKey::random(&mut OsRng);
        let pk = sk.public_key();
        let pub_raw = pk.to_encoded_point(false).as_bytes().to_vec();
        let priv_raw = sk.to_bytes().to_vec();

        let pub_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&pub_raw);
        let priv_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&priv_raw);

        tracing::warn!(
            "No VAPID keys configured. Generated temporary keys. \
             Save to .env:\n  VAPID_PUBLIC_KEY={}\n  VAPID_PRIVATE_KEY={}",
            pub_b64,
            priv_b64,
        );

        Ok((pub_raw, priv_raw))
    }

    fn create_vapid_jwt(
        private_key_raw: &[u8],
        _public_key_raw: &[u8],
        endpoint: &str,
        subject: &str,
    ) -> Result<String> {
        let aud = url::Url::parse(endpoint)
            .map_err(|_| AppError::InternalError("Invalid push endpoint URL".into()))?
            .origin()
            .ascii_serialization();

        let header = json!({"typ": "JWT", "alg": "ES256"});
        let now = Utc::now().timestamp();
        let payload = json!({
            "aud": aud,
            "exp": now + 86400,
            "sub": subject,
        });

        let b64_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            &serde_json::to_vec(&header).unwrap(),
        );
        let b64_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            &serde_json::to_vec(&payload).unwrap(),
        );

        let signing_input = format!("{}.{}", b64_header, b64_payload);

        let signing_key = SigningKey::from_slice(private_key_raw)
            .map_err(|e| AppError::InternalError(format!("Invalid VAPID private key: {}", e)))?;

        let signature: Signature = signing_key.sign(signing_input.as_bytes());
        let b64_sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            signature.to_bytes().as_slice(),
        );

        Ok(format!("{}.{}", signing_input, b64_sig))
    }

    // ── Push subscription management ──

    pub async fn subscribe(
        &self,
        user_id: Uuid,
        endpoint: &str,
        p256dh_key: &str,
        auth_key: &str,
        user_agent: Option<String>,
    ) -> Result<PushSubscriptionInfo> {
        let row = sqlx::query(
            r#"
            INSERT INTO push_subscriptions (user_id, endpoint, p256dh_key, auth_key, user_agent)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, endpoint)
            DO UPDATE SET
                p256dh_key = EXCLUDED.p256dh_key,
                auth_key = EXCLUDED.auth_key,
                user_agent = EXCLUDED.user_agent
            RETURNING id, endpoint, user_agent, created_at
            "#,
        )
        .bind(user_id)
        .bind(endpoint)
        .bind(p256dh_key)
        .bind(auth_key)
        .bind(&user_agent)
        .fetch_one(&self.pool)
        .await?;

        Ok(PushSubscriptionInfo {
            id: row.get("id"),
            endpoint: row.get("endpoint"),
            user_agent: row.get("user_agent"),
            created_at: row.get("created_at"),
        })
    }

    pub async fn unsubscribe(&self, subscription_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM push_subscriptions WHERE id = $1 AND user_id = $2",
        )
        .bind(subscription_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Subscription not found".into()));
        }
        Ok(())
    }

    pub async fn list_subscriptions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PushSubscriptionInfo>> {
        let rows = sqlx::query(
            "SELECT id, endpoint, user_agent, created_at FROM push_subscriptions WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| PushSubscriptionInfo {
                id: r.get("id"),
                endpoint: r.get("endpoint"),
                user_agent: r.get("user_agent"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    // ── Push sending ──

    pub async fn send_push_to_user(
        &self,
        user_id: Uuid,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<()> {
        let subscriptions = sqlx::query(
            "SELECT id, endpoint, p256dh_key, auth_key FROM push_subscriptions WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        if subscriptions.is_empty() {
            return Ok(());
        }

        let (vapid_pub, vapid_priv) = self.get_or_init_vapid()?;
        let payload_bytes = serde_json::to_vec(&json!({
            "title": title,
            "body": body,
            "data": data,
        }))
        .map_err(|e| AppError::InternalError(format!("Serialization: {}", e)))?;

        for sub in &subscriptions {
            let p256dh: Vec<u8> = match base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(sub.get::<String, _>("p256dh_key"))
            {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Invalid p256dh key: {}", e);
                    continue;
                }
            };
            let auth: Vec<u8> = match base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(sub.get::<String, _>("auth_key"))
            {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Invalid auth key: {}", e);
                    continue;
                }
            };

            let encrypted = match web_push::encrypt_push_payload(&p256dh, &auth, &payload_bytes) {
                Ok(e) => e,
                Err(err) => {
                    tracing::error!("Encryption failed: {}", err);
                    continue;
                }
            };

            let sub_id: Uuid = sub.get("id");
            let endpoint: String = sub.get("endpoint");

            let vapid_jwt = Self::create_vapid_jwt(
                &vapid_priv,
                &vapid_pub,
                &endpoint,
                &self.config.vapid_subject,
            )?;
            let vapid_pub_b64 =
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&vapid_pub);

            let client = reqwest::Client::new();
            let response = client
                .post(&endpoint)
                .header("Content-Type", "application/octet-stream")
                .header("Content-Encoding", "aes128gcm")
                .header("TTL", "86400")
                .header(
                    "Authorization",
                    format!("vapid t={}, k={}", vapid_jwt, vapid_pub_b64),
                )
                .body(encrypted.body)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    tracing::debug!("Push sent to subscription {}", sub_id);
                }
                Ok(ref resp) if resp.status().as_u16() == 410 => {
                    tracing::warn!("Push endpoint expired, removing subscription {}", sub_id);
                    let _ = sqlx::query("DELETE FROM push_subscriptions WHERE id = $1")
                        .bind(sub_id)
                        .execute(&self.pool)
                        .await;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    tracing::warn!("Push to {} returned {}: {}", sub_id, status, text);
                }
                Err(e) => {
                    tracing::error!("Push to {} failed: {}", sub_id, e);
                }
            }
        }

        Ok(())
    }

    // ── Email sending ──

    pub async fn send_email(&self, to: &str, subject: &str, html: &str) -> Result<()> {
        if self.config.brevo_api_key.is_empty() {
            return Err(AppError::InternalError("BREVO_API_KEY not configured".into()));
        }

        let client = reqwest::Client::new();
        let payload = json!({
            "sender": { "email": self.config.email_from, "name": "NoteFlow" },
            "to": [{ "email": to }],
            "subject": subject,
            "htmlContent": html,
        });

        let response = client
            .post("https://api.brevo.com/v3/smtp/email")
            .header("api-key", &self.config.brevo_api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Email send failed: {}", e)))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            tracing::error!("Brevo API error: {}", text);
            return Err(AppError::InternalError("Email send failed".into()));
        }

        tracing::info!("Email sent to {}: {}", to, subject);
        Ok(())
    }

    pub async fn send_notification_email(
        &self,
        user_id: Uuid,
        subject: &str,
        body_html: &str,
    ) -> Result<()> {
        let user = sqlx::query("SELECT email, display_name FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".into()))?;

        let email: String = user.get("email");
        let display_name: String = user.get("display_name");

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:20px;background-color:#1a1a1f;color:#e1e1e6;border-radius:12px;">
<h1 style="color:#b8a4d4;">NoteFlow</h1>
<p style="line-height:1.5;">Hi {},</p>
{}
<hr style="border:0;border-top:1px solid #3a3a44;margin:20px 0;">
<p style="font-size:12px;color:#71717a;">Sent by NoteFlow</p>
</body>
</html>"#,
            display_name, body_html,
        );

        self.send_email(&email, subject, &html).await
    }

    // ── Convenience ──

    #[allow(unused)]
    pub async fn notify_note_updated(
        &self,
        note_id: Uuid,
        note_title: &str,
        updater_name: &str,
        owner_id: Uuid,
    ) -> Result<()> {
        let body = format!("{} edited \"{}\"", updater_name, note_title);
        let data = json!({ "note_id": note_id, "type": "note_updated" });
        self.send_push_to_user(owner_id, "NoteFlow", &body, Some(data))
            .await
            .ok();

        let email_body = format!(
            "<p>{} edited your note <strong>\"{}\"</strong>.</p>
             <p><a href=\"{}/notes/{}\" style=\"color:#b8a4d4;\">Open in NoteFlow</a></p>",
            updater_name,
            note_title,
            self.config.app_url.trim_end_matches('/'),
            note_id,
        );
        self.send_notification_email(
            owner_id,
            &format!("{} edited \"{}\"", updater_name, note_title),
            &email_body,
        )
        .await
        .ok();
        Ok(())
    }

    #[allow(unused)]
    pub async fn notify_password_changed(&self, user_id: Uuid) -> Result<()> {
        self.send_notification_email(
            user_id,
            "Password Changed",
            "<p>Your NoteFlow password was successfully changed.</p>
             <p>If you did not make this change, please reset your password immediately.</p>",
        )
        .await
        .ok();
        Ok(())
    }
}
