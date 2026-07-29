-- Block-based note content model
-- Each note is composed of ordered blocks (paragraphs, tables, charts, etc.)

CREATE TABLE IF NOT EXISTS note_blocks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    block_type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    position INT NOT NULL DEFAULT 0,
    parent_id UUID REFERENCES note_blocks(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_note_blocks_note_id ON note_blocks(note_id, position ASC);

-- Trigger to update updated_at
CREATE TRIGGER update_note_blocks_updated_at
    BEFORE UPDATE ON note_blocks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to migrate existing notes.content into a single paragraph block
CREATE OR REPLACE FUNCTION migrate_note_content_to_blocks()
RETURNS void AS $$
BEGIN
    INSERT INTO note_blocks (note_id, block_type, data, position)
    SELECT 
        n.id,
        'paragraph',
        jsonb_build_object('text', n.content),
        0
    FROM notes n
    WHERE n.content IS NOT NULL AND n.content != ''
    AND NOT EXISTS (
        SELECT 1 FROM note_blocks nb WHERE nb.note_id = n.id
    );
END;
$$ LANGUAGE plpgsql;
