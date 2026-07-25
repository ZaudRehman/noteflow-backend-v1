# NoteFlow API Reference

> Base URL: `https://noteflow-backend-v1.onrender.com/api/v1`
>
> All authenticated endpoints require `Authorization: Bearer <access_token>` header.

---

## Authentication

### POST `/auth/register`

**Purpose:** Create a new user account. The user fills in email, password, and display name on the registration page. On success, the frontend stores the JWT tokens and redirects to `/dashboard`. On failure (e.g. email already taken), it shows an inline error.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | yes | User email address |
| `password` | string | yes | User password |
| `display_name` | string | yes | Display name |

**Response `201`:** `AuthResponse`
```json
{
  "user": { "id": "uuid", "email": "string", "display_name": "string", "avatar_url": "string|null", "theme": "string", "preferences": {}, "created_at": "ISO8601", "last_login_at": "ISO8601" },
  "access_token": "string (JWT, 1 hour)",
  "refresh_token": "string (JWT, 30 days)"
}
```

---

### POST `/auth/login`

**Purpose:** Log in an existing user. Validates email + password, returns JWT tokens. On success, frontend stores tokens in localStorage and navigates to `/dashboard`. On 401, shows "Invalid email or password" error.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | yes | User email |
| `password` | string | yes | User password |

**Response `200`:** `AuthResponse` (same shape as register)

---

### POST `/auth/refresh`

**Purpose:** Silently renew an expired access token. Called automatically by the axios response interceptor when any API returns a 401. If this also fails (refresh token expired), the user is logged out and redirected to `/login`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `refresh_token` | string | yes | The stored refresh token |

**Response `200`:**
```json
{
  "access_token": "string (JWT, 1 hour)",
  "refresh_token": "string (JWT, 30 days)"
}
```

---

### GET `/auth/me`

**Purpose:** Validate the stored token and hydrate the user state. Called on app startup and page navigation to confirm the session is still valid and fetch user data (display name, avatar, theme, preferences). If the token is expired, the 401 triggers the refresh flow.

**Headers:** `Authorization: Bearer <access_token>`

**Response `200`:** `User` object

---

### POST `/auth/logout`

**Purpose:** End the current session. The frontend sends the refresh token to invalidate it server-side. On success, both tokens are cleared from localStorage and cookies, and the user is redirected to `/login`.

**Headers:** `Authorization: Bearer <access_token>`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `refresh_token` | string | yes | The stored refresh token |

**Response `200`:**
```json
{ "message": "Successfully logged out" }
```

---

### POST `/auth/forgot-password`

**Purpose:** Send a password reset email. The user enters their email on the "Forgot Password" page. The backend sends an email with a reset link containing a token. The frontend shows a success message telling the user to check their email.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | yes | User's registered email |

**Response `200`:**
```json
{ "message": "If that email exists, a password reset link has been sent" }
```

---

### POST `/auth/reset-password`

**Purpose:** Complete the password reset flow. The user clicks the reset link in their email (which brings them to the reset password page with a token in the URL). They enter a new password. The backend validates the token and updates the password. On success, the frontend redirects to the login page.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `token` | string | yes | Reset token from email link |
| `new_password` | string | yes | The new password |

**Response `200`:**
```json
{ "message": "Password reset successful" }
```

---

## Notes

All notes endpoints require `Authorization: Bearer <access_token>`.

### GET `/notes`

**Purpose:** List the user's notes for the main Notes page. Supports pagination, view filtering (all / favorites / archived), and tag filtering. The frontend renders notes as a grid or list depending on the user's saved preference.

| Query Param | Type | Default | Description |
|-------------|------|---------|-------------|
| `page` | number | 1 | Page number |
| `limit` | number | 20 | Items per page |
| `filter` | string | `all` | `all` | `favorites` | `archived` |
| `tag_id` | string | — | Filter by tag |

**Response `200`:**
```json
{
  "notes": [
    {
      "id": "uuid",
      "title": "string",
      "content": "string (preview — first ~200 chars)",
      "last_edited_by": "uuid",
      "is_favorited": false,
      "is_archived": false,
      "tags": ["tag_name_1"],
      "active_users": [],
      "created_at": "ISO8601",
      "updated_at": "ISO8601"
    }
  ],
  "total": 42,
  "page": 1,
  "limit": 20
}
```

