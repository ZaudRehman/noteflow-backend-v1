use crate::config::Config;
use crate::models::note::*;
use crate::services::{NoteCollaboratorService, NotificationService};
use crate::utils::{
    errors::{AppError, Result},
    validation,
};
use sqlx::PgPool;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct NoteService {
    pool: PgPool,
    config: Config,
    collab_service: Arc<NoteCollaboratorService>,
    notification_service: Arc<NotificationService>,
}

impl NoteService {
    pub fn new(pool: PgPool, config: Config, collab_service: Arc<NoteCollaboratorService>, notification_service: Arc<NotificationService>) -> Self {
        Self { pool, config, collab_service, notification_service }
    }

    pub async fn create(&self, user_id: Uuid, req: CreateNoteRequest) -> Result<NoteResponse> {
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

        let note = sqlx::query!(
            r#"INSERT INTO notes (user_id, title, content, last_edited_by)
               VALUES ($1, $2, '', $1)
               RETURNING id, user_id, title, content, last_edited_by, 
                         is_favorited, is_archived, is_deleted, created_at, updated_at"#,
            user_id,
            title
        )
        .fetch_one(&self.pool)
        .await?;

        if let Some(blocks) = &req.blocks {
            for block in blocks {
                self.insert_block(note.id, block).await?;
            }
            self.rebuild_content(note.id).await?;
        }

        self.get(note.id, user_id).await
    }

