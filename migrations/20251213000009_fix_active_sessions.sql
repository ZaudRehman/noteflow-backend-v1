-- migrations/20251213000009_fix_active_sessions.sql
-- Ensure active_sessions table is properly configured

-- Drop and recreate if exists with wrong structure
DROP TABLE IF EXISTS active_sessions CASCADE;

CREATE TABLE active_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cursor_line INTEGER DEFAULT 0,
    cursor_column INTEGER DEFAULT 0,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(note_id, user_id)
);

-- Indexes for real-time queries
CREATE INDEX idx_active_sessions_note_id ON active_sessions(note_id, last_seen_at DESC);
CREATE INDEX idx_active_sessions_user_id ON active_sessions(user_id);
CREATE INDEX idx_active_sessions_last_seen ON active_sessions(last_seen_at) WHERE last_seen_at > NOW() - INTERVAL '5 minutes';

-- Auto-cleanup stale sessions (consider users inactive after 5 minutes)
CREATE OR REPLACE FUNCTION cleanup_stale_sessions()
RETURNS void AS $$
BEGIN
  DELETE FROM active_sessions 
  WHERE last_seen_at < NOW() - INTERVAL '5 minutes';
END;
$$ LANGUAGE plpgsql;

COMMENT ON TABLE active_sessions IS 'Tracks real-time collaboration sessions and cursor positions';
