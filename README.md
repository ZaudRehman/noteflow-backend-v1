# NoteFlow Backend 🚀

<div align="center">

**A production-grade REST API for real-time collaborative note-taking built with Rust, Axum, PostgreSQL, and Redis**

![Rust](https://img.shields.io/badge/Rust-1.75+-000000?style=for-the-badge&logo=rust&logoColor=white) ![Axum](https://img.shields.io/badge/Axum-0.7-EC5800?style=for-the-badge&logo=rust&logoColor=white) ![PostgreSQL](https://img.shields.io/badge/PostgreSQL-15+-4169E1?style=for-the-badge&logo=postgresql&logoColor=white) ![Redis](https://img.shields.io/badge/Redis-7+-DC382D?style=for-the-badge&logo=redis&logoColor=white) ![Supabase](https://img.shields.io/badge/Supabase-3FCF8E?style=for-the-badge&logo=supabase&logoColor=white) ![Upstash](https://img.shields.io/badge/Upstash-00E9A3?style=for-the-badge&logo=upstash&logoColor=white)

[Live Demo](#) · [API Docs](#api-documentation) · [Report Bug](https://github.com/ZaudRehman/noteflow-backend-v1/issues) · [Request Feature](https://github.com/ZaudRehman/noteflow-backend-v1/issues)

</div>

---

## 📖 Table of Contents

- [About](#-about-the-project)
- [Features](#-key-features)
- [Tech Stack](#️-tech-stack)
- [Architecture](#️-architecture)
- [Getting Started](#-getting-started)
- [API Documentation](#-api-documentation)
- [Database Schema](#️-database-schema)
- [WebSocket Integration](#-websocket-integration)
- [Deployment](#-deployment)
- [Project Structure](#-project-structure)
- [Security](#-security)
- [Contributing](#-contributing)
- [License](#-license)
- [Contact](#-contact)

---

## 🎯 About The Project

NoteFlow Backend is a **high-performance REST API** built with Rust that powers a collaborative note-taking application. Designed with production readiness in mind, it demonstrates modern backend development practices including asynchronous programming, JWT authentication, real-time WebSocket communication and cloud-native deployment.

### Why NoteFlow Backend?

This project showcases mastery of critical backend engineering competencies:

- **High Performance** - Built with Rust for blazing-fast response times (<200ms p95)
- **Enterprise Security** - JWT authentication, bcrypt hashing, rate limiting
- **Scalable Architecture** - Async/await patterns, connection pooling, horizontal scaling
- **Real-Time Sync** - WebSocket support with Redis pub/sub for multi-instance coordination
- **Version Control** - Automatic revision history via PostgreSQL triggers
- **Smart Organization** - Tag system with many-to-many relationships
- **Production Ready** - Comprehensive error handling, structured logging, health checks
- **Cloud Native** - Docker support, Supabase/Upstash integration, multiple deployment options

### What This Demonstrates

- Building production-grade APIs with Rust and Axum framework
- Implementing JWT-based authentication with refresh tokens
- Working with PostgreSQL using SQLx with compile-time query verification
- Redis integration for caching and pub/sub messaging
- WebSocket real-time communication architecture
- Database migration management with version control
- Async/await programming patterns in Rust
- Cloud service integration (Supabase, Upstash)
- Docker containerization and orchestration
- RESTful API design following industry standards

---

## ✨ Key Features

### Authentication & Authorization
- **JWT Token System** - Dual token approach with access (1h) and refresh (30d) tokens
- **Secure Password Storage** - Bcrypt hashing with configurable cost factor
- **Token Refresh Flow** - Seamless token renewal without re-authentication
- **User Management** - Registration, login, and session management

### Note Management
- **Full CRUD Operations** - Create, read, update, delete with ownership verification
- **Favorites & Archive** - Mark important notes and archive old ones
- **Soft Delete** - 30-day recovery window for deleted notes
- **Advanced Filtering** - Filter by favorite, archive, tags, search query
- **Pagination** - Efficient data retrieval with configurable page sizes
- **Sorting** - Sort by created, updated, or title with ASC/DESC
- **Content Limits** - Configurable max size (100KB) and notes per user (500)

### Tag System
- **Custom Tags** - User-specific tags with unique names
- **Many-to-Many Relations** - Multiple tags per note, multiple notes per tag
- **Tag CRUD** - Create, update, delete tags independently
- **Tag Statistics** - View note counts per tag
- **Batch Operations** - Add/remove tags from notes
- **Tag-Based Filtering** - List all notes with specific tag

### Search & Discovery
- **Full-Text Search** - PostgreSQL GIN indexes for fast content search
- **Multi-Field Search** - Search across titles and content
- **Ranking** - Results ranked by relevance (title match > content match)
- **Fuzzy Matching** - ILIKE queries for partial matches
- **Real-Time Results** - Sub-second search response times

### User Management
- **Profile Customization** - Update display name and avatar
- **Theme Settings** - Light/dark/auto theme preferences
- **Custom Preferences** - JSONB field for arbitrary user settings
- **Last Login Tracking** - Monitor user activity
- **Account Security** - Password change and reset flows

### Version History
- **Automatic Revisions** - PostgreSQL triggers create snapshots on content changes
- **Revision Browsing** - List all historical versions with metadata
- **Point-in-Time Restore** - Revert notes to any previous version
- **Change Tracking** - Author and timestamp for every revision

### Real-Time Collaboration
- **WebSocket Infrastructure** - Persistent connections for instant updates
- **Active User Presence** - See who's currently editing a note
- **Cursor Tracking** - View collaborator cursor positions in real-time
- **Multi-Instance Sync** - Redis pub/sub broadcasts across servers
- **Connection Management** - Auto-cleanup of stale sessions
- **Message Types** - Note updates, cursor moves, user join/leave events

### Security & Performance
- **Rate Limiting** - IP-based throttling (20/min anonymous, 100/min authenticated)
- **Input Validation** - Comprehensive sanitization and format checking
- **SQL Injection Prevention** - Parameterized queries via SQLx
- **CORS Configuration** - Customizable cross-origin policies
- **Connection Pooling** - Optimized database connection management (20 max)
- **Indexed Queries** - Composite indexes for fast lookups
- **Compression** - Automatic gzip compression for responses

---

## 🛠️ Tech Stack

### Backend Framework
- **[Axum](https://github.com/tokio-rs/axum)** 0.7 - Ergonomic and modular web framework
- **[Tokio](https://tokio.rs/)** - Async runtime with multi-threading
- **[Tower](https://github.com/tower-rs/tower)** - Middleware and service abstractions
- **[Tower-HTTP](https://github.com/tower-rs/tower-http)** - CORS, compression, tracing middleware

### Database & Caching
- **[PostgreSQL](https://www.postgresql.org/)** 15+ - Relational database with ACID guarantees
- **[SQLx](https://github.com/launchbadge/sqlx)** 0.7 - Async SQL toolkit with compile-time verification
- **[Redis](https://redis.io/)** 7+ - In-memory data store for caching and pub/sub
- **[Supabase](https://supabase.com/)** - Managed PostgreSQL with connection pooling
- **[Upstash](https://upstash.com/)** - Serverless Redis with TLS support

### Authentication & Security
- **[jsonwebtoken](https://github.com/Keats/jsonwebtoken)** - JWT implementation with HS256/RS256
- **[bcrypt](https://github.com/Keats/rust-bcrypt)** - Password hashing with salt rounds
- **[uuid](https://github.com/uuid-rs/uuid)** - Universally unique identifiers
- **[validator](https://github.com/Keats/validator)** - Struct validation with derive macros
- **[sha2](https://github.com/RustCrypto/hashes)** - Token hashing
- **[rand](https://github.com/rust-random/rand)** - Secure random generation

### Serialization & Validation
- **[Serde](https://serde.rs/)** - Serialization framework for JSON/YAML/TOML
- **[serde_json](https://github.com/serde-rs/json)** - JSON support for Serde
- **[chrono](https://github.com/chronotope/chrono)** - Date and time library

### Configuration & Logging
- **[dotenvy](https://github.com/allan2/dotenvy)** - Environment variable management
- **[tracing](https://github.com/tokio-rs/tracing)** - Structured logging and diagnostics
- **[tracing-subscriber](https://github.com/tokio-rs/tracing)** - Log formatting and filtering

### DevOps & Deployment
- **[Docker](https://www.docker.com/)** - Containerization with multi-stage builds
- **[Docker Compose](https://docs.docker.com/compose/)** - Local development orchestration
- **GitHub Actions** - CI/CD pipeline

---

## 🏗️ Architecture

NoteFlow Backend follows a **clean layered architecture** inspired by Domain-Driven Design:

```
┌─────────────────────────────────────────────────────────────┐
│                       Axum Web Server                       │
│                    (Tower Middleware Stack)                 │
├─────────────────────────────────────────────────────────────┤
│  Middleware Layer                                           │
│  ├─ CORS                    (Cross-origin resource sharing) │
│  ├─ Compression             (Gzip compression)              │
│  ├─ Request Tracing         (Structured logging)            │
│  ├─ Rate Limiting           (IP-based throttling)           │
│  └─ Authentication          (JWT verification)              │
├─────────────────────────────────────────────────────────────┤
│  Presentation Layer (HTTP Handlers)                         │
│  ├─ Auth Routes             (register, login, refresh)      │
│  ├─ Note Routes             (CRUD operations)               │
│  ├─ Revision Routes         (history, restore)              │
│  ├─ Tag Routes              (tag management)                │
│  └─ WebSocket Handler       (real-time messaging)           │
├─────────────────────────────────────────────────────────────┤
│  Business Logic Layer (Services)                            │
│  ├─ AuthService             (authentication & tokens)       │
│  ├─ NoteService             (note operations)               │
│  ├─ UserService             (user management)               │
│  ├─ TagService              (tagging system)                │
│  └─ CollaborationService        (real-time sync)            │
├─────────────────────────────────────────────────────────────┤
│  Data Access Layer                                          │
│  ├─ SQLx Queries            (parameterized SQL)             │
│  ├─ Connection Pool         (PostgreSQL sessions)           │
│  ├─ Redis Manager           (pub/sub, caching)              │
│  └─ Migration Manager       (schema versioning)             │
├─────────────────────────────────────────────────────────────┤
│  Infrastructure                                             │
│  ├─ PostgreSQL 15+          (primary data store)            │
│  ├─ Redis 7+                (cache & pub/sub)               │
│  └─ File System             (static assets)                 │
└─────────────────────────────────────────────────────────────┘
```

### Design Patterns

- **Repository Pattern** - Data access abstraction through services
- **Dependency Injection** - Axum's State for service sharing
- **Middleware Pattern** - Request/response transformation pipeline
- **Factory Pattern** - JWT token generation and validation
- **Observer Pattern** - Redis pub/sub for real-time events
- **Strategy Pattern** - Configurable rate limiting and validation

### Data Flow

```
HTTP Request → Middleware → Handler → Service → Database → Response
                   ↓
              [Rate Limit]
              [Auth Check]
              [Validation]
                   ↓
                Response
```

---

## 🚀 Getting Started

### Prerequisites

Ensure you have these installed:

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **PostgreSQL 15+** - [Download](https://www.postgresql.org/download/) or use [Supabase](https://supabase.com/)
- **Redis 7+** - [Download](https://redis.io/download) or use [Upstash](https://upstash.com/)
- **SQLx CLI** - For database migrations
- **Git** - Version control

### Local Development Setup

#### 1. **Clone the repository**
```bash
git clone https://github.com/ZaudRehman/noteflow-backend-v1.git
cd noteflow-backend-v1
```

#### 2. **Install SQLx CLI**
```bash
cargo install sqlx-cli --features postgres
```

#### 3. **Configure environment variables**

Create `.env` file:
```bash
cp .env.example .env
```

Edit `.env` with your credentials:
```env
# Server
HOST=0.0.0.0
PORT=8080
RUST_LOG=info,noteflow_backend=debug

# PostgreSQL (Local or Supabase)
DATABASE_URL=postgresql://user:password@localhost:5432/noteflow
DATABASE_MAX_CONNECTIONS=20

# Redis (Local or Upstash)
REDIS_URL=redis://localhost:6379
# Or for Upstash: rediss://default:password@endpoint.upstash.io:6379

# JWT (Generate: openssl rand -base64 32)
JWT_SECRET=your-super-secret-key-minimum-32-characters
JWT_ACCESS_EXPIRATION=86400
JWT_REFRESH_EXPIRATION=604800

# Limits
MAX_NOTE_SIZE=102400
MAX_NOTES_PER_USER=50
RATE_LIMIT_ANONYMOUS=20
RATE_LIMIT_AUTHENTICATED=100
```

#### 4. **Setup database**
```bash
# Create database
sqlx database create

# Run migrations
sqlx migrate run
```

#### 5. **Build and run**
```bash
# Development mode (with auto-reload)
cargo watch -x run

# Or standard run
cargo run

# Production build
cargo build --release
./target/release/noteflow-backend
```

#### 6. **Verify installation**
```bash
# Health check
curl http://localhost:8080/health

# Expected response: "OK"
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

## 📚 API Documentation

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

#### Authentication (Public)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/auth/register` | Register new user (also stores refresh token) |
| `POST` | `/api/v1/auth/login` | Login (stores refresh token, updates last_login) |
| `POST` | `/api/v1/auth/refresh` | Refresh access token |
| `POST` | `/api/v1/auth/forgot-password` | Request password reset email |
| `POST` | `/api/v1/auth/reset-password` | Reset password with token |

#### Authentication (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/auth/me` | Get current user profile |
| `POST` | `/api/v1/auth/logout` | Revoke refresh token |

#### Notes (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/notes` | List notes (filters: favorites/archived/all) |
| `POST` | `/api/v1/notes` | Create new note |
| `GET` | `/api/v1/notes/:id` | Get specific note with tags |
| `PUT` | `/api/v1/notes/:id` | Update note |
| `DELETE` | `/api/v1/notes/:id` | Soft delete note |
| `POST` | `/api/v1/notes/:id/favorite` | Toggle favorite status |
| `POST` | `/api/v1/notes/:id/archive` | Toggle archive status |

#### Tags

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/tags` | List all user tags with note counts |
| `POST` | `/api/v1/tags` | Create new tag |
| `PUT` | `/api/v1/tags/:id` | Update tag name |
| `DELETE` | `/api/v1/tags/:id` | Delete tag |
| `GET` | `/api/v1/tags/:id/notes` | Get all notes with this tag |
| `POST` | `/api/v1/notes/:note_id/tags` | Add tag to note |
| `DELETE` | `/api/v1/notes/:note_id/tags/:tag_id` | Remove tag from note |

#### User Profile (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/users/profile` | Get user profile |
| `PUT` | `/api/v1/users/profile` | Update profile (name, avatar) |
| `PUT` | `/api/v1/users/preferences` | Update preferences (theme, custom JSON) |

#### Search (Protected)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/search?q=query` | Full-text search across notes |

#### WebSocket (Protected)

| Protocol | Endpoint | Description |
|----------|----------|-------------|
| `WS` | `/api/v1/notes/:id/ws` | Real-time collaboration |

#### Health Check

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | API health status |


### HTTP Status Codes

| Code | Status | Usage |
|------|--------|-------|
| `200` | OK | Successful GET, PUT requests |
| `201` | Created | Successful POST requests |
| `204` | No Content | Successful DELETE requests |
| `400` | Bad Request | Invalid input/validation errors |
| `401` | Unauthorized | Missing or invalid authentication |
| `403` | Forbidden | Insufficient permissions |
| `404` | Not Found | Resource doesn't exist |
| `409` | Conflict | Duplicate resource (email exists) |
| `429` | Too Many Requests | Rate limit exceeded |
| `500` | Internal Server Error | Server-side errors |

---

## 🗄️ Database Schema

### Entity Relationship Diagram

```
                    ┌──────────────────────────┐
                    │         users            │
                    ├──────────────────────────┤
                    │ id (PK, UUID)            │
                    │ email (UNIQUE)           │
                    │ password_hash            │
                    │ display_name             │
                    │ avatar_url               │
                    │ theme                    │
                    │ preferences (JSONB)      │
                    │ reset_token              │
                    │ reset_token_expires      │
                    │ last_login_at            │
                    │ created_at               │
                    │ updated_at               │
                    └────────┬─────────────────┘
                             │
                             │ 1:N 
              ┌──────────────┼──────────────┬──────────────┐
              │              │              │              │
              ▼              ▼              ▼              ▼
    ┌─────────────────┐ ┌─────────────┐ ┌──────────────┐ ┌──────────────────┐
    │     notes       │ │    tags     │ │  revisions   │ │ refresh_tokens   │
    ├─────────────────┤ ├─────────────┤ ├──────────────┤ ├──────────────────┤
    │ id (PK)         │ │ id (PK)     │ │ id (PK)      │ │ id (PK)          │
    │ user_id (FK)    │ │ user_id(FK) │ │ note_id (FK) │ │ user_id (FK)     │
    │ title           │ │ name        │ │ content      │ │ token_hash       │
    │ content         │ │ created_at  │ │ created_by   │ │ expires_at       │
    │ is_favorited    │ └──────┬──────┘ │ created_at   │ │ revoked          │
    │ is_archived     │        │        └──────────────┘ │ revoked_at       │
    │ is_deleted      │        │                         │ user_agent       │
    │ last_edited_by  │        │ N:M                     │ ip_address       │
    │ created_at      │        │                         │ created_at       │
    │ updated_at      │   ┌────┴─────────┐               └──────────────────┘
    └────────┬────────┘   │  note_tags   │
             │            ├──────────────┤
             │            │ note_id (FK) │
             │            │ tag_id (FK)  │
             │            │ created_at   │
             │            └──────────────┘
             │ 1:N 
             │
             ▼
    ┌──────────────────┐
    │ active_sessions  │
    ├──────────────────┤
    │ id (PK)          │
    │ note_id (FK)     │
    │ user_id (FK)     │
    │ cursor_line      │
    │ cursor_column    │
    │ last_seen_at     │
    │ created_at       │
    └──────────────────┘

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

-- Performance indexes
CREATE INDEX idx_notes_user_id ON notes(user_id);
CREATE INDEX idx_notes_user_filters ON notes(user_id, is_deleted, is_archived, is_favorited, updated_at DESC);
CREATE INDEX idx_notes_user_favorited ON notes(user_id, updated_at DESC) WHERE is_favorited = true;
CREATE INDEX idx_notes_user_archived ON notes(user_id, updated_at DESC) WHERE is_archived = true;

-- Full-text search
CREATE INDEX idx_notes_content_search ON notes USING GIN (to_tsvector('english', content));
CREATE INDEX idx_notes_title_search ON notes USING GIN (to_tsvector('english', title));
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

-- Automatic revision trigger
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
CREATE INDEX idx_active_sessions_user_id ON active_sessions(user_id);
WHERE last_seen_at > NOW() - INTERVAL '5 minutes';
```

### Database Optimizations

- **Composite Indexes** - Fast user-specific queries
- **Full-Text Search** - GIN indexes for content search
- **Foreign Key Constraints** - Referential integrity
- **Cascade Deletes** - Automatic cleanup of related data
- **Automatic Timestamps** - Trigger-based updated_at
- **Soft Deletes** - Recovery of deleted notes
- **Connection Pooling** - Efficient resource usage

---

## 🔄 WebSocket Integration

### Connection Flow

```rust
Client connects: ws://api/v1/notes/{note_id}/ws
└─ Authorization: Bearer <token>

Server verifies JWT and note access
└─ Creates active_session record

Client sends cursor moves or edits
└─ Broadcast to all connections for this note
└─ Publish to Redis for other instances

Client receives updates from other users
└─ Update UI with changes and cursor positions

Client disconnects
└─ Delete active_session record
└─ Broadcast "user_left" event
```

### Message Types

```typescript
// Cursor movement
{
"type": "cursor:move",
"note_id": "uuid",
"user_id": "uuid",
"user_name": "John Doe",
"position": { "line": 5, "column": 12 },
"timestamp": "2025-12-13T..."
}

// Note update
{
"type": "note:updated",
"note_id": "uuid",
"user_id": "uuid",
"content_delta": "new text",
"timestamp": "2025-12-13T..."
}

// User joined
{
"type": "user:joined",
"note_id": "uuid",
"user_id": "uuid",
"user_name": "Jane Smith",
"timestamp": "2025-12-13T..."
}

// Ping/Pong (keepalive)
{
"type": "ping",
"timestamp": "2025-12-13T..."
}
```

### Multi-Instance Sync

```
Instance 1 ──┐
             ├─→ Redis Pub/Sub ──→ All Instances
Instance 2 ──┘
```

---

## 🚀 Deployment

### Cloud Database Setup

#### Supabase (PostgreSQL)

1. Go to [supabase.com](https://supabase.com/)
2. Create new project
3. Get connection string from **Settings → Database**
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
DATABASE_URL=<supabase-connection-string>
REDIS_URL=<upstash-connection-string>
JWT_SECRET=<secure-random-key>
JWT_ACCESS_EXPIRATION=3600
JWT_REFRESH_EXPIRATION=2592000
RUST_LOG=info
MAX_NOTE_SIZE=102400
MAX_NOTES_PER_USER=50
RATE_LIMIT_ANONYMOUS=20
RATE_LIMIT_AUTHENTICATED=100
```

---

## 📂 Project Structure

```
noteflow-backend/
├── 📄 Cargo.toml                     # Rust dependencies
├── 📄 .env.example                   # Environment template
├── 📄 Dockerfile                     # Container build
├── 📄 docker-compose.yml             # Local dev stack
├── 📄 README.md                      # This file
├── 📄 LICENSE                        # MIT License
│
├── 📁 migrations/                    # Database migrations
│   ├── 20251208000001_create_users.sql
│   ├── 20251208000002_create_notes.sql
│   ├── 20251208000003_create_revisions.sql
│   ├── 20251208000004_create_tags.sql
│   ├── 20251208000005_create_active_sessions.sql
│   ├── 20251213000006_enhance_notes.sql
│   ├── 20251213000007_enhance_users.sql
│   ├── 20251213000008_create_refresh_tokens.sql
│   └── 20251213000009_fix_active_sessions.sql
│
└── 📁 src/
    ├── 📄 main.rs                    # Application entry
    ├── 📄 lib.rs                     # Library exports
    ├── 📄 config.rs                  # Configuration
    │
    ├── 📁 utils/                     # Utilities
    │   ├── mod.rs
    │   ├── errors.rs                 # Error handling
    │   ├── jwt.rs                    # JWT manager
    │   └── validation.rs             # Input validation
    │
    ├── 📁 models/                    # Data models
    │   ├── mod.rs
    │   ├── user.rs                   # User model
    │   ├── note.rs                   # Note model
    │   ├── revision.rs               # Revision model
    │   ├── tag.rs                    # Tag model
    │   ├── session.rs                # Active session mode
    │   └── collaboration.rs          # WebSocket message types
    │
    ├── 📁 db/                        # Database layer
    │   ├── mod.rs
    │   ├── postgres.rs               # PostgreSQL pool
    │   └── redis.rs                  # Redis manager
    │
    ├── 📁 services/                  # Business logic
    │   ├── mod.rs
    │   ├── auth_service.rs           # Authentication
    │   ├── note_service.rs           # Note operations
    │   ├── tag_service.rs            # Tag management
    │   ├── user_service.rs           # User profile management
    │   └── collaboration_service.rs  # Real-time collaboration
    │
    ├── 📁 handlers/                  # HTTP handlers
    │   ├── mod.rs
    │   ├── auth.rs                   # Auth endpoints
    │   ├── notes.rs                  # Note endpoints
    │   ├── tags.rs                   # Tag management endpoints
    │   ├── users.rs                  # User profile endpoints
    │   ├── search.rs                 # Search endpoint
    │   └── websocket.rs              # WebSocket handler
    │
    └── 📁 middleware/                # Middleware
        ├── mod.rs
        ├── auth.rs                   # JWT verification
        └── rate_limit.rs             # Rate limiting
```

---

## 🔒 Security

### Authentication
- **JWT Tokens** - Industry-standard JSON Web Tokens
- **Token Expiration** - Short-lived access (24h) + refresh (7d)
- **Stateless Design** - No server-side session storage
- **Secure Defaults** - HS256 algorithm with strong secrets

### Password Security
- **Bcrypt Hashing** - Industry-standard with cost factor 10
- **Salt Generation** - Unique salt per password
- **No Plain Text** - Passwords never stored or logged
- **Timing-Safe Comparison** - Prevents timing attacks

### Input Validation
- **Email Validation** - RFC 5322 compliant
- **Password Strength** - Minimum 8 characters
- **Content Sanitization** - Trim and validate all inputs
- **Size Limits** - Configurable maximum sizes
- **SQL Injection Prevention** - Parameterized queries

### Rate Limiting
- **IP-Based Throttling** - Sliding window algorithm
- **Anonymous Limits** - 20 requests/minute
- **Authenticated Limits** - 100 requests/minute
- **Background Cleanup** - Prevents memory leaks

### Additional Measures
- **CORS Configuration** - Controlled origin access
- **TLS Support** - HTTPS enforcement in production
- **Error Sanitization** - No sensitive data in errors
- **Structured Logging** - Audit trail without secrets

---

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork the repository**
2. **Create feature branch** (`git checkout -b feature/AmazingFeature`)
3. **Commit changes** (`git commit -m 'Add AmazingFeature'`)
4. **Push to branch** (`git push origin feature/AmazingFeature`)
5. **Open Pull Request**

### Development Guidelines

- Follow Rust conventions and `rustfmt` formatting
- Write tests for new features
- Update documentation as needed
- Ensure all tests pass (`cargo test`)
- Run clippy lints (`cargo clippy`)

---

## 📄 License

Distributed under the [**MIT License**](LICENSE). See `LICENSE` file for more information.

---

## 📧 Contact

**Zaud Rehman** - [@RehmanZaud](https://x.com/RehmanZaud) · [LinkedIn](https://www.linkedin.com/in/zaud-rehman-31514a288/) · zaudrehman@gmail.com

**Project Link**: [https://github.com/ZaudRehman/noteflow-backend-v1](https://github.com/ZaudRehman/noteflow-backend-v1)

---

## 🙏 Acknowledgments

Built with these amazing open-source technologies:

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - SQL toolkit
- [Tokio](https://tokio.rs/) - Async runtime
- [PostgreSQL](https://www.postgresql.org/) - Database
- [Redis](https://redis.io/) - In-memory store
- [Supabase](https://supabase.com/) - Database hosting
- [Upstash](https://upstash.com/) - Redis hosting

---

<div align="center">

### ⭐ Star this repository if you find it helpful!

**Built with 🦀 Rust and ❤️ for performance**

[⬆ Back to Top](#noteflow-backend-)

</div>
