use crate::models::revision::*;
use crate::utils::errors::{AppError, Result};
use sqlx::PgPool;
use uuid::Uuid;

pub struct RevisionService {
    pool: PgPool,
}

impl RevisionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        note_id: Uuid,
        user_id: Uuid,
        page: i64,
        limit: i64,
    ) -> Result<RevisionListResponse> {
        let offset = (page.max(1) - 1) * limit.min(100);

        let revisions = sqlx::query_as::<_, Revision>(
            r#"
            SELECT r.id, r.note_id, r.content, r.created_by, r.created_at
            FROM revisions r
            INNER JOIN notes n ON n.id = r.note_id
            WHERE r.note_id = $1 AND n.user_id = $2 AND n.is_deleted = false
            ORDER BY r.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(note_id)
        .bind(user_id)
        .bind(limit.min(100))
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count
            FROM revisions r
            INNER JOIN notes n ON n.id = r.note_id
            WHERE r.note_id = $1 AND n.user_id = $2 AND n.is_deleted = false
            "#,
            note_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(RevisionListResponse {
            revisions: revisions.into_iter().map(RevisionResponse::from).collect(),
            total,
        })
    }

    pub async fn get(
        &self,
        note_id: Uuid,
        revision_id: Uuid,
        user_id: Uuid,
    ) -> Result<RevisionResponse> {
        let revision = sqlx::query_as::<_, Revision>(
            r#"
            SELECT r.id, r.note_id, r.content, r.created_by, r.created_at
            FROM revisions r
            INNER JOIN notes n ON n.id = r.note_id
            WHERE r.id = $1 AND r.note_id = $2 AND n.user_id = $3 AND n.is_deleted = false
            "#,
        )
        .bind(revision_id)
        .bind(note_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Revision not found".to_string()))?;

        Ok(RevisionResponse::from(revision))
    }

    pub async fn restore(
        &self,
        note_id: Uuid,
        revision_id: Uuid,
        user_id: Uuid,
    ) -> Result<()> {
        let revision = sqlx::query_as::<_, Revision>(
            r#"
            SELECT r.id, r.note_id, r.content, r.created_by, r.created_at
            FROM revisions r
            INNER JOIN notes n ON n.id = r.note_id
            WHERE r.id = $1 AND r.note_id = $2 AND n.user_id = $3 AND n.is_deleted = false
            "#,
        )
        .bind(revision_id)
        .bind(note_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Revision not found".to_string()))?;

        sqlx::query!(
            r#"
            UPDATE notes
            SET content = $1, last_edited_by = $2, updated_at = NOW()
            WHERE id = $3 AND user_id = $4 AND is_deleted = false
            "#,
            revision.content,
            user_id,
            note_id,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "Revision {} restored for note {} by user {}",
            revision_id,
            note_id,
            user_id
        );
        Ok(())
    }
}