---

### POST `/notes`

**Purpose:** Create a new empty note. The user clicks "New Note" from anywhere in the app. A title is required; content is optional (starts empty). The frontend immediately navigates to the new note's edit page.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | yes | Note title |
| `content` | string | no | Initial content (Markdown) |

**Response `201`:** `Note` object

---

### GET `/notes/:id`

**Purpose:** Open a note for editing or viewing. Returns the full note including content, tags, and active_users (for showing live collaborators in the UI). The frontend renders the note in the editor. If the note doesn't exist or doesn't belong to the user, returns 404.

**Response `200`:** `Note` object

---

### PUT `/notes/:id`

**Purpose:** Save changes to a note. Called by the auto-save mechanism (every few seconds while typing) and on manual save. Only changed fields need to be sent. The backend updates the note and returns it. The frontend uses the returned `updated_at` to confirm the save succeeded.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | no | Updated title |
| `content` | string | no | Updated content (Markdown) |

**Response `200`:** `Note` object (with updated `updated_at`)

---

### DELETE `/notes/:id`

**Purpose:** Delete a note. The frontend shows a confirmation dialog first. On confirm, it sends the DELETE request. The note is removed from the list. If other collaborators are viewing this note via WebSocket, they should receive a `note:deleted` event.

**Response `204`:** `void`

---

### POST `/notes/:id/favorite`

**Purpose:** Toggle the favorite (star) status on a note. The user clicks the star icon in the note list or editor. Each call flips the boolean. Favorited notes appear in the "Favorites" filter view and on the Dashboard.

**Response `200`:** `Note` object (with updated `is_favorited`)

---

### POST `/notes/:id/archive`

**Purpose:** Toggle archive status on a note. Archiving hides the note from the main "All Notes" list without deleting it. The user can find archived notes via the "Archived" filter view. Unarchiving moves it back to the main list.

**Response `200`:** `Note` object (with updated `is_archived`)

---

### POST `/notes/:id/tags`

**Purpose:** Assign an existing tag to a note. The user selects a tag from the tag picker in the editor sidebar. The tag must already exist (create it via `POST /tags` first). The note then appears under that tag's filter view and the tag's detail page.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tag_id` | string | yes | ID of the tag to assign |

**Response `201`:**
```json
{ "message": "Tag added to note" }
```

---

### DELETE `/notes/:id/tags/:tagId`

**Purpose:** Remove a tag from a note. The user clicks the "×" on a tag badge in the editor. The tag is disassociated from the note but the tag itself is not deleted.

**Response `204`:** `void`

---

## Tags

All tags endpoints require `Authorization: Bearer <access_token>`.

### GET `/tags`

**Purpose:** List all tags the user has created. Each tag includes a `note_count` so the sidebar can show how many notes use each tag. Used to populate the sidebar tag list, the tag picker in the editor, and the main Tags management page.

**Response `200`:**
```json
{
  "tags": [
    { "id": "uuid", "name": "string", "note_count": 5, "created_at": "ISO8601" }
  ],
  "total": 12
}
```

---

### GET `/tags/:id`

**Purpose:** Get a single tag's metadata. Used when navigating to a tag's detail page to show the tag name and note count.

**Response `200`:** `Tag` object

---

### POST `/tags`

**Purpose:** Create a new tag. The user types a name in the tag creation form. The tag is created without any notes assigned — notes are tagged separately via `POST /notes/:id/tags`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Tag name (should be unique per user) |

**Response `201`:** `Tag` object

---

### PUT `/tags/:id`

**Purpose:** Rename an existing tag. The user edits the tag name in the tag management UI. The change propagates everywhere the tag appears (sidebar, note badges, filter views).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | New tag name |

**Response `200`:** `Tag` object

---

### DELETE `/tags/:id`

**Purpose:** Delete a tag entirely. All tag-note associations are removed. The notes themselves are NOT deleted — they just lose the tag. The tag disappears from the sidebar, tag picker, and all note tag badges.

**Response `204`:** `void`

---

### GET `/tags/:id/notes`

**Purpose:** Get all notes that have a specific tag. Used to render the tag detail page, showing every note tagged with this tag in a paginated list. Supports the same pagination and sorting as `GET /notes`.

**Response `200`:**
```json
{ "notes": [ /* Note[] */ ] }
```

---

### POST `/notes/:note_id/tags`

**Purpose:** (Alternative path) Assign a tag to a note from the tags module. Same effect as `POST /notes/:id/tags`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tag_id` | string | yes | ID of the tag to assign |

