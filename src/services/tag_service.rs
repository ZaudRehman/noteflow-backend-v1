// src/services/tag_service.rs
use crate::models::{
    note::{NoteListResponse, NoteResponse},
    tag::*,
};
use crate::services::NoteCollaboratorService;
use crate::utils::{
    errors::{AppError, Result},
    validation,
};
use sqlx::PgPool;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct NoteTagRow {
    id: Uuid,
    user_id: Uuid,
    title: String,
    content: String,
    last_edited_by: Option<Uuid>,
    is_favorited: bool,
    is_archived: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    permission: Option<String>,
}

pub struct TagService {
    pool: PgPool,
    collab_service: Arc<NoteCollaboratorService>,
}

impl TagService {
    pub fn new(pool: PgPool, collab_service: Arc<NoteCollaboratorService>) -> Self {
        Self { pool, collab_service }
    }

    /// List all tags for a user with note counts
    pub async fn list(&self, user_id: Uuid) -> Result<TagListResponse> {
        let tags = sqlx::query_as!(
            TagWithCount,
            r#"
            SELECT 
                t.id, 
                t.name, 
                t.created_at, 
                COUNT(DISTINCT nt.note_id) as "note_count!"
            FROM tags t
            LEFT JOIN note_tags nt ON t.id = nt.tag_id
            LEFT JOIN notes n ON nt.note_id = n.id AND n.is_deleted = false
            WHERE t.user_id = $1
            GROUP BY t.id, t.name, t.created_at
            ORDER BY t.name ASC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let total = tags.len() as i64;
        let tag_responses: Vec<TagResponse> = tags.into_iter().map(|t| t.into()).collect();

        Ok(TagListResponse {
            tags: tag_responses,
            total,
        })
    }

    /// Get a single tag by ID
    pub async fn get(&self, tag_id: Uuid, user_id: Uuid) -> Result<TagResponse> {
        let tag = sqlx::query_as!(
            Tag,
            "SELECT id, user_id, name, created_at FROM tags WHERE id = $1 AND user_id = $2",
            tag_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Tag not found or access denied".into()))?;

        let count = self.get_note_count(tag.id).await?;

        Ok(TagResponse {
            id: tag.id,
            name: tag.name,
            note_count: count,
            created_at: tag.created_at,
        })
    }

    /// Create a new tag
    pub async fn create(&self, user_id: Uuid, req: CreateTagRequest) -> Result<TagResponse> {
        let name = validation::sanitize_string(&req.name).trim().to_lowercase();

        // Validate tag name
        if name.is_empty() {
            return Err(AppError::ValidationError("Tag name cannot be empty".into()));
        }

        if name.len() > 50 {
            return Err(AppError::ValidationError(
                "Tag name must be 50 characters or less".into(),
            ));
        }

        // Check for invalid characters
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
        {
            return Err(AppError::ValidationError(
                "Tag name can only contain letters, numbers, hyphens, underscores, and spaces"
                    .into(),
            ));
        }

        let tag = sqlx::query_as!(
            Tag,
            r#"
            INSERT INTO tags (user_id, name)
            VALUES ($1, $2)
            RETURNING id, user_id, name, created_at
            "#,
            user_id,
            name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") || e.to_string().contains("unique") {
                AppError::Conflict("A tag with this name already exists".into())
            } else {
                AppError::from(e)
            }
        })?;

        Ok(TagResponse {
            id: tag.id,
            name: tag.name,
            note_count: 0,
            created_at: tag.created_at,
        })
    }

