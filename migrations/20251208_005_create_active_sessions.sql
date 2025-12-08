-- Create active_sessions table for WebSocket presence tracking
CREATE TABLE IF NOT EXISTS active_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    connection_id VARCHAR(255) NOT NULL,
    last_active TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_active_sessions_note_id ON active_sessions(note_id);
CREATE INDEX idx_active_sessions_user_id ON active_sessions(user_id);
CREATE INDEX idx_active_sessions_connection ON active_sessions(connection_id);
CREATE INDEX idx_active_sessions_last_active ON active_sessions(last_active);

-- Function to cleanup stale sessions (older than 5 minutes)
CREATE OR REPLACE FUNCTION cleanup_stale_sessions()
RETURNS void AS $$
BEGIN
    DELETE FROM active_sessions
    WHERE last_active < NOW() - INTERVAL '5 minutes';
END;
$$ LANGUAGE plpgsql;