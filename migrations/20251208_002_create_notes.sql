-- Create notes table with full-text search indexes
CREATE TABLE IF NOT EXISTS notes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL DEFAULT 'Untitled',
    content TEXT NOT NULL DEFAULT '',
    last_edited_by UUID REFERENCES users(id),
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_notes_user_id ON notes(user_id);
CREATE INDEX idx_notes_user_created ON notes(user_id, created_at DESC);
CREATE INDEX idx_notes_updated_at ON notes(updated_at DESC);

-- Full-text search indexes
CREATE INDEX idx_notes_content_search ON notes USING GIN (to_tsvector('english', content));
CREATE INDEX idx_notes_title_search ON notes USING GIN (to_tsvector('english', title));

-- Auto-update trigger
CREATE TRIGGER update_notes_updated_at 
    BEFORE UPDATE ON notes
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();