    /// Update an existing tag
    pub async fn update(
        &self,
        tag_id: Uuid,
        user_id: Uuid,
        req: UpdateTagRequest,
    ) -> Result<TagResponse> {
        let name = validation::sanitize_string(&req.name).trim().to_lowercase();

        if name.is_empty() || name.len() > 50 {
            return Err(AppError::ValidationError(
                "Tag name must be 1-50 characters".into(),
            ));
        }

        let updated = sqlx::query_as!(
            Tag,
            r#"
            UPDATE tags 
            SET name = $1 
            WHERE id = $2 AND user_id = $3
            RETURNING id, user_id, name, created_at
            "#,
            name,
            tag_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("A tag with this name already exists".into())
            } else {
                AppError::from(e)
            }
        })?
        .ok_or_else(|| AppError::NotFound("Tag not found or access denied".into()))?;

        let count = self.get_note_count(tag_id).await?;

        Ok(TagResponse {
            id: updated.id,
            name: updated.name,
            note_count: count,
            created_at: updated.created_at,
        })
    }

    /// Delete a tag (cascade removes from note_tags)
    pub async fn delete(&self, tag_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            "DELETE FROM tags WHERE id = $1 AND user_id = $2",
            tag_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Tag not found or access denied".into()));
        }

        tracing::info!("Tag {} deleted by user {}", tag_id, user_id);
        Ok(())
    }

    /// Add a tag to a note
    pub async fn add_to_note(&self, note_id: Uuid, tag_id: Uuid, user_id: Uuid) -> Result<()> {
        // Verify note ownership
        self.verify_note_access(note_id, user_id).await?;

        // Verify tag ownership
        self.verify_tag_access(tag_id, user_id).await?;

        sqlx::query!(
            r#"
            INSERT INTO note_tags (note_id, tag_id) 
            VALUES ($1, $2) 
            ON CONFLICT (note_id, tag_id) DO NOTHING
            "#,
            note_id,
            tag_id
        )
        .execute(&self.pool)
        .await?;

        tracing::debug!(
            "Tag {} added to note {} by user {}",
            tag_id,
            note_id,
            user_id
        );
        Ok(())
    }

    /// Remove a tag from a note
    pub async fn remove_from_note(&self, note_id: Uuid, tag_id: Uuid, user_id: Uuid) -> Result<()> {
        // Verify note ownership
        self.verify_note_access(note_id, user_id).await?;

        let result = sqlx::query!(
            "DELETE FROM note_tags WHERE note_id = $1 AND tag_id = $2",
            note_id,
            tag_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Tag not associated with this note".into(),
            ));
        }

        tracing::debug!(
            "Tag {} removed from note {} by user {}",
            tag_id,
            note_id,
            user_id
        );
        Ok(())
    }

    /// Get all notes with a specific tag
    pub async fn get_notes_by_tag(
        &self,
        tag_id: Uuid,
        user_id: Uuid,
        page: i64,
        limit: i64,
    ) -> Result<NoteListResponse> {
        self.verify_tag_access(tag_id, user_id).await?;

        let offset = (page - 1) * limit;

        let notes = sqlx::query_as::<_, NoteTagRow>(
            r#"
            SELECT 
                n.id, n.user_id, n.title, n.content, n.last_edited_by,
                n.is_favorited, n.is_archived, n.created_at, n.updated_at,
                nc.permission
            FROM notes n
            INNER JOIN note_tags nt ON n.id = nt.note_id
            LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1
            WHERE nt.tag_id = $2 
                AND (n.user_id = $1 OR nc.user_id IS NOT NULL)
                AND n.is_deleted = false
            ORDER BY n.updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(tag_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM notes n 
            INNER JOIN note_tags nt ON n.id = nt.note_id
            LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1
            WHERE nt.tag_id = $2 
                AND (n.user_id = $1 OR nc.user_id IS NOT NULL)
                AND n.is_deleted = false
            "#,
        )
        .bind(user_id)
        .bind(tag_id)
        .fetch_one(&self.pool)
        .await?;

        let note_ids: Vec<Uuid> = notes.iter().map(|n| n.id).collect();
        let tags_map = self.batch_get_notes_tags(&note_ids).await?;
        let active_users_map = self.batch_get_active_users_map(&note_ids).await?;
        let collab_map = self.collab_service.batch_get_note_collaborators(&note_ids).await?;
        let blocks_map = self.batch_get_blocks_map(&note_ids).await?;

        let mut responses = Vec::with_capacity(notes.len());
        for note in notes {
            let permission = if note.user_id == user_id {
                "owner".to_string()
            } else {
                note.permission.clone().unwrap_or_else(|| "read".into())
            };

            responses.push(NoteResponse {
                id: note.id,
                user_id: note.user_id,
                title: note.title,
                content: note.content,
                last_edited_by: note.last_edited_by,
                is_favorited: note.is_favorited,
                is_archived: note.is_archived,
                created_at: note.created_at,
                updated_at: note.updated_at,
                tags: tags_map.get(&note.id).cloned().unwrap_or_default(),
                active_users: active_users_map.get(&note.id).cloned().unwrap_or_default(),
                collaborators: collab_map.get(&note.id).cloned().unwrap_or_default(),
                permission,
                blocks: blocks_map.get(&note.id).cloned().unwrap_or_default(),
            });
        }

        Ok(NoteListResponse {
            notes: responses,
            total: total.unwrap_or(0),
            page,
            limit,
        })
    }

    // Helper methods

    async fn batch_get_notes_tags(&self, note_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<String>>> {
        if note_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            r#"
            SELECT nt.note_id, t.name
            FROM note_tags nt
            INNER JOIN tags t ON t.id = nt.tag_id
            WHERE nt.note_id = ANY($1)
            ORDER BY t.name
            "#,
        )
        .bind(note_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
        for row in rows {
            let nid: Uuid = row.try_get("note_id")?;
            let name: String = row.try_get("name")?;
            map.entry(nid).or_default().push(name);
        }
        Ok(map)
    }

    async fn batch_get_active_users_map(&self, note_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<crate::models::note::ActiveUserInfo>>> {
        if note_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            r#"
            SELECT s.note_id, s.user_id, u.display_name, s.cursor_line, s.cursor_column
            FROM active_sessions s
            INNER JOIN users u ON u.id = s.user_id
            WHERE s.note_id = ANY($1)
                AND s.last_seen_at > NOW() - INTERVAL '2 minutes'
            ORDER BY s.last_seen_at DESC
            "#,
        )
        .bind(note_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: HashMap<Uuid, Vec<crate::models::note::ActiveUserInfo>> = HashMap::new();
        for row in rows {
            let nid: Uuid = row.try_get("note_id")?;
            let entry = map.entry(nid).or_default();
            entry.push(crate::models::note::ActiveUserInfo {
                user_id: row.try_get("user_id")?,
                display_name: row.try_get("display_name")?,
                cursor_line: row.try_get("cursor_line").unwrap_or(0),
                cursor_column: row.try_get("cursor_column").unwrap_or(0),
            });
        }
        Ok(map)
    }

    async fn batch_get_blocks_map(&self, note_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<crate::models::note::BlockData>>> {
        if note_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query_as::<_, crate::models::block::Block>(
            r#"
            SELECT id, note_id, block_type, data, position, parent_id, created_at, updated_at
            FROM note_blocks
            WHERE note_id = ANY($1)
            ORDER BY note_id, position ASC, created_at ASC
            "#,
        )
        .bind(note_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: HashMap<Uuid, Vec<crate::models::note::BlockData>> = HashMap::new();
        for row in rows {
            let nid = row.note_id;
            map.entry(nid).or_default().push(crate::models::note::BlockData::from(row));
        }
        Ok(map)
    }

    async fn verify_note_access(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let is_owner = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)",
            note_id, 
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if is_owner.unwrap_or(false) {
            return Ok(());
        }

        let is_collaborator: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM note_collaborators WHERE note_id = $1 AND user_id = $2)",
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !is_collaborator.unwrap_or(false) {
            return Err(AppError::NotFound("Note not found or access denied".into()));
        }
        Ok(())
    }

    async fn verify_tag_access(&self, tag_id: Uuid, user_id: Uuid) -> Result<()> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1 AND user_id = $2)",
            tag_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if !exists.unwrap_or(false) {
            return Err(AppError::NotFound("Tag not found or access denied".into()));
        }
        Ok(())
    }

    async fn get_note_count(&self, tag_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM note_tags nt INNER JOIN notes n ON nt.note_id = n.id WHERE nt.tag_id = $1 AND n.is_deleted = false",
            tag_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(count)
    }

}