**Response `201`:**
```json
{ "message": "Tag added to note" }
```

---

### DELETE `/notes/:note_id/tags/:tag_id`

**Purpose:** (Alternative path) Remove a tag from a note from the tags module. Same effect as `DELETE /notes/:id/tags/:tagId`.

**Response `204`:** `void`

---

## Search

Requires `Authorization: Bearer <access_token>`.

### GET `/search`

**Purpose:** Full-text search across all of the user's notes. Used by the search modal (Cmd+K / Ctrl+K) and the dedicated Search page. Searches note titles and content. Returns matching notes ordered by relevance.

| Query Param | Type | Default | Description |
|-------------|------|---------|-------------|
| `q` | string | — | Search query (required) — searches both title and content |
| `page` | number | 1 | Page number |
| `limit` | number | 20 | Items per page |

**Response `200`:**
```json
{
  "notes": [ /* Note[] */ ],
  "total": 10,
  "query": "search term"
}
```

---

## Users / Profile

All endpoints require `Authorization: Bearer <access_token>`.

### GET `/users/profile`

**Purpose:** Fetch the full user profile. Called on the Settings / Profile page to populate the form with current values (display name, email, theme, preferences). The `preferences` object contains language, timezone, editor mode, and notification settings that are restored on every login.

**Response `200`:** `UserProfile` object
```json
{
  "id": "uuid",
  "email": "string",
  "display_name": "string",
  "avatar_url": "string|null",
  "theme": "string",
  "preferences": {},
  "created_at": "ISO8601",
  "last_login_at": "ISO8601"
}
```

---

### PUT `/users/profile`

**Purpose:** Update the user's display name and/or avatar URL. Called when the user edits their profile information on the Settings / Profile page. Only changed fields need to be sent.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `display_name` | string | no | New display name |
| `avatar_url` | string | no | URL to the uploaded avatar image |

**Response `200`:** `UserProfile` object

---

### PUT `/users/preferences`

**Purpose:** Persist user preferences server-side. Called automatically by the settings store (Zustand + persist middleware) whenever the user changes any setting on the Settings page — theme, language, timezone, auto-save toggle, editor mode, or notification toggles. No "Save" button needed. Preferences are restored on every login via `GET /users/profile`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `theme` | string | no | `"light"` | `"dark"` | `"system"` |
| `preferences` | object | no | Nested settings object |

The `preferences` object shape sent by the frontend:
```json
{
  "language": "English",
  "timezone": "UTC",
  "autoSave": true,
  "editorMode": "split",
  "emailNotifications": true,
  "pushNotifications": false
}
```

**Response `200`:** `UserProfile` object

---

## Version History

All endpoints require `Authorization: Bearer <access_token>`.

### GET `/notes/:noteId/history`

**Purpose:** List all saved revisions for a note. Each revision is a full content snapshot created whenever the note is saved. Used on the Version History page to show a timeline of changes. The user can click any revision to preview it or restore it.

**Response `200`:**
```json
{
  "revisions": [
    {
      "id": "uuid",
      "note_id": "uuid",
      "content": "string (full content at that point in time)",
      "created_by": "uuid",
      "created_at": "ISO8601"
    }
  ],
  "total": 10
}
```

---

### GET `/notes/:noteId/history/:revisionId`

**Purpose:** Fetch a specific revision's full content. Called when the user clicks a revision in the history timeline to preview what the note looked like at that revision.

**Response `200`:** `Revision` object

---

### POST `/notes/:noteId/history/:revisionId/restore`