    pub async fn get(&self, note_id: Uuid, user_id: Uuid) -> Result<NoteResponse> {
        let permission = self.collab_service.verify_note_access(note_id, user_id).await?;

        let note = sqlx::query!(
            r#"SELECT id, user_id, title, content, last_edited_by, 
                      is_favorited, is_archived, is_deleted, created_at, updated_at 
               FROM notes 
               WHERE id = $1 AND is_deleted = false"#,
            note_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Note not found".to_string()))?;

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

        let active_users = self.get_active_users(note_id).await?;
        let collaborators = self.collab_service.get_note_collaborators(note_id).await?;
        let blocks = self.get_note_blocks(note_id).await?;

        Ok(NoteResponse {
            id: note.id,
            user_id: note.user_id,
            title: note.title,
            content: note.content,
            last_edited_by: note.last_edited_by,
            is_favorited: note.is_favorited,
            is_archived: note.is_archived,
            created_at: note.created_at,
            updated_at: note.updated_at,
            tags,
            active_users,
            collaborators,
            permission,
            blocks,
        })
    }

    pub async fn list(&self, user_id: Uuid, params: NoteQueryParams) -> Result<NoteListResponse> {
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        let rows = sqlx::query(
            r#"SELECT n.id, n.user_id, n.title, n.content, n.last_edited_by, 
                      n.is_favorited, n.is_archived, n.created_at, n.updated_at,
                      nc.permission
               FROM notes n
               LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1
               WHERE (n.user_id = $1 OR nc.user_id IS NOT NULL) AND n.is_deleted = false
               ORDER BY n.updated_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        let total: Option<i64> = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM notes n
               LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1
               WHERE (n.user_id = $1 OR nc.user_id IS NOT NULL) AND n.is_deleted = false"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let total = total.unwrap_or(0);
        self.build_note_responses(rows, user_id).await
            .map(|notes| NoteListResponse { notes, total, page, limit })
    }

    async fn build_note_responses(&self, rows: Vec<sqlx::postgres::PgRow>, user_id: Uuid) -> Result<Vec<NoteResponse>> {
        if rows.is_empty() {
            return Ok(vec![]);
        }

        let note_ids: Vec<Uuid> = rows.iter().map(|r| r.try_get::<Uuid, _>("id").unwrap()).collect();

        let tags_map = self.batch_get_tags_map(&note_ids).await?;
        let active_users_map = self.batch_get_active_users_map(&note_ids).await?;
        let collab_map = self.collab_service.batch_get_note_collaborators(&note_ids).await?;
        let blocks_map = self.batch_get_blocks_map(&note_ids).await?;

        let mut responses = Vec::with_capacity(rows.len());
        for row in rows {
            let note_id: Uuid = row.try_get("id")?;
            let note_user_id: Uuid = row.try_get("user_id")?;

            let collab_perm: Option<String> = row.try_get("permission").unwrap_or(None);
            let permission = if note_user_id == user_id {
                "owner".to_string()
            } else {
                collab_perm.unwrap_or_else(|| "read".into())
            };

            responses.push(NoteResponse {
                id: note_id,
                user_id: note_user_id,
                title: row.try_get("title")?,
                content: row.try_get("content")?,
                last_edited_by: row.try_get("last_edited_by")?,
                is_favorited: row.try_get("is_favorited")?,
                is_archived: row.try_get("is_archived")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                tags: tags_map.get(&note_id).cloned().unwrap_or_default(),
                active_users: active_users_map.get(&note_id).cloned().unwrap_or_default(),
                collaborators: collab_map.get(&note_id).cloned().unwrap_or_default(),
                permission,
                blocks: blocks_map.get(&note_id).cloned().unwrap_or_default(),
            });
        }
        Ok(responses)
    }

    pub async fn update(
        &self,
        note_id: Uuid,
        user_id: Uuid,
        req: UpdateNoteRequest,
    ) -> Result<NoteResponse> {
        let permission = self.collab_service.verify_note_access(note_id, user_id).await?;
        if permission != "owner" && permission != "write" && permission != "admin" {
            return Err(AppError::Forbidden("Not authorized to edit this note".to_string()));
        }

        let note = sqlx::query!(
            r#"SELECT id, user_id, title, content, last_edited_by, 
                      is_favorited, is_archived, is_deleted, created_at, updated_at
               FROM notes 
               WHERE id = $1 AND is_deleted = false"#,
            note_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Note not found".to_string()))?;

        let title = match req.title {
            Some(ref t) => {
                let t = validation::sanitize_string(t);
                validation::validate_note_title(&t)?;
                t
            }
            None => note.title.clone(),
        };

        if let Some(blocks) = &req.blocks {
            self.sync_blocks(note_id, blocks).await?;
            self.rebuild_content(note_id).await?;
        }

        sqlx::query!(
            r#"UPDATE notes 
               SET title = $1, last_edited_by = $2, updated_at = NOW()
               WHERE id = $3"#,
            title,
            user_id,
            note_id
        )
        .execute(&self.pool)
        .await?;

        let updater_name: Option<String> = sqlx::query_scalar(
            "SELECT display_name FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        if let Some(ref name) = updater_name {
            self.notification_service
                .notify_note_updated(note_id, &title, name, note.user_id)
                .await
                .ok();
        }

        self.get(note_id, user_id).await
    }

    pub async fn delete(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let note = sqlx::query!("SELECT user_id FROM notes WHERE id = $1", note_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Note not found".to_string()))?;

        if note.user_id != user_id {
            return Err(AppError::Forbidden("Only the owner can delete a note".to_string()));
        }

        sqlx::query!("UPDATE notes SET is_deleted = true WHERE id = $1", note_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn toggle_favorite(&self, note_id: Uuid, user_id: Uuid) -> Result<NoteResponse> {
        self.verify_note_ownership(note_id, user_id).await?;

        sqlx::query!(
            "UPDATE notes SET is_favorited = NOT is_favorited, updated_at = NOW() WHERE id = $1",
            note_id
        )
        .execute(&self.pool)
        .await?;

        self.get(note_id, user_id).await
    }

    pub async fn toggle_archive(&self, note_id: Uuid, user_id: Uuid) -> Result<NoteResponse> {
        self.verify_note_ownership(note_id, user_id).await?;

        sqlx::query!(
            "UPDATE notes SET is_archived = NOT is_archived, updated_at = NOW() WHERE id = $1",
            note_id
        )
        .execute(&self.pool)
        .await?;

        self.get(note_id, user_id).await
    }

    pub async fn list_filtered(
        &self,
        user_id: Uuid,
        params: NoteFilterParams,
    ) -> Result<NoteListResponse> {
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        let sort_by = params.sort_by.as_deref().unwrap_or("updated_at");
        let sort_order = params.sort_order.as_deref().unwrap_or("DESC");

        let valid_sort_fields = ["created_at", "updated_at", "title"];
        if !valid_sort_fields.contains(&sort_by) {
            return Err(AppError::ValidationError("Invalid sort field".into()));
        }

        if sort_order != "ASC" && sort_order != "DESC" {
            return Err(AppError::ValidationError(
                "Sort order must be ASC or DESC".into(),
            ));
        }

        let (filter_clause, _is_archived_filter) = match params.filter.as_deref() {
            Some("favorites") => ("AND n.is_favorited = true AND n.is_archived = false", false),
            Some("archived") => ("AND n.is_archived = true", true),
            _ => ("AND n.is_archived = false", false),
        };

        let (tag_join, tag_filter) = if params.tag_id.is_some() {
            ("INNER JOIN note_tags nt ON n.id = nt.note_id", " AND nt.tag_id = $4")
        } else {
            ("", "")
        };

        let query_str = format!(
            r#"
            SELECT
                n.id, n.user_id, n.title, n.content, n.last_edited_by,
                n.is_favorited, n.is_archived,
                n.created_at, n.updated_at,
                nc.permission
            FROM notes n
            {}
            LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1
            WHERE (n.user_id = $1 OR nc.user_id IS NOT NULL) AND n.is_deleted = false {} {}
            ORDER BY n.{} {}
            LIMIT $2 OFFSET $3
            "#,
            tag_join, filter_clause, tag_filter, sort_by, sort_order
        );

        let mut query = sqlx::query(&query_str)
            .bind(user_id)
            .bind(limit as i64)
            .bind(offset as i64);

        if let Some(tag_id) = params.tag_id {
            query = query.bind(tag_id);
        }

        let notes = query.fetch_all(&self.pool).await?;

        let count_query = format!(
            r#"SELECT COUNT(*) as count FROM notes n {} LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1 WHERE (n.user_id = $1 OR nc.user_id IS NOT NULL) AND n.is_deleted = false {} {}"#,
            tag_join, filter_clause, tag_filter
        );

        let mut count_bind = sqlx::query_scalar(&count_query)
            .bind(user_id);

        if let Some(tag_id) = params.tag_id {
            count_bind = count_bind.bind(tag_id);
        }

        let total: i64 = count_bind.fetch_one(&self.pool).await?;

        self.build_note_responses(notes, user_id).await
            .map(|notes| NoteListResponse { notes, total, page, limit })
    }

    pub async fn search(&self, user_id: Uuid, params: SearchParams) -> Result<SearchResponse> {
        let query = validation::sanitize_string(&params.q).trim().to_string();

        if query.is_empty() {
            return Ok(SearchResponse {
                notes: vec![],
                total: 0,
                query,
            });
        }

        let limit = params.limit.unwrap_or(50).min(100) as i64;
        let like_pattern = format!("%{}%", query);

        let notes = sqlx::query(
            r#"
            SELECT
                n.id, n.user_id, n.title, n.content, n.last_edited_by,
                n.is_favorited, n.is_archived, n.created_at, n.updated_at,
                nc.permission
            FROM notes n
            LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1
            WHERE (n.user_id = $1 OR nc.user_id IS NOT NULL)
                AND n.is_deleted = false
                AND (
                    to_tsvector('english', n.title || ' ' || n.content) @@ plainto_tsquery('english', $2)
                    OR n.title ILIKE $3
                    OR n.content ILIKE $3
                )
            ORDER BY
                CASE
                    WHEN n.title ILIKE $3 THEN 1
                    WHEN n.content ILIKE $3 THEN 2
                    ELSE 3
                END,
                ts_rank(to_tsvector('english', n.title || ' ' || n.content), plainto_tsquery('english', $2)) DESC,
                n.updated_at DESC
            LIMIT $4
            "#,
        )
        .bind(user_id)
        .bind(&query)
        .bind(&like_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM notes n
            LEFT JOIN note_collaborators nc ON n.id = nc.note_id AND nc.user_id = $1
            WHERE (n.user_id = $1 OR nc.user_id IS NOT NULL)
                AND n.is_deleted = false
                AND (
                    to_tsvector('english', n.title || ' ' || n.content) @@ plainto_tsquery('english', $2)
                    OR n.title ILIKE $3
                    OR n.content ILIKE $3
                )
            "#,
        )
        .bind(user_id)
        .bind(&query)
        .bind(&like_pattern)
        .fetch_one(&self.pool)
        .await?;

        self.build_note_responses(notes, user_id).await
            .map(|notes| SearchResponse { notes, total, query })
    }

    // ── Block operations ──

    async fn insert_block(&self, note_id: Uuid, block: &CreateBlockRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO note_blocks (note_id, block_type, data, position, parent_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(note_id)
        .bind(&block.block_type)
        .bind(&block.data)
        .bind(block.position)
        .bind(block.parent_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_note_blocks(&self, note_id: Uuid) -> Result<Vec<BlockData>> {
        let rows = sqlx::query_as::<_, crate::models::block::Block>(
            r#"
            SELECT id, note_id, block_type, data, position, parent_id, created_at, updated_at
            FROM note_blocks
            WHERE note_id = $1
            ORDER BY position ASC, created_at ASC
            "#,
        )
        .bind(note_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(BlockData::from).collect())
    }

    async fn batch_get_blocks_map(&self, note_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<BlockData>>> {
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

        let mut map: HashMap<Uuid, Vec<BlockData>> = HashMap::new();
        for row in rows {
            let nid = row.note_id;
            map.entry(nid).or_default().push(BlockData::from(row));
        }
        Ok(map)
    }

    async fn sync_blocks(&self, note_id: Uuid, blocks: &[UpdateBlockRequest]) -> Result<()> {
        for block in blocks {
            if let Some(block_type) = &block.block_type {
                sqlx::query(
                    r#"
                    INSERT INTO note_blocks (note_id, block_type, data, position, parent_id)
                    VALUES ($1, $2, '{}'::jsonb, 0, NULL)
                    RETURNING id
                    "#,
                )
                .bind(note_id)
                .bind(block_type)
                .execute(&self.pool)
                .await?;
            } else {
                let mut set_clauses = Vec::new();
                let mut param_count = 0;

                if block.data.is_some() {
                    param_count += 1;
                    set_clauses.push(format!("data = ${}", param_count + 1));
                }
                if block.position.is_some() {
                    param_count += 1;
                    set_clauses.push(format!("position = ${}", param_count + 1));
                }
                if block.parent_id.is_some() {
                    param_count += 1;
                    set_clauses.push(format!("parent_id = ${}", param_count + 1));
                }

                if set_clauses.is_empty() {
                    continue;
                }

                let sql = format!(
                    "UPDATE note_blocks SET {} WHERE id = $1 AND note_id = $2",
                    set_clauses.join(", ")
                );

                let mut q = sqlx::query(&sql).bind(block.id).bind(note_id);

                if let Some(ref data) = block.data {
                    q = q.bind(data);
                }
                if let Some(pos) = block.position {
                    q = q.bind(pos);
                }
                if let Some(pid) = block.parent_id {
                    q = q.bind(pid);
                }

                q.execute(&self.pool).await?;
            }
        }
        Ok(())
    }

    async fn rebuild_content(&self, note_id: Uuid) -> Result<()> {
        let blocks = sqlx::query_as::<_, crate::models::block::Block>(
            r#"
            SELECT id, note_id, block_type, data, position, parent_id, created_at, updated_at
            FROM note_blocks
            WHERE note_id = $1
            ORDER BY position ASC, created_at ASC
            "#,
        )
        .bind(note_id)
        .fetch_all(&self.pool)
        .await?;

        let plain_text = blocks_to_plain_text(&blocks);

        sqlx::query!(
            "UPDATE notes SET content = $1, updated_at = NOW() WHERE id = $2",
            plain_text,
            note_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ── Helpers ──

    async fn batch_get_tags_map(&self, note_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<String>>> {
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

    async fn batch_get_active_users_map(&self, note_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<ActiveUserInfo>>> {
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

        let mut map: HashMap<Uuid, Vec<ActiveUserInfo>> = HashMap::new();
        for row in rows {
            let nid: Uuid = row.try_get("note_id")?;
            let entry = map.entry(nid).or_default();
            entry.push(ActiveUserInfo {
                user_id: row.try_get("user_id")?,
                display_name: row.try_get("display_name")?,
                cursor_line: row.try_get("cursor_line").unwrap_or(0),
                cursor_column: row.try_get("cursor_column").unwrap_or(0),
            });
        }
        Ok(map)
    }

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
                cursor_line: u.cursor_line.unwrap_or(0),
                cursor_column: u.cursor_column.unwrap_or(0),
            })
            .collect())
    }

    pub async fn get_all_blocks(&self, note_id: Uuid) -> Result<Vec<crate::models::block::Block>> {
        let blocks = sqlx::query_as::<_, crate::models::block::Block>(
            r#"
            SELECT id, note_id, block_type, data, position, parent_id, created_at, updated_at
            FROM note_blocks
            WHERE note_id = $1
            ORDER BY position ASC, created_at ASC
            "#,
        )
        .bind(note_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(blocks)
    }
}

/// Convert blocks to plain text for full-text search indexing
pub fn blocks_to_plain_text(blocks: &[crate::models::block::Block]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        match block.block_type.as_str() {
            "heading" | "paragraph" => {
                if let Some(text) = block.data.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                }
            }
            "code" => {
                if let Some(text) = block.data.get("code").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                }
            }
            "table" => {
                if let Some(rows) = block.data.get("rows").and_then(|v| v.as_array()) {
                    for row in rows {
                        if let Some(cells) = row.as_array() {
                            for cell in cells {
                                if let Some(s) = cell.as_str() {
                                    parts.push(s.to_string());
                                }
                            }
                        }
                    }
                }
            }
            "chart" => {
                if let Some(title) = block.data.get("title").and_then(|v| v.as_str()) {
                    parts.push(title.to_string());
                }
                if let Some(labels) = block.data.get("labels").and_then(|v| v.as_array()) {
                    for label in labels {
                        if let Some(s) = label.as_str() {
                            parts.push(s.to_string());
                        }
                    }
                }
            }
            "quote" => {
                if let Some(text) = block.data.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                }
            }
            "bullet_list" | "numbered_list" | "todo_list" => {
                if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                            parts.push(text.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    parts.join("\n")
}
