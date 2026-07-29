# NoteFlow Backend

A production-grade REST API for real-time collaborative note-taking built with Rust, Axum, PostgreSQL, and Redis.

[Live Demo](https://noteflow-frontend-phi.vercel.app/) · [API Docs](https://noteflow-backend-v1.onrender.com/docs) · [Frontend UI](https://github.com/ZaudRehman/noteflow-frontend)

---

## Table of Contents

- [About](#about)
- [Features](#features)
- [Tech Stack](#tech-stack)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
- [API Documentation](#api-documentation)
- [Database Schema](#database-schema)
- [WebSocket Integration](#websocket-integration)
- [Deployment](#deployment)
- [Project Structure](#project-structure)
- [Security](#security)
- [Contributing](#contributing)
- [License](#license)
- [Contact](#contact)

---

## About

NoteFlow Backend is a high-performance REST API built with Rust that powers a collaborative note-taking application. It demonstrates modern backend development practices including asynchronous programming, JWT authentication, real-time WebSocket communication, CRDT-based conflict resolution, push notifications, and cloud-native deployment.

### Capabilities

- **High Performance** - Built with Rust for sub-200ms p95 response times
- **Enterprise Security** - JWT authentication, bcrypt hashing, rate limiting, account lockout
- **Scalable Architecture** - Async/await patterns, connection pooling, horizontal scaling through Redis pub/sub
- **Real-Time Sync** - WebSocket support with CRDT operation relay for conflict-free collaborative editing
- **Version Control** - Automatic revision history via PostgreSQL triggers
- **Collaboration** - Permission-based sharing with owner/edit/view roles
- **Smart Organization** - Tag system with many-to-many relationships
- **Observability** - Prometheus metrics, structured JSON logging, request tracing, health checks
- **Cloud Native** - Docker support, Supabase/Upstash integration, OpenAPI docs

---

## Features

### Authentication & Authorization
- JWT dual-token system with access (1h) and refresh (30d) tokens
- Secure password storage with bcrypt hashing (cost factor 12)
- Token rotation: refresh tokens are one-time-use
- Session listing and revocation across devices
- User registration, login, and management
- Account lockout after 10 failed attempts (15-minute lock)
- Per-email brute-force rate limiter (5 req/60s)

### Note Management
- Full CRUD operations with ownership verification
- **Block-based content model**: notes composed of typed blocks (paragraph, heading, list, code, table, chart, image, divider, quote, todo)
- Favorites and archive organization (toggle endpoints)
- Soft delete with filter exclusion
- Advanced filtering by favorite, archived, tag, with sorting
- Paginated retrieval with configurable page sizes
- Sorting by created_at, updated_at, or title (ASC/DESC)
- **Multi-format export**: markdown, HTML, plain text, RTF, PDF, EPUB
- **Customizable styling**: 7 style parameters for exports (page size, font, font size, theme, border, line spacing, margins) with dark/sepia 10-color palettes

### Collaboration & Sharing
- Permission-based collaborator system (owner, edit, view)
- Invite collaborators by email
- Update/revoke collaborator permissions
- Real-time collaborative editing via CRDT operation relay
- Live cursor position broadcasting
- Active user presence tracking

### Tag System
- User-unique tags with many-to-many note relationships
- Full CRUD operations with note count stats
- Tag assignment and removal from notes
- Tag-based note filtering and listing

### Search & Discovery
- PostgreSQL full-text search via GIN indexes (`to_tsvector` / `plainto_tsquery`)
- Stemming-aware (handles "running" → "run", "notes" → "note")
- Fallback ILIKE matching for partial/fuzzy queries
- Relevance-ranked results (title matches weighted first)
- Per-user scope (own notes + notes shared as collaborator)

### User Management
- Display name and profile management
- Avatar upload via multipart/form-data, stored on ImageKit CDN (max 5MB, JPEG/PNG/GIF/WebP)
- Theme preferences (light, dark, system)
- JSONB custom preferences (language, timezone, editor mode, notification toggles)
- Password change and reset flows with email notifications
- Last login tracking

### Version History
- Automatic revision snapshots via PostgreSQL trigger on every note update
- Paginated revision listing with author and timestamps
- Point-in-time restore to any previous revision
- Pre-restore snapshot preserves undo capability

### Push Notifications
- RFC 8291 Web Push protocol with VAPID authentication
- Pure Rust cryptography (AES-256-GCM, ECDH, HKDF, no system OpenSSL)
- Subscription CRUD (subscribe, unsubscribe, list)
- Automatic cleanup of expired push endpoints (410 responses)
- Transactional email via Brevo (300 emails/day free)
- Notifications on note edits by collaborators and password changes

### Real-Time Collaboration (Block CRDT)
- Scalable to 1,000+ concurrent WebSocket connections
- **Block-level CRDT relay**: `block:add`, `block:update`, `block:remove`, `block:move` operations
- Late-join sync: `block:sync_batch` for disconnected clients
- Per-user connection limit (max 5 concurrent WS connections)
- Active user presence with cursor position broadcasting
- Redis pub/sub for multi-instance cross-cluster messaging
- Automatic cleanup of stale sessions
- **Ticket-based WS auth**: short-lived (30s), single-use tickets vs. JWT in URL

### Observability & Operations
- Prometheus metrics endpoint (`GET /metrics`) with latency histograms
- Structured JSON logging via `tracing-subscriber` with `flatten_event`
- Request-scoped UUIDs (`X-Request-Id`) across every request
- Span-based tracing with `method`, `uri`, `status_code`, `user_id`
- Health check endpoint with DB/Redis status + structured JSON response (503 on DB down)
- OpenAPI 3.0 docs at `/docs` with interactive Swagger UI
- Graceful shutdown (SIGTERM/SIGINT) with in-flight request draining

### Security & Performance
- Global request body size limit (5MB via `RequestBodyLimitLayer`)
- IP-based rate limiting with `X-RateLimit-Limit` / `X-RateLimit-Remaining` headers
- Per-email brute-force protection (5 req/60s on login/register)
- Account lockout (10 failures → 15-min lock)
- WS connection rate limiting (max 5 per user)
- SQL injection prevention via parameterized queries
- bcrypt password hashing with per-password salts
- Configurable CORS policies
- Gzip response compression
- Composite + GIN indexes for query performance
- In-memory cache layer for tags and notification subscriptions (TTL-based)
- Cursor-based pagination for large result sets

---

## Tech Stack

### Backend Framework
- [Axum](https://github.com/tokio-rs/axum) 0.7 - Ergonomic and modular web framework
- [Tokio](https://tokio.rs/) - Async runtime with multi-threading
- [Tower](https://github.com/tower-rs/tower) - Middleware and service abstractions
- [Tower-HTTP](https://github.com/tower-rs/tower-http) - CORS, compression, tracing middleware

### Database & Caching
- [PostgreSQL](https://www.postgresql.org/) 15+ - Relational database with ACID guarantees
- [SQLx](https://github.com/launchbadge/sqlx) 0.7 - Async SQL toolkit with compile-time verification
- [Redis](https://redis.io/) 7+ - In-memory data store for caching and pub/sub
- [Supabase](https://supabase.com/) - Managed PostgreSQL with connection pooling
- [Upstash](https://upstash.com/) - Serverless Redis with TLS support

### Authentication & Security
- [jsonwebtoken](https://github.com/Keats/jsonwebtoken) - JWT implementation with HS256/RS256
- [bcrypt](https://github.com/Keats/rust-bcrypt) - Password hashing with salt rounds
- [uuid](https://github.com/uuid-rs/uuid) - Universally unique identifiers
- [validator](https://github.com/Keats/validator) - Struct validation with derive macros
- [sha2](https://github.com/RustCrypto/hashes) - Token hashing
- [rand](https://github.com/rust-random/rand) - Secure random generation

### Encryption
- [aes-gcm](https://github.com/RustCrypto/AEADs) - AES-256-GCM for Web Push payload encryption
- [p256](https://github.com/RustCrypto/elliptic-curves) - NIST P-256 ECDH + ECDSA for VAPID
- [hkdf](https://github.com/RustCrypto/KDFs) - HMAC-based key derivation for push salt

### Serialization & Validation
- [Serde](https://serde.rs/) - Serialization framework for JSON
- [serde_json](https://github.com/serde-rs/json) - JSON support for Serde
- [base64](https://github.com/marshallpierce/rust-base64) - Base64 encoding/decoding
- [url](https://github.com/servo/rust-url) - URL parsing and validation
- [chrono](https://github.com/chronotope/chrono) - Date and time library

### Email & Notifications
- [Brevo](https://www.brevo.com/) - Transactional email delivery (300 emails/day free)
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client for API integration

### Configuration & Logging
- [dotenvy](https://github.com/allan2/dotenvy) - Environment variable management
- [tracing](https://github.com/tokio-rs/tracing) - Structured logging and diagnostics
- [tracing-subscriber](https://github.com/tokio-rs/tracing) - Log formatting and filtering

### DevOps & Deployment
- [Docker](https://www.docker.com/) - Containerization with multi-stage builds
- [Docker Compose](https://docs.docker.com/compose/) - Local development orchestration
- GitHub Actions - CI/CD pipeline

---

## Architecture

NoteFlow Backend follows a clean layered architecture:

```
+--------------------------------------------------------------------+
|                       Axum Web Server                              |
|                    (Tower Middleware Stack)                        |
+--------------------------------------------------------------------+
|  Middleware Layer  (applied outermost → innermost for incoming)    |
|  +-- Extension(PrometheusHandle)    (metrics recorder injection)   |
|  +-- CORS                           (cross-origin resource sharing)|
|  +-- RequestBodyLimit               (5MB limit)                    |
|  +-- Compression                    (gzip response compression)    |
|  +-- RequestId                      (UUID per request)             |
|  +-- TraceLayer                     (structured span logging)      |
|  +-- Rate Limiting                  (IP-based + per-email)         |
|  +-- Authentication                 (JWT verification)             |
+--------------------------------------------------------------------+
|  Presentation Layer (HTTP + WebSocket Handlers)                    |
|  +-- Auth Routes              (register, login, refresh, sessions) |
|  +-- Note Routes              (CRUD, favorite, archive, export)    |
|  +-- Revision Routes          (history, restore)                   |
|  +-- Tag Routes               (tag CRUD, assignment)               |
|  +-- Collaborator Routes      (share, permission management)       | 
|  +-- User Routes              (profile, preferences, avatar)       |
|  +-- Notification Routes      (push subscribe/unsubscribe)         |
|  +-- Search Route             (full-text search)                   |
|  +-- WebSocket Handler        (CRDT relay, cursor broadcast)       |
|  +-- Health / Metrics / Docs  (observability)                      |
+--------------------------------------------------------------------+
|  Business Logic Layer (Services)                                   |
|  +-- AuthService             (authentication, tokens, lockout)     |
|  +-- NoteService             (note CRUD, batch loading, cache)     |
|  +-- UserService             (profile, avatar, preferences)        |
|  +-- TagService              (tag CRUD, note assignment)           |
|  +-- NoteCollaboratorService (permission management)               |
|  +-- RevisionService         (version history)                     |
|  +-- NotificationService     (Web Push + Brevo email)              |
|  +-- CollaborationService    (WS lifecycle, CRDT relay, sync)      |
+--------------------------------------------------------------------+
|  Data Access Layer                                                 |
|  +-- SQLx (raw queries + compile-time macros)                      |
|  +-- PostgreSQL Connection Pool     (up to 20 sessions)            |
|  +-- Redis Connection Manager       (pub/sub, caching)             |
|  +-- Migration Manager              (sqlx::migrate!)               |
+--------------------------------------------------------------------+
|  Infrastructure                                                    |
|  +-- PostgreSQL 15+               (primary data store)             |
|  +-- Redis 7+                     (pub/sub, limiter counters)      |
+--------------------------------------------------------------------+
```

### Design Patterns

- Repository Pattern - Data access abstraction through services
- Dependency Injection - Axum's State for service sharing
- Middleware Pattern - Request/response transformation pipeline
- Factory Pattern - JWT token generation and validation
- Observer Pattern - Redis pub/sub for real-time events
- Strategy Pattern - Configurable rate limiting and validation

### Data Flow

```
HTTP Request -> Middleware -> Handler -> Service -> Database -> Response
                    |
              [Rate Limit]
              [Auth Check]
              [Validation]
                    |
                 Response
```

---

## Getting Started

### Prerequisites

- Rust 1.75+ - [Install Rust](https://rustup.rs/)
- PostgreSQL 15+ - [Download](https://www.postgresql.org/download/) or use [Supabase](https://supabase.com/)
- Redis 7+ - [Download](https://redis.io/download) or use [Upstash](https://upstash.com/)
- SQLx CLI - For database migrations
- Git - Version control

### Local Development Setup

#### 1. Clone the repository

```bash
git clone https://github.com/ZaudRehman/noteflow-backend-v1.git
cd noteflow-backend-v1
```

#### 2. Install SQLx CLI

```bash
cargo install sqlx-cli --features postgres
```

#### 3. Configure environment variables

Create `.env` file:

```bash
cp .env.example .env
```

Edit `.env` with your credentials:

```env
# Server
HOST=0.0.0.0
PORT=8080
RUST_LOG=info

# PostgreSQL (Local or Supabase)
DATABASE_URL=postgresql://user:password@localhost:5432/noteflow

# Redis (Local or Upstash)
REDIS_URL=redis://localhost:6379

# JWT (Generate: openssl rand -base64 32)
JWT_SECRET=your-super-secret-key-minimum-32-characters
ENCRYPTION_KEY=0123456789abcdef0123456789abcdef   # 32 hex chars for AES-GCM

# Email (Brevo, 300 emails/day free)
BREVO_API_KEY=
BREVO_FROM_EMAIL=noreply@noteflow.app

# Frontend URL (used in reset password email links)
SELF_URL=http://localhost:8080

# Web Push VAPID Keys (optional, auto-generated if empty)
VAPID_PUBLIC_KEY=
VAPID_PRIVATE_KEY=
VAPID_SUBJECT=mailto:notifications@noteflow.app

# ImageKit (free tier avatar CDN; get keys from https://imagekit.io)
IMAGEKIT_PRIVATE_KEY=

```

#### 4. Setup database

```bash
# Create database
sqlx database create

# Run migrations
sqlx migrate run
```

#### 5. Build and run

```bash
# Development mode (with auto-reload)
cargo watch -x run

# Or standard run
cargo run

# Production build
cargo build --release
./target/release/noteflow-backend
```

#### 6. Verify installation

```bash
# Health check
curl http://localhost:8080/health

# Expected response: (JSON with DB + Redis status)
{"status":"ok","version":"0.1.0","database":true,"redis":true,"timestamp":"..."}
```

### Docker Setup (Alternative)

```bash
# Start all services (PostgreSQL + Redis + Backend)
docker-compose up -d

# View logs
docker-compose logs -f backend

# Stop services
docker-compose down
```

---

## API Documentation

Interactive Swagger UI available at [`/docs`](https://noteflow-backend-v1.onrender.com/docs) (auto-generated via `utoipa`).

### Base URL

```
http://localhost:8080
```

### Authentication

All protected endpoints require JWT token in `Authorization` header:

```
Authorization: Bearer <access_token>
```

### Endpoints Overview

#### Public

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check (DB + Redis status, structured JSON) |
| `GET` | `/metrics` | Prometheus metrics (latency histograms, request counts) |
| `GET` | `/docs` | Interactive Swagger UI |
| `GET` | `/api-docs/openapi.json` | Raw OpenAPI 3.0 spec |

#### Authentication (Public)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/auth/register` | Register new user (stores refresh token) |
| `POST` | `/api/v1/auth/login` | Login (stores refresh token, updates last_login) |
| `POST` | `/api/v1/auth/refresh` | Refresh access + refresh token (rotation) |
| `POST` | `/api/v1/auth/forgot-password` | Request password reset email |
| `POST` | `/api/v1/auth/reset-password` | Reset password with token |

#### Authentication (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/auth/me` | Get current user profile |
| `POST` | `/api/v1/auth/logout` | Revoke refresh token |
| `GET` | `/api/v1/auth/sessions` | List active sessions (user-agent, IP, dates) |
| `DELETE` | `/api/v1/auth/sessions/:session_id` | Revoke specific session |

#### Notes (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/notes?filter=&tag_id=&sort_by=&sort_order=&page=&limit=` | List notes with advanced filtering |
| `POST` | `/api/v1/notes` | Create new note |
| `GET` | `/api/v1/notes/:id` | Get note with tags, collaborators, active users |
| `PUT` | `/api/v1/notes/:id` | Update note (triggers push notification to owner) |
| `DELETE` | `/api/v1/notes/:id` | Soft delete note |
| `POST` | `/api/v1/notes/:id/favorite` | Toggle favorite status |
| `POST` | `/api/v1/notes/:id/archive` | Toggle archive status |
| `GET` | `/api/v1/notes/:id/export?format=markdown\|html\|txt\|rtf\|pdf\|epub&page_size=&font=&font_size=&theme=&border=&line_spacing=&margins=` | Export note (7 formats, 7 style params) |

#### Collaborators (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/notes/:note_id/collaborators` | List collaborators with permissions |
| `POST` | `/api/v1/notes/:note_id/collaborators` | Invite collaborator by email |
| `PUT` | `/api/v1/notes/:note_id/collaborators/:target_user_id` | Change collaborator permission |
| `DELETE` | `/api/v1/notes/:note_id/collaborators/:target_user_id` | Remove collaborator |

#### Tags (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/tags` | List all tags with note counts |
| `POST` | `/api/v1/tags` | Create new tag |
| `PUT` | `/api/v1/tags/:id` | Rename tag |
| `DELETE` | `/api/v1/tags/:id` | Delete tag (removes associations, keeps notes) |
| `GET` | `/api/v1/tags/:id/notes` | List notes with this tag |
| `POST` | `/api/v1/notes/:note_id/tags` | Assign tag to note |
| `DELETE` | `/api/v1/notes/:note_id/tags/:tag_id` | Remove tag from note |

#### Search (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/search?q=&page=&limit=` | Full-text search across notes (FTS + ILIKE fallback) |

#### Version History (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/notes/:note_id/history` | List revisions for a note |
| `GET` | `/api/v1/notes/:note_id/history/:revision_id` | Get specific revision content |
| `POST` | `/api/v1/notes/:note_id/history/:revision_id/restore` | Restore note to revision |

#### User Profile (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/users/profile` | Get full profile with preferences |
| `PUT` | `/api/v1/users/profile` | Update display name / avatar URL |
| `PUT` | `/api/v1/users/preferences` | Save theme, language, timezone, editor mode, toggles |
| `POST` | `/api/v1/users/avatar` | Upload avatar (multipart/form-data, max 5MB) |

#### Push Notifications (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/notifications/push/subscriptions` | List push subscriptions |
| `POST` | `/api/v1/notifications/push/subscribe` | Subscribe browser to push |
| `DELETE` | `/api/v1/notifications/push/subscribe/:id` | Unsubscribe from push |

#### WebSocket (Protected, ticket-based auth)

| Protocol | Endpoint | Description |
|----------|----------|-------------|
| `POST` | `/api/v1/ws/ticket` | Obtain short-lived WS ticket (`Authorization: Bearer` + `{ "note_id": "uuid" }`) → `{ "ticket": "64-char-hex", "expires_in": 30 }` |
| `WS` | `/api/v1/notes/:id/ws?ticket=<64-char-hex>` | Real-time collaboration (block CRDT ops, cursors, presence) |

JWT is **never** passed in URL query strings. Client calls `POST /api/v1/ws/ticket` with JWT in header to get a 30s single-use ticket, then passes `?ticket=` to the WS upgrade URL. Legacy `?token=` kept as fallback.

### HTTP Status Codes

| Code | Status | Usage |
|------|--------|-------|
| `200` | OK | Successful GET, PUT, POST requests |
| `201` | Created | Successful resource creation |
| `204` | No Content | Successful delete operations |
| `400` | Bad Request | Validation error (invalid input, missing field) |
| `401` | Unauthorized | Missing or invalid access token |
| `403` | Forbidden | Insufficient permissions (view-only collaborator) |
| `404` | Not Found | Resource doesn't exist or no access |
| `429` | Too Many Requests | Rate limit exceeded or account locked |
| `500` | Internal Server Error | Server-side error |

---

## Database Schema

### Entity Relationship Diagram

```
                    +------------------------------------------+
                    |                users                     |
                    +------------------------------------------+
                    | id (PK, UUID)                            |
                    | email (UNIQUE)                           |
                    | password_hash                            |
                    | display_name                             |
                    | avatar_url                               |
                    | theme                                    |
                    | preferences (JSONB)                      |
                    | reset_token                              |
                    | reset_token_expires                      |
                    | last_login_at                            |
                    | failed_login_attempts                    |
                    | locked_until                             |
                    | created_at                               |
                    | updated_at                               |
                    +--------+-------------------+-------------+
                             |                   |
                             | 1:N               | 1:N
            +----------------+------+    +-------+-----------+
            |                       |    |                   |
            v                       v    v                   v
  +---------------------+ +-------------+ +-----------------------+
  |       notes         | |    tags     | |  refresh_tokens       |
  +---------------------+ +-------------+ +-----------------------+
  | id (PK)             | | id (PK)     | | id (PK)               |
  | user_id (FK)        | | user_id(FK) | | user_id (FK)          |
  | title               | | name        | | token_hash            | 
  | content             | | created_at  | | expires_at            | 
  | is_favorited        | +------+------+ | revoked               |
  | is_archived         |        |        | revoked_at            |
  | is_deleted          |        |        | user_agent            |
  | last_edited_by (FK) |        | N:M    | ip_address            |
  | created_at          |        |        | created_at            |
  | updated_at          |   +----+------+ +-----------------------+
  +----------+----------+   | note_tags |
             |              +-----------+
             |              | note_id   |
             | 1:N          | tag_id    |
             |              | created_at|
             |              +-----------+
             |
             | 1:N
             |
              | 1:N
              v
  +-------------------------------+
  |  note_collaborators           |
  +-------------------------------+
  | note_id (FK)                  |
  | user_id (FK)                  |
  | permission (owner/edit/view)  |
  | invited_by (FK)               |
  | created_at                    |
  +-------------------------------+

              | 1:N
              v
  +-------------------------------+
  |  note_blocks                  |
  +-------------------------------+
  | id (PK, UUID)                 |
  | note_id (FK)                  |
  | block_type                    |
  | data (JSONB)                  |
  | position (INT)                |
  | parent_id (FK, self-ref)      |
  | created_at                    |
  | updated_at                    |
  +-------------------------------+

    +-----------------------------+     +--------------------------------+
    |  push_subscriptions         |     |  active_sessions               |
    +-----------------------------+     +--------------------------------+
    | id (PK)                     |     | id (PK)                        |
    | user_id (FK)                |     | note_id (FK)                   |
    | endpoint                    |     | user_id (FK)                   |
    | p256dh                      |     | cursor_line                    |
    | auth                        |     | cursor_column                  |
    | user_agent (new)            |     | last_seen_at                   |
    | created_at                  |     | created_at                     |
    +-----------------------------+     +--------------------------------+
```

### Table Definitions

#### Users Table

```sql
CREATE TABLE users (
id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
email VARCHAR(255) UNIQUE NOT NULL,
password_hash VARCHAR(255) NOT NULL,
display_name VARCHAR(100) NOT NULL,
avatar_url TEXT,
theme VARCHAR(20) DEFAULT 'light' CHECK (theme IN ('light', 'dark', 'auto')),
preferences JSONB DEFAULT '{}'::jsonb,
reset_token VARCHAR(64),
reset_token_expires TIMESTAMPTZ,
last_login_at TIMESTAMPTZ,
failed_login_attempts INTEGER DEFAULT 0,     
locked_until TIMESTAMPTZ,                    
created_at TIMESTAMPTZ DEFAULT NOW(),
updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_reset_token ON users(reset_token) WHERE reset_token IS NOT NULL;
CREATE INDEX idx_users_preferences ON users USING GIN(preferences);
```

#### Notes Table

```sql
CREATE TABLE notes (
id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
title VARCHAR(255) NOT NULL DEFAULT 'Untitled',
content TEXT NOT NULL DEFAULT '',
last_edited_by UUID REFERENCES users(id),
is_favorited BOOLEAN NOT NULL DEFAULT FALSE,
is_archived BOOLEAN NOT NULL DEFAULT FALSE,
is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
created_at TIMESTAMPTZ DEFAULT NOW(),
updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_notes_user_id ON notes(user_id);
CREATE INDEX idx_notes_user_filters ON notes(user_id, is_deleted, is_archived, is_favorited, updated_at DESC);
CREATE INDEX idx_notes_user_favorited ON notes(user_id, updated_at DESC) WHERE is_favorited = true;
CREATE INDEX idx_notes_user_archived ON notes(user_id, updated_at DESC) WHERE is_archived = true;

CREATE INDEX idx_notes_content_search ON notes USING GIN (to_tsvector('english', content));
CREATE INDEX idx_notes_title_search ON notes USING GIN (to_tsvector('english', title));

CREATE INDEX IF NOT EXISTS idx_notes_access_check ON notes(id, user_id, is_deleted);
CREATE INDEX IF NOT EXISTS idx_notes_full_text_search
    ON notes USING GIN (to_tsvector('english', coalesce(title, '') || ' ' || coalesce(content, '')));
```

#### Revisions Table

```sql
CREATE TABLE revisions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_revisions_note_id ON revisions(note_id, created_at DESC);

CREATE TRIGGER trigger_create_note_revision
    BEFORE UPDATE ON notes
    FOR EACH ROW
    EXECUTE FUNCTION create_note_revision();
```

#### Tags Tables

```sql
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, name)
);

CREATE TABLE note_tags (
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (note_id, tag_id)
);
```

#### Refresh Tokens

```sql
CREATE TABLE refresh_tokens (
id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
token_hash VARCHAR(64) NOT NULL UNIQUE,
expires_at TIMESTAMPTZ NOT NULL,
revoked BOOLEAN NOT NULL DEFAULT FALSE,
revoked_at TIMESTAMPTZ,
user_agent TEXT,
ip_address INET,
created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens(token_hash) WHERE NOT revoked;
CREATE INDEX idx_refresh_tokens_expires ON refresh_tokens(expires_at) WHERE NOT revoked;
```

#### Push Subscriptions

```sql
CREATE TABLE push_subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_push_subscriptions_user_id ON push_subscriptions(user_id);
```

#### Active Sessions

```sql
CREATE TABLE active_sessions (
id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
cursor_line INTEGER NOT NULL DEFAULT 0,
cursor_column INTEGER NOT NULL DEFAULT 0,
last_seen_at TIMESTAMPTZ DEFAULT NOW(),
created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_active_sessions_note_id ON active_sessions(note_id);
CREATE INDEX idx_active_sessions_user_id ON active_sessions(user_id)
WHERE last_seen_at > NOW() - INTERVAL '5 minutes';
```

#### Note Collaborators (new)

```sql
CREATE TABLE note_collaborators (
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission VARCHAR(20) NOT NULL DEFAULT 'edit' CHECK (permission IN ('edit', 'view')),
    invited_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (note_id, user_id)
);
```

#### Note Blocks (block-based content model)

Replaces flat `notes.content` with typed, ordered blocks.

```sql
CREATE TABLE note_blocks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    block_type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    position INTEGER NOT NULL DEFAULT 0,
    parent_id UUID REFERENCES note_blocks(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_note_blocks_note_id ON note_blocks(note_id, position);

CREATE OR REPLACE FUNCTION update_note_updated_at()
RETURNS TRIGGER AS $$ BEGIN
    UPDATE notes SET updated_at = NOW() WHERE id = NEW.note_id;
    RETURN NEW;
END; $$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_note_updated_at
    AFTER INSERT OR DELETE OR UPDATE ON note_blocks
    FOR EACH ROW EXECUTE FUNCTION update_note_updated_at();
```

**Supported block types**: `paragraph`, `heading`, `bullet_list`, `numbered_list`, `todo_list`, `quote`, `code`, `divider`, `table`, `image`, `chart`. See [API docs](#api-documentation) for data schemas.

### Database Optimizations

- Composite indexes for fast user-specific queries
- GIN indexes for full-text search (separate title + content + combined)
- Composite index for access checks (`notes(id, user_id, is_deleted)`)
- Composite index for note_tags joins (`note_tags(note_id, tag_id)`)
- Foreign key constraints for referential integrity
- Cascade deletes for automatic cleanup of related data
- Trigger-based updated_at timestamps
- Trigger-based revision snapshots on note update
- Soft deletes for note recovery
- Auto-cleanup of stale collaboration operations
- Connection pooling for efficient resource usage

---

## WebSocket Integration

### Connection Flow

```
1. CLIENT: POST /api/v1/ws/ticket  (Authorization: Bearer <jwt>)
   BODY: { "note_id": "uuid" }
   SERVER: { "ticket": "64-char-hex", "expires_in": 30 }

2. CLIENT: wss://host/api/v1/notes/{note_id}/ws?ticket=<64-char-hex>

   Server validates ticket against Redis, deletes on first use
   +-- Checks note access
   +-- Checks per-user WS limit (max 5)
   +-- Rejects with error if limit exceeded
   +-- Creates active_session record
   +-- Broadcasts "user:joined" to all other connected clients

3. Client sends cursor moves, block edits, or block CRDT operations
   +-- "block:add", "block:update", "block:remove", "block:move"
   +-- Relayed to all other connections for this note
   +-- Note content rebuilt from blocks after updates
   +-- Published to Redis for other instances

4. Client sends "block:sync_request"
   +-- Server responds with "block:sync_batch" of all current blocks

5. Client receives updates from other users
   +-- Updates editor blocks, cursor positions, presence list

6. Client sends "ping" every 30 seconds
   +-- Server responds with "pong"

7. Client disconnects (navigates away, closes tab, timeout)
   +-- Deletes active_session record
   +-- Broadcasts "user:left" event
   +-- Decrements per-user connection counter
```

### Client → Server Message Types

```typescript
// Heartbeat
{ "type": "ping", "timestamp": "ISO8601" }

// Cursor position (throttled, broadcast to other clients)
{
  "type": "cursor:move",
  "note_id": "uuid",
  "user_id": "uuid",
  "user_name": "John Doe",
  "position": { "line": 5, "column": 12 },
  "timestamp": "ISO8601"
}

// Block: Add a new block (relayed to other clients)
{
  "type": "block:add",
  "note_id": "uuid",
  "block_id": "uuid",
  "block_type": "paragraph",
  "data": { "text": "Hello" },
  "position": 0,
  "parent_id": null,
  "client_id": "session-uuid",
  "timestamp": "ISO8601"
}

// Block: Update block data (relayed to other clients)
{
  "type": "block:update",
  "note_id": "uuid",
  "block_id": "uuid",
  "data": { "text": "Updated" },
  "client_id": "session-uuid",
  "timestamp": "ISO8601"
}

// Block: Remove a block (relayed to other clients)
{
  "type": "block:remove",
  "note_id": "uuid",
  "block_id": "uuid",
  "client_id": "session-uuid",
  "timestamp": "ISO8601"
}

// Block: Move block to new position/parent (relayed to other clients)
{
  "type": "block:move",
  "note_id": "uuid",
  "block_id": "uuid",
  "new_position": 2,
  "new_parent_id": null,
  "client_id": "session-uuid",
  "timestamp": "ISO8601"
}

// Block: Sync request (sent on reconnect to get all blocks)
{
  "type": "block:sync_request",
  "note_id": "uuid",
  "timestamp": "ISO8601"
}
```

### Server → Client Message Types

```typescript
// Heartbeat response
{ "type": "pong", "timestamp": "ISO8601" }

// Cursor from another user (render colored indicator)
{
  "type": "cursor:move",
  "note_id": "uuid",
  "user_id": "uuid",
  "user_name": "Jane Smith",
  "position": { "line": 5, "column": 12 },
  "timestamp": "ISO8601"
}

// Presence
{ "type": "user:joined",  "note_id": "uuid", "user_id": "uuid", "user_name": "string", "timestamp": "ISO8601" }
{ "type": "user:left",    "note_id": "uuid", "user_id": "uuid", "timestamp": "ISO8601" }

// Block operations from other users
{ "type": "block:add",    "note_id": "uuid", "block_id": "uuid", "block_type": "string", "data": {...}, "position": 0, "parent_id": null, "client_id": "string", "timestamp": "ISO8601" }
{ "type": "block:update", "note_id": "uuid", "block_id": "uuid", "data": {...}, "client_id": "string", "timestamp": "ISO8601" }
{ "type": "block:remove", "note_id": "uuid", "block_id": "uuid", "client_id": "string", "timestamp": "ISO8601" }
{ "type": "block:move",   "note_id": "uuid", "block_id": "uuid", "new_position": 2, "new_parent_id": null, "client_id": "string", "timestamp": "ISO8601" }

// Block sync batch (response to sync_request)
{
  "type": "block:sync_batch",
  "note_id": "uuid",
  "blocks": [
    { "id": "uuid", "block_type": "paragraph", "data": { "text": "..." }, "position": 0, "parent_id": null }
  ],
  "timestamp": "ISO8601"
}

// Error
{ "type": "error", "message": "string", "timestamp": "ISO8601" }
```

### Multi-Instance Sync

```
Instance 1 --+
             +-> Redis Pub/Sub --> All Instances
Instance 2 --+
```

### WS Rate Limiting & Security

- Maximum 5 concurrent WebSocket connections per user account
- Connections exceeding the limit are rejected immediately with an error message
- Counter decremented on clean disconnect or timeout (60s without ping)
- JWT never appears in URL query strings; short-lived (30s) single-use tickets are obtained via `POST /api/v1/ws/ticket`

---

## Deployment

### Cloud Database Setup

#### Supabase (PostgreSQL)

1. Go to [supabase.com](https://supabase.com/)
2. Create new project
3. Get connection string from **Settings -> Database**
4. Use **Session Mode** connection pooler:

```env
DATABASE_URL=postgresql://postgres.[PROJECT_REF]:[PASSWORD]@aws-0-[REGION].pooler.supabase.com:5432/postgres
```

#### Upstash (Redis)

1. Go to [upstash.com](https://upstash.com/)
2. Create Redis database
3. Copy connection string:

```env
REDIS_URL=rediss://default:[PASSWORD]@[ENDPOINT].upstash.io:6379
```

### Deployment Options

#### Option 1: Railway.app (Recommended)

1. Push code to GitHub
2. Connect repository at [railway.app](https://railway.app/)
3. Add PostgreSQL and Redis services
4. Set environment variables
5. Deploy automatically

#### Option 2: Render.com

1. Create Web Service at [render.com](https://render.com/)
2. Build command: `cargo build --release`
3. Start command: `./target/release/noteflow-backend`
4. Add environment variables
5. Deploy

#### Option 3: Fly.io

```bash
flyctl launch
flyctl deploy
```

#### Option 4: Docker

```bash
# Build image
docker build -t noteflow-backend .

# Run container
docker run -p 8080:8080 --env-file .env noteflow-backend
```

### Environment Variables (Production)

```env
# Database
DATABASE_URL=<supabase-connection-string>
REDIS_URL=<upstash-connection-string>

# Security
JWT_SECRET=<openssl rand -base64 32>
ENCRYPTION_KEY=<32-bytes-as-hex>

# Logging
RUST_LOG=info

# Email (Brevo, 300 emails/day free)
BREVO_API_KEY=<your-brevo-api-key>
BREVO_FROM_EMAIL=noreply@noteflow.app

# Self URL (for health check self-ping + reset links)
SELF_URL=https://your-app.onrender.com

# Web Push VAPID (auto-generated if empty)
VAPID_PUBLIC_KEY=
VAPID_PRIVATE_KEY=
VAPID_SUBJECT=mailto:notifications@noteflow.app

# ImageKit (free tier avatar CDN)
IMAGEKIT_PRIVATE_KEY=<your-imagekit-private-key>

```

---

## Project Structure

```
noteflow-backend/
+-- Cargo.toml                          # Rust dependencies
+-- .env.example                        # Environment template
+-- Dockerfile                          # Container build
+-- docker-compose.yml                  # Local dev stack
+-- README.md                           # This file
+-- LICENSE                             # MIT License

+-- .sqlx/                              # SQLx offline query cache (builds without live DB)

+-- migrations/                         # Database migrations
|   +-- 20251208000001_create_users.sql
|   +-- 20251208000002_create_notes.sql
|   +-- 20251208000003_create_revisions.sql
|   +-- 20251208000004_create_tags.sql
|   +-- 20251208000005_create_active_sessions.sql
|   +-- 20251213000006_enhance_notes.sql
|   +-- 20251213000007_enhance_users.sql
|   +-- 20251213000008_create_refresh_tokens.sql
|   +-- 20251213000009_fix_active_sessions.sql
|   +-- 20251213000010_create_push_subscriptions.sql
|   +-- 20251213000011_create_note_collaborators.sql    
|   +-- 20251213000012_create_collab_operations.sql     
|   +-- 20251213000013_add_account_lockout.sql          
|   +-- 20251213000014_add_missing_indexes.sql          
|   +-- 20260729000001_create_note_blocks.sql

+-- src/
    +-- main.rs                         # Application entry, router, Prometheus setup
    +-- lib.rs                          # Library exports
    +-- config.rs                       # Configuration

    +-- models/                         # Data models
    |   +-- mod.rs
    |   +-- user.rs                     # User + auth request/response types
    |   +-- note.rs                     # Note + collaborator + filter types
    |   +-- revision.rs                 # Revision models
    |   +-- tag.rs                      # Tag models
    |   +-- session.rs                  # Active session model
    |   +-- block.rs                     # Block model
    |   +-- collaboration.rs            # WsMessage enum + block CRDT types

    +-- db/                             # Database layer
    |   +-- mod.rs
    |   +-- postgres.rs                 # PostgreSQL pool
    |   +-- redis.rs                    # Redis connection manager

    +-- services/                       # Business logic
    |   +-- mod.rs
    |   +-- auth_service.rs             # Authentication, tokens, lockout
    |   +-- note_service.rs             # Note CRUD, batch loading, search, cache
    |   +-- note_collaborator_service.rs # Share/invite/permission management
    |   +-- tag_service.rs              # Tag CRUD, note assignment
    |   +-- user_service.rs             # Profile, avatar, preferences
    |   +-- revision_service.rs         # Version history
    |   +-- notification_service.rs     # Web Push + Brevo email
    |   +-- export_service.rs           # Multi-format export with styling (markdown, html, txt, rtf, pdf, epub)
    |   +-- collaboration_service.rs    # WS lifecycle, block CRDT relay

    +-- handlers/                       # HTTP + WebSocket handlers
    |   +-- mod.rs
    |   +-- auth.rs                     # Auth endpoints
    |   +-- ws_ticket.rs               # POST /api/v1/ws/ticket (short-lived WS ticket)
    |   +-- notes.rs                    # Note CRUD, favorite, archive, export, search
    |   +-- tags.rs                     # Tag management
    |   +-- collaborators.rs            # Share/invite management
    |   +-- users.rs                    # Profile, preferences, avatar
    |   +-- notifications.rs            # Push subscribe/unsubscribe
    |   +-- revisions.rs                # Version history
    |   +-- websocket.rs                # WebSocket upgrade handler

    +-- middleware/                     # Tower middleware
    |   +-- mod.rs
    |   +-- auth.rs                     # JWT verification
    |   +-- rate_limit.rs               # IP + per-email rate limiting
    |   +-- request_id.rs               # UUID per request + X-Request-Id

    +-- utils/                          # Utilities
        +-- mod.rs
        +-- errors.rs                   # AppError + Result types
        +-- jwt.rs                      # JWT manager
        +-- validation.rs               # Input sanitization + validation
        +-- web_push.rs                 # Pure Rust Web Push encryption
```

---

## Security

### Authentication
- JWT tokens with HS256 algorithm (HMAC-SHA256)
- Short-lived access tokens (1h) and long-lived refresh tokens (30d)
- Token rotation: every refresh invalidates the old token and issues a new one
- Session listing and revocation from any device
- Stateless design with no server-side session storage

### Account Lockout
- 10 consecutive failed login attempts → account locked for 15 minutes
- `failed_login_attempts` counter resets on successful login
- Locked accounts return 429 regardless of password correctness
- Defends against credential stuffing and brute-force attacks

### Password Security
- Bcrypt hashing with cost factor 12
- Unique salt generation per password
- Passwords never stored or logged in plain text
- Timing-safe comparison to prevent timing attacks

### Input Validation
- RFC 5322 compliant email validation via `validator` crate
- Minimum 8-character password requirement (max 128)
- Content sanitization and size limits on all inputs
- Parameterized queries prevent SQL injection
- Request body size limit: 5MB (configurable via `RequestBodyLimitLayer`)

### Rate Limiting
- IP-based sliding window algorithm
- Per-email brute-force protection on login/register (5 requests per 60 seconds)
- Global limits enforced via `X-RateLimit-Limit` / `X-RateLimit-Remaining` response headers
- WebSocket connection limit: max 5 concurrent connections per user
- Background cleanup prevents memory leaks

### Layered Middleware Stack
```
Incoming Request
  → Extension(PrometheusHandle) (metrics)
  → CorsLayer                    (CORS)
  → RequestBodyLimitLayer        (5MB cap)
  → CompressionLayer             (gzip)
  → RequestId middleware         (UUID + X-Request-Id header)
  → TraceLayer                   (structured span logging)
  → Rate Limit middleware        (IP + per-email limits)
  → Auth middleware / OptionalAuth middleware  (JWT/ticket verification)
    → Handler
```

### Push Notification Security
- VAPID authentication with signed JWT per push request
- AES-256-GCM payload encryption with ECDH key agreement
- Pure Rust cryptography implementation (no system OpenSSL)
- No shared push secrets across endpoints

### Error Handling
- All errors follow a consistent `{ "error": "...", "status": N }` shape
- Internal errors logged server-side but return generic messages to clients
- Stack traces never exposed in production responses
- Graceful shutdown handler (SIGTERM/SIGINT) drains in-flight requests

### Additional Measures
- Configurable CORS origin policies
- HTTPS enforcement in production
- Structured JSON logging with no secrets in output

---

## Contributing

Contributions are welcome. Please follow these guidelines:

1. Fork the repository
2. Create feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit changes (`git commit -m 'Add AmazingFeature'`)
4. Push to branch (`git push origin feature/AmazingFeature`)
5. Open Pull Request

### Development Guidelines

- Follow Rust conventions and `rustfmt` formatting
- Write tests for new features
- Update documentation as needed
- Ensure all tests pass (`cargo test`)
- Run clippy lints (`cargo clippy`)

---

## License

Distributed under the [MIT License](LICENSE). See `LICENSE` file for more information.

---

## Contact

**Zaud Rehman** - [@RehmanZaud](https://x.com/RehmanZaud) | [LinkedIn](https://www.linkedin.com/in/zaud-rehman-31514a288/) | zaudrehman@gmail.com

**Project Link**: [https://github.com/ZaudRehman/noteflow-backend-v1](https://github.com/ZaudRehman/noteflow-backend-v1)

---

## Acknowledgments

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - SQL toolkit
- [Tokio](https://tokio.rs/) - Async runtime
- [PostgreSQL](https://www.postgresql.org/) - Database
- [Redis](https://redis.io/) - In-memory store
- [Supabase](https://supabase.com/) - Database hosting
- [Upstash](https://upstash.com/) - Redis hosting
