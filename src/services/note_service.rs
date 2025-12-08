use sqlx::PgPool;
use uuid::Uuid;
use crate::models::note::*;
use crate::utils::{errors::{AppError, Result}, validation};
use crate::config::Config;

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
            return Err(AppError::Forbidden("Not authorized to access this note".to_string()));
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
            user_id, limit, offset
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
    
    pub async fn update(&self, note_id: Uuid, user_id: Uuid, req: UpdateNoteRequest) -> Result<NoteResponse> {
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
            title, content, user_id, note_id
        )
        .execute(&self.pool)
        .await?;
        
        self.get(note_id, user_id).await
    }
    
    pub async fn delete(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let note = sqlx::query!(
            "SELECT user_id FROM notes WHERE id = $1",
            note_id
        )
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
}