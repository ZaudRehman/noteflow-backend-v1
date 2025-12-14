// src/services/note_service.rs
use crate::config::Config;
use crate::models::note::*;
use crate::utils::{
    errors::{AppError, Result},
    validation,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct NoteService {
    pool: PgPool,
    config: Config,
}

impl NoteService {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }

    pub async fn create(&self, user_id: Uuid, req: CreateNoteRequest) -> Result<NoteResponse> {
        // Check note limit
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM notes WHERE user_id = $1 AND is_deleted = false",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if count.count.unwrap_or(0) >= self.config.max_notes_per_user {
            return Err(AppError::Forbidden("Note limit reached".to_string()));
        }

        let title = validation::sanitize_string(&req.title);
        validation::validate_note_title(&title)?;

        let content = req.content.unwrap_or_default();
        validation::validate_note_content(&content, self.config.max_note_size)?;

        let note = sqlx::query_as!(
            Note,
            r#"INSERT INTO notes (user_id, title, content, last_edited_by)
               VALUES ($1, $2, $3, $1)
               RETURNING id, user_id, title, content, last_edited_by, is_deleted, created_at, updated_at"#,
            user_id, title, content
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(NoteResponse {
            id: note.id,
            title: note.title,
            content: note.content,
            last_edited_by: note.last_edited_by,
            created_at: note.created_at,
            updated_at: note.updated_at,
            tags: vec![],
        })
    }

    pub async fn get(&self, note_id: Uuid, user_id: Uuid) -> Result<NoteResponse> {
        let note = sqlx::query_as!(
            Note,
            "SELECT * FROM notes WHERE id = $1 AND is_deleted = false",
            note_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Note not found".to_string()))?;

        if note.user_id != user_id {
            return Err(AppError::Forbidden(
                "Not authorized to access this note".to_string(),
            ));
        }

        // Fetch tags
        let tags = sqlx::query!(
            r#"SELECT t.name FROM tags t
               INNER JOIN note_tags nt ON t.id = nt.tag_id
               WHERE nt.note_id = $1"#,
            note_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| r.name)
        .collect();

        Ok(NoteResponse {
            id: note.id,
            title: note.title,
            content: note.content,
            last_edited_by: note.last_edited_by,
            created_at: note.created_at,
            updated_at: note.updated_at,
            tags,
        })
    }

    pub async fn list(&self, user_id: Uuid, params: NoteQueryParams) -> Result<NoteListResponse> {
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        let notes = sqlx::query_as!(
            Note,
            r#"SELECT * FROM notes 
               WHERE user_id = $1 AND is_deleted = false
               ORDER BY updated_at DESC
               LIMIT $2 OFFSET $3"#,
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query!(
            "SELECT COUNT(*) as count FROM notes WHERE user_id = $1 AND is_deleted = false",
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .count
        .unwrap_or(0);

        let mut responses = vec![];
        for note in notes {
            responses.push(NoteResponse {
                id: note.id,
                title: note.title,
                content: note.content,
                last_edited_by: note.last_edited_by,
                created_at: note.created_at,
                updated_at: note.updated_at,
                tags: vec![],
            });
        }

        Ok(NoteListResponse {
            notes: responses,
            total,
        })
    }

    pub async fn update(
        &self,
        note_id: Uuid,
        user_id: Uuid,
        req: UpdateNoteRequest,
    ) -> Result<NoteResponse> {
        let note = sqlx::query_as!(
            Note,
            "SELECT * FROM notes WHERE id = $1 AND is_deleted = false",
            note_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Note not found".to_string()))?;

        if note.user_id != user_id {
            return Err(AppError::Forbidden("Not authorized".to_string()));
        }

        let title = req.title.unwrap_or(note.title);
        let content = req.content.unwrap_or(note.content);

        validation::validate_note_title(&title)?;
        validation::validate_note_content(&content, self.config.max_note_size)?;

        sqlx::query!(
            r#"UPDATE notes 
               SET title = $1, content = $2, last_edited_by = $3, updated_at = NOW()
               WHERE id = $4"#,
            title,
            content,
            user_id,
            note_id
        )
        .execute(&self.pool)
        .await?;

        self.get(note_id, user_id).await
    }

    pub async fn delete(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let note = sqlx::query!("SELECT user_id FROM notes WHERE id = $1", note_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Note not found".to_string()))?;

        if note.user_id != user_id {
            return Err(AppError::Forbidden("Not authorized".to_string()));
        }

        sqlx::query!("UPDATE notes SET is_deleted = true WHERE id = $1", note_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Toggle favorite status
    pub async fn toggle_favorite(
        &self,
        note_id: Uuid,
        user_id: Uuid,
    ) -> Result<NoteResponseEnhanced> {
        // Verify ownership
        self.verify_note_ownership(note_id, user_id).await?;

        sqlx::query!(
            "UPDATE notes SET is_favorited = NOT is_favorited, updated_at = NOW() WHERE id = $1",
            note_id
        )
        .execute(&self.pool)
        .await?;

        self.get_enhanced(note_id, user_id).await
    }

    /// Toggle archive status
    pub async fn toggle_archive(
        &self,
        note_id: Uuid,
        user_id: Uuid,
    ) -> Result<NoteResponseEnhanced> {
        self.verify_note_ownership(note_id, user_id).await?;

        sqlx::query!(
            "UPDATE notes SET is_archived = NOT is_archived, updated_at = NOW() WHERE id = $1",
            note_id
        )
        .execute(&self.pool)
        .await?;

        self.get_enhanced(note_id, user_id).await
    }

    /// List notes with advanced filtering
    pub async fn list_filtered(
        &self,
        user_id: Uuid,
        params: NoteFilterParams,
    ) -> Result<NoteListResponseEnhanced> {
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        let sort_by = params.sort_by.as_deref().unwrap_or("updated_at");
        let sort_order = params.sort_order.as_deref().unwrap_or("DESC");

        // Validate sort fields
        let valid_sort_fields = ["created_at", "updated_at", "title"];
        if !valid_sort_fields.contains(&sort_by) {
            return Err(AppError::ValidationError("Invalid sort field".into()));
        }

        if sort_order != "ASC" && sort_order != "DESC" {
            return Err(AppError::ValidationError(
                "Sort order must be ASC or DESC".into(),
            ));
        }

        // Build WHERE clause based on filter
        let (filter_clause, is_archived_filter) = match params.filter.as_deref() {
            Some("favorites") => ("AND n.is_favorited = true AND n.is_archived = false", false),
            Some("archived") => ("AND n.is_archived = true", true),
            _ => ("AND n.is_archived = false", false), // "all" or no filter
        };

        let query_str = format!(
            r#"
            SELECT
                n.id, n.title, n.content, n.last_edited_by,
                n.is_favorited, n.is_archived,
                n.created_at, n.updated_at
            FROM notes n
            WHERE n.user_id = $1 AND n.is_deleted = false {}
            ORDER BY n.{} {}
            LIMIT $2 OFFSET $3
            "#,
            filter_clause, sort_by, sort_order
        );

        let notes = sqlx::query(&query_str)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let count_query = format!(
            "SELECT COUNT(*) FROM notes n WHERE n.user_id = $1 AND n.is_deleted = false {}",
            filter_clause
        );

        let total: i64 = sqlx::query_scalar(&count_query)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        let mut responses = Vec::new();
        for row in notes {
            let note_id: Uuid = row.try_get("id")?;
            let tags = self.get_note_tags(note_id).await?;
            let active_users = self.get_active_users(note_id).await?;

            responses.push(NoteResponseEnhanced {
                id: note_id,
                title: row.try_get("title")?,
                content: row.try_get("content")?,
                last_edited_by: row.try_get("last_edited_by")?,
                is_favorited: row.try_get("is_favorited")?,
                is_archived: row.try_get("is_archived")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                tags,
                active_users,
            });
        }

        let total_pages = (total as f64 / limit as f64).ceil() as i64;

        Ok(NoteListResponseEnhanced {
            notes: responses,
            total,
            page,
            per_page: limit,
            total_pages,
        })
    }

    /// Search notes with full-text search
    pub async fn search(&self, user_id: Uuid, params: SearchParams) -> Result<SearchResponse> {
        let query = validation::sanitize_string(&params.q).trim().to_string();

        if query.is_empty() {
            return Ok(SearchResponse {
                notes: vec![],
                total: 0,
                query,
            });
        }

        let limit = params.limit.unwrap_or(50).min(100);

        let notes = sqlx::query!(
            r#"
            SELECT
                id, title, content, last_edited_by,
                is_favorited, is_archived, created_at, updated_at
            FROM notes
            WHERE user_id = $1
                AND is_deleted = false
                AND (
                    to_tsvector('english', title || ' ' || content) @@ plainto_tsquery('english', $2)
                    OR title ILIKE $3
                    OR content ILIKE $3
                )
            ORDER BY
                CASE
                    WHEN title ILIKE $3 THEN 1
                    WHEN content ILIKE $3 THEN 2
                    ELSE 3
                END,
                ts_rank(to_tsvector('english', title || ' ' || content), plainto_tsquery('english', $2)) DESC,
                updated_at DESC
            LIMIT $4
            "#,
            user_id,
            query,
            format!("%{}%", query),
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut responses = Vec::new();
        for note in notes {
            let tags = self.get_note_tags(note.id).await?;
            let active_users = self.get_active_users(note.id).await?;

            responses.push(NoteResponseEnhanced {
                id: note.id,
                title: note.title,
                content: note.content,
                last_edited_by: note.last_edited_by,
                is_favorited: note.is_favorited,
                is_archived: note.is_archived,
                created_at: note.created_at,
                updated_at: note.updated_at,
                tags,
                active_users,
            });
        }

        let total = responses.len() as i64;

        Ok(SearchResponse {
            notes: responses,
            total,
            query,
        })
    }

    /// Get enhanced note response with tags and active users
    pub async fn get_enhanced(&self, note_id: Uuid, user_id: Uuid) -> Result<NoteResponseEnhanced> {
        let note = sqlx::query!(
            r#"
            SELECT
                id, title, content, last_edited_by,
                is_favorited, is_archived, created_at, updated_at
            FROM notes
            WHERE id = $1 AND is_deleted = false
            "#,
            note_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Note not found".into()))?;

        if note.last_edited_by != user_id {
            // Check if user has access (for future collaboration features)
            self.verify_note_ownership(note_id, user_id).await?;
        }

        let tags = self.get_note_tags(note_id).await?;
        let active_users = self.get_active_users(note_id).await?;

        Ok(NoteResponseEnhanced {
            id: note.id,
            title: note.title,
            content: note.content,
            last_edited_by: note.last_edited_by,
            is_favorited: note.is_favorited,
            is_archived: note.is_archived,
            created_at: note.created_at,
            updated_at: note.updated_at,
            tags,
            active_users,
        })
    }

    // Helper methods

    async fn verify_note_ownership(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)",
            note_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);

        if !exists {
            return Err(AppError::NotFound("Note not found or access denied".into()));
        }
        Ok(())
    }

    async fn get_note_tags(&self, note_id: Uuid) -> Result<Vec<String>> {
        let tags = sqlx::query_scalar!(
            r#"
            SELECT t.name
            FROM tags t
            INNER JOIN note_tags nt ON t.id = nt.tag_id
            WHERE nt.note_id = $1
            ORDER BY t.name
            "#,
            note_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tags)
    }

    async fn get_active_users(&self, note_id: Uuid) -> Result<Vec<ActiveUserInfo>> {
        let users = sqlx::query!(
            r#"
            SELECT
                s.user_id,
                u.display_name,
                s.cursor_line,
                s.cursor_column
            FROM active_sessions s
            INNER JOIN users u ON s.user_id = u.id
            WHERE s.note_id = $1
                AND s.last_seen_at > NOW() - INTERVAL '2 minutes'
            ORDER BY s.last_seen_at DESC
            "#,
            note_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users
            .into_iter()
            .map(|u| ActiveUserInfo {
                user_id: u.user_id,
                display_name: u.display_name,
                cursor_line: u.cursor_line,
                cursor_column: u.cursor_column,
            })
            .collect())
    }
}
