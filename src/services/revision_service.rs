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

    async fn verify_note_access(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let is_owner = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)",
            note_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);

        if is_owner {
            return Ok(());
        }

        let collab_exists: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM note_collaborators WHERE note_id = $1 AND user_id = $2)",
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !collab_exists.unwrap_or(false) {
            return Err(AppError::NotFound("Note not found or access denied".into()));
        }
        Ok(())
    }

    async fn verify_write_access(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let is_owner = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)",
            note_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);

        if is_owner {
            return Ok(());
        }

        let permission: Option<String> = sqlx::query_scalar(
            "SELECT permission FROM note_collaborators WHERE note_id = $1 AND user_id = $2 AND permission IN ('write', 'admin')",
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match permission {
            Some(_) => Ok(()),
            None => Err(AppError::Forbidden("Not authorized to modify this note".into())),
        }
    }

    pub async fn list(
        &self,
        note_id: Uuid,
        user_id: Uuid,
        page: i64,
        limit: i64,
    ) -> Result<RevisionListResponse> {
        self.verify_note_access(note_id, user_id).await?;

        let offset = (page.max(1) - 1) * limit.min(100);

        let revisions = sqlx::query_as::<_, Revision>(
            r#"
            SELECT r.id, r.note_id, r.content, r.created_by, r.created_at
            FROM revisions r
            WHERE r.note_id = $1
            ORDER BY r.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(note_id)
        .bind(limit.min(100))
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count
            FROM revisions r
            WHERE r.note_id = $1
            "#,
            note_id
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
        self.verify_note_access(note_id, user_id).await?;

        let revision = sqlx::query_as::<_, Revision>(
            r#"
            SELECT r.id, r.note_id, r.content, r.created_by, r.created_at
            FROM revisions r
            WHERE r.id = $1 AND r.note_id = $2
            "#,
        )
        .bind(revision_id)
        .bind(note_id)
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
        self.verify_write_access(note_id, user_id).await?;

        let revision = sqlx::query_as::<_, Revision>(
            r#"
            SELECT r.id, r.note_id, r.content, r.created_by, r.created_at
            FROM revisions r
            WHERE r.id = $1 AND r.note_id = $2
            "#,
        )
        .bind(revision_id)
        .bind(note_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Revision not found".to_string()))?;

        sqlx::query!(
            r#"
            UPDATE notes
            SET content = $1, last_edited_by = $2, updated_at = NOW()
            WHERE id = $3 AND is_deleted = false
            "#,
            revision.content,
            user_id,
            note_id,
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