**Purpose:** Restore a note to a previous revision. The backend copies the revision's content into the current note. A new revision should be created to preserve the pre-restore state (so the restore itself can be undone). The frontend navigates back to the editor with the restored content.

**Response `200`:**
```json
{ "message": "Revision restored" }
```

---

## WebSocket

### Connect

```
wss://noteflow-backend-v1.onrender.com/api/v1/notes/:noteId/ws?token=<access_token>
```

**Purpose:** Open a real-time connection for live collaboration on a specific note. Used when the user opens a note in the editor. The connection enables seeing other users' cursors, receiving content updates from collaborators, and broadcasting one's own changes. The token is passed as a query parameter for authentication.

### Client → Server Messages

| Type | Payload | When It's Sent |
|------|---------|----------------|
| `note:created` | `{ title, content? }` | User creates a new note while connected |
| `note:updated` | `{ title?, content?, content_delta? }` | User types in the editor (debounced) |
| `note:deleted` | `{}` | User deletes the note |
| `tag:created` | `{ name }` | User creates a new tag |
| `tag:updated` | `{ name }` | User renames a tag |
| `tag:deleted` | `{}` | User deletes a tag |
| `cursor:move` | `{ line, column }` | User moves their cursor in the editor (throttled) |
| `user:joined` | `{ note_id }` | User opens a note for editing |
| `user:left` | `{ note_id }` | User navigates away from a note |
| `ping` | — | Every 30 seconds to keep the connection alive |

### Server → Client Messages

| Type | Payload | What the Frontend Does With It |
|------|---------|--------------------------------|
| `pong` | — | Confirms connection is alive |
| `content_update` | `{ content, title?, updated_by }` | Updates the editor content to reflect another user's changes |
| `cursor_update` | `{ user_id, display_name, line, column }` | Renders another user's cursor position in the editor |
| `user_joined` | `{ user_id, display_name }` | Shows a toast "X joined" and adds them to the collaborator list |
| `user_left` | `{ user_id, display_name }` | Shows a toast "X left" and removes them from the collaborator list |
| `active_users` | `[ { user_id, display_name } ]` | Updates the full list of current collaborators in the sidebar |
| `error` | `{ message }` | Shows an error toast |

---

## Standard Error Response

All API errors follow this shape:

```json
{
  "error": "Human-readable description",
  "status": 400
}
```

The `status` field reflects the HTTP status code (400, 401, 404, 429, 500, etc.).

### HTTP Status Codes Used

| Code | Meaning | When the Frontend Expects It |
|------|---------|------------------------------|
| 200 | Success | Standard success |
| 201 | Created | Resource creation (POST /notes, POST /tags) |
| 400 | Bad request | Validation error — frontend shows the message inline |
| 401 | Unauthorized | Missing/invalid token — frontend attempts silent refresh; if that fails, redirects to `/login` |
| 403 | Forbidden | User lacks permission for the resource (shouldn't occur in single-user context) |
| 404 | Resource not found | Note/tag doesn't exist — frontend shows a "Not found" page |
| 429 | Rate limited | Too many requests (100 req/min/user) — frontend waits and retries |
| 500 | Internal server error | Backend error — frontend shows a generic "Something went wrong" message |

---

## JWT Token Flow

| Token | Storage | Lifetime | Usage |
|-------|---------|----------|-------|
| `access_token` | localStorage + cookie | 1 hour | `Authorization: Bearer` header on all auth-required requests |
| `refresh_token` | localStorage + cookie | 30 days | `POST /auth/refresh` to get a new `access_token` |

On every 401 response, the axios interceptor tries `POST /auth/refresh` with the stored refresh token. If that succeeds, the failed request is retried with the new access token. If it also fails (refresh token expired or revoked), the user is logged out and redirected to `/login`.

---

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NEXT_PUBLIC_API_BASE_URL` | yes | — | Backend REST API base URL |
| `NEXT_PUBLIC_WS_BASE_URL` | yes | — | WebSocket base URL |
| `NEXT_PUBLIC_APP_NAME` | no | `Noteflow` | Application name (used in page titles, email subjects) |
| `NEXT_PUBLIC_APP_URL` | no | `http://localhost:3000` | Frontend URL (used in reset password email links, etc.) |
