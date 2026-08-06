use crate::config::Config;
use crate::models::note::{AddCollaboratorRequest, CollaboratorInfo, CollaboratorListResponse, UpdateCollaboratorRequest};
use crate::utils::errors::{AppError, Result};
use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct NoteCollaboratorService {
    pool: PgPool,
    config: Config,
}

impl NoteCollaboratorService {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }

    pub async fn list(&self, note_id: Uuid, user_id: Uuid) -> Result<CollaboratorListResponse> {
        let permission = self.get_user_permission(note_id, user_id).await?;
        if permission != "owner" && permission != "admin" {
            return Err(AppError::Forbidden("Only the owner or admins can list collaborators".into()));
        }

        let rows = sqlx::query(
            "SELECT nc.user_id, u.display_name, u.email, nc.permission, nc.created_at FROM note_collaborators nc INNER JOIN users u ON u.id = nc.user_id WHERE nc.note_id = $1 ORDER BY nc.created_at ASC"
        )
        .bind(note_id)
        .fetch_all(&self.pool)
        .await?;

        let mut result: Vec<CollaboratorInfo> = rows
            .iter()
            .map(|row| CollaboratorInfo {
                user_id: row.try_get("user_id").unwrap_or_default(),
                display_name: row.try_get("display_name").unwrap_or_default(),
                email: row.try_get("email").unwrap_or_default(),
                permission: row.try_get("permission").unwrap_or_default(),
                invited_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            })
            .collect();

        let owner_row = sqlx::query(
            "SELECT u.id, u.display_name, u.email FROM users u INNER JOIN notes n ON n.user_id = u.id WHERE n.id = $1"
        )
        .bind(note_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(owner) = owner_row {
            result.insert(0, CollaboratorInfo {
                user_id: owner.try_get("id").unwrap_or_default(),
                display_name: owner.try_get("display_name").unwrap_or_default(),
                email: owner.try_get("email").unwrap_or_default(),
                permission: "owner".into(),
                invited_at: Utc::now(),
            });
        }

        let total = result.len();
        Ok(CollaboratorListResponse { collaborators: result, total })
    }

    pub async fn add(&self, note_id: Uuid, actor_id: Uuid, req: AddCollaboratorRequest) -> Result<CollaboratorInfo> {
        let permission = self.get_user_permission(note_id, actor_id).await?;
        if permission != "owner" && permission != "admin" {
            return Err(AppError::Forbidden("Only the owner or admins can add collaborators".into()));
        }

        let target_row = sqlx::query(
            "SELECT id, display_name, email FROM users WHERE email = $1"
        )
        .bind(&req.email)
        .fetch_optional(&self.pool)
        .await?;

        let target = target_row.ok_or_else(|| AppError::NotFound("User with this email not found".into()))?;
        let target_id: Uuid = target.try_get("id").unwrap_or_default();
        let target_display_name: String = target.try_get("display_name").unwrap_or_default();
        let target_email: String = target.try_get("email").unwrap_or_default();

        if target_id == actor_id {
            return Err(AppError::BadRequest("Cannot add yourself as a collaborator".into()));
        }

        let note_owner_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM notes WHERE id = $1 AND is_deleted = false"
        )
        .bind(note_id)
        .fetch_optional(&self.pool)
        .await?;

        let note_owner_id = note_owner_id.ok_or_else(|| AppError::NotFound("Note not found".into()))?;

        if target_id == note_owner_id {
            return Err(AppError::BadRequest("User is the owner of this note".into()));
        }

        // Accept both the frontend vocabulary ('edit'/'view') and the
        // canonical vocabulary ('write'/'read') stored in the database.
        let collab_permission = req
            .permission
            .as_deref()
            .map(normalize_permission)
            .unwrap_or_else(|| "write".into());
        if collab_permission != "read" && collab_permission != "write" && collab_permission != "admin" {
            return Err(AppError::ValidationError(
                "Permission must be 'read', 'write', or 'admin' (or 'view'/'edit')".into(),
            ));
        }

        if permission == "admin" && collab_permission == "admin" {
            return Err(AppError::Forbidden("Admins cannot grant admin permission".into()));
        }

        let existing_count: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM note_collaborators WHERE note_id = $1"
        )
        .bind(note_id)
        .fetch_one(&self.pool)
        .await?;

        if (existing_count.unwrap_or(0) as usize) >= self.config.max_collaborators_per_note {
            return Err(AppError::Forbidden(format!(
                "Maximum collaborators ({}) reached for this note",
                self.config.max_collaborators_per_note
            )));
        }

        sqlx::query(
            "INSERT INTO note_collaborators (note_id, user_id, permission, invited_by) VALUES ($1, $2, $3, $4) ON CONFLICT (note_id, user_id) DO UPDATE SET permission = EXCLUDED.permission, invited_by = EXCLUDED.invited_by"
        )
        .bind(note_id)
        .bind(target_id)
        .bind(&collab_permission)
        .bind(actor_id)
        .execute(&self.pool)
        .await?;

        Ok(CollaboratorInfo {
            user_id: target_id,
            display_name: target_display_name,
            email: target_email,
            permission: collab_permission,
            invited_at: Utc::now(),
        })
    }

    pub async fn update_permission(
        &self,
        note_id: Uuid,
        target_user_id: Uuid,
        actor_id: Uuid,
        req: UpdateCollaboratorRequest,
    ) -> Result<CollaboratorInfo> {
        let actor_permission = self.get_user_permission(note_id, actor_id).await?;
        if actor_permission != "owner" && actor_permission != "admin" {
            return Err(AppError::Forbidden("Only the owner or admins can change permissions".into()));
        }

        let new_permission = normalize_permission(&req.permission);
        if new_permission != "read" && new_permission != "write" && new_permission != "admin" {
            return Err(AppError::ValidationError(
                "Permission must be 'read', 'write', or 'admin' (or 'view'/'edit')".into(),
            ));
        }

        if actor_permission == "admin" && new_permission == "admin" {
            return Err(AppError::Forbidden("Admins cannot grant admin permission".into()));
        }

        let target_perm: Option<String> = sqlx::query_scalar(
            "SELECT permission FROM note_collaborators WHERE note_id = $1 AND user_id = $2"
        )
        .bind(note_id)
        .bind(target_user_id)
        .fetch_optional(&self.pool)
        .await?;

        let target_permission = target_perm.ok_or_else(|| AppError::NotFound("Collaborator not found".into()))?;

        if target_permission == "admin" && actor_permission != "owner" {
            return Err(AppError::Forbidden("Only the owner can change an admin's permissions".into()));
        }

        sqlx::query(
            "UPDATE note_collaborators SET permission = $1 WHERE note_id = $2 AND user_id = $3"
        )
        .bind(&new_permission)
        .bind(note_id)
        .bind(target_user_id)
        .execute(&self.pool)
        .await?;

        let target_row = sqlx::query(
            "SELECT u.id, u.display_name, u.email FROM users u WHERE u.id = $1"
        )
        .bind(target_user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(CollaboratorInfo {
            user_id: target_row.try_get("id").unwrap_or_default(),
            display_name: target_row.try_get("display_name").unwrap_or_default(),
            email: target_row.try_get("email").unwrap_or_default(),
            permission: new_permission,
            invited_at: Utc::now(),
        })
    }

    pub async fn remove(&self, note_id: Uuid, target_user_id: Uuid, actor_id: Uuid) -> Result<()> {
        let permission = self.get_user_permission(note_id, actor_id).await?;
        if permission != "owner" && permission != "admin" {
            return Err(AppError::Forbidden("Only the owner or admins can remove collaborators".into()));
        }

        let target_perm: Option<String> = sqlx::query_scalar(
            "SELECT permission FROM note_collaborators WHERE note_id = $1 AND user_id = $2"
        )
        .bind(note_id)
        .bind(target_user_id)
        .fetch_optional(&self.pool)
        .await?;

        let target_permission = target_perm.ok_or_else(|| AppError::NotFound("Collaborator not found".into()))?;

        if target_permission == "admin" && permission != "owner" {
            return Err(AppError::Forbidden("Only the owner can remove an admin".into()));
        }

        sqlx::query("DELETE FROM active_sessions WHERE note_id = $1 AND user_id = $2")
            .bind(note_id)
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM note_collaborators WHERE note_id = $1 AND user_id = $2")
            .bind(note_id)
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_user_permission(&self, note_id: Uuid, user_id: Uuid) -> Result<String> {
        let is_owner: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)"
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if is_owner.unwrap_or(false) {
            return Ok("owner".into());
        }

        let collab_permission: Option<String> = sqlx::query_scalar(
            "SELECT permission FROM note_collaborators WHERE note_id = $1 AND user_id = $2"
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match collab_permission {
            Some(p) => Ok(p),
            None => Err(AppError::Forbidden("Not authorized to access this note".into())),
        }
    }

    pub async fn verify_note_access(&self, note_id: Uuid, user_id: Uuid) -> Result<String> {
        let is_owner: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)"
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if is_owner.unwrap_or(false) {
            return Ok("owner".into());
        }

        let collab_permission: Option<String> = sqlx::query_scalar(
            "SELECT permission FROM note_collaborators WHERE note_id = $1 AND user_id = $2"
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match collab_permission {
            Some(p) => Ok(p),
            None => Err(AppError::NotFound("Note not found or access denied".into())),
        }
    }

    pub async fn batch_get_note_collaborators(&self, note_ids: &[Uuid]) -> Result<std::collections::HashMap<Uuid, Vec<CollaboratorInfo>>> {
        if note_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT nc.note_id, nc.user_id, u.display_name, u.email, nc.permission, nc.created_at
            FROM note_collaborators nc
            INNER JOIN users u ON u.id = nc.user_id
            WHERE nc.note_id = ANY($1)
            UNION ALL
            SELECT n.id AS note_id, n.user_id, u.display_name, u.email, 'owner' AS permission, n.created_at
            FROM notes n
            INNER JOIN users u ON u.id = n.user_id
            WHERE n.id = ANY($1)
            ORDER BY note_id, created_at ASC
            "#,
        )
        .bind(note_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: std::collections::HashMap<Uuid, Vec<CollaboratorInfo>> = std::collections::HashMap::new();
        for row in rows {
            let note_id: Uuid = row.try_get("note_id").unwrap_or_default();
            let entry = map.entry(note_id).or_default();
            entry.push(CollaboratorInfo {
                user_id: row.try_get("user_id").unwrap_or_default(),
                display_name: row.try_get("display_name").unwrap_or_default(),
                email: row.try_get("email").unwrap_or_default(),
                permission: row.try_get("permission").unwrap_or_default(),
                invited_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(map)
    }

    pub async fn get_note_collaborators(&self, note_id: Uuid) -> Result<Vec<CollaboratorInfo>> {
        let rows = sqlx::query(
            "SELECT nc.user_id, u.display_name, u.email, nc.permission, nc.created_at FROM note_collaborators nc INNER JOIN users u ON u.id = nc.user_id WHERE nc.note_id = $1 ORDER BY nc.created_at ASC"
        )
        .bind(note_id)
        .fetch_all(&self.pool)
        .await?;

        let mut result: Vec<CollaboratorInfo> = rows
            .iter()
            .map(|row| CollaboratorInfo {
                user_id: row.try_get("user_id").unwrap_or_default(),
                display_name: row.try_get("display_name").unwrap_or_default(),
                email: row.try_get("email").unwrap_or_default(),
                permission: row.try_get("permission").unwrap_or_default(),
                invited_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            })
            .collect();

        let owner_row = sqlx::query(
            "SELECT u.id, u.display_name, u.email FROM users u INNER JOIN notes n ON n.user_id = u.id WHERE n.id = $1"
        )
        .bind(note_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(owner) = owner_row {
            result.insert(0, CollaboratorInfo {
                user_id: owner.try_get("id").unwrap_or_default(),
                display_name: owner.try_get("display_name").unwrap_or_default(),
                email: owner.try_get("email").unwrap_or_default(),
                permission: "owner".into(),
                invited_at: Utc::now(),
            });
        }

        Ok(result)
    }
}

/// Map the frontend permission vocabulary ('edit'/'view') onto the canonical
/// vocabulary stored in the database ('write'/'read').
fn normalize_permission(permission: &str) -> String {
    match permission {
        "edit" => "write".into(),
        "view" => "read".into(),
        other => other.into(),
    }
}
