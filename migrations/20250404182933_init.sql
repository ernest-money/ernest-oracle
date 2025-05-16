-- Add migration script here

CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    announcement_signature BYTEA NOT NULL,
    oracle_event BYTEA NOT NULL,
    name TEXT NOT NULL,
    is_enum BOOLEAN NOT NULL,
    announcement_event_id TEXT,
    attestation_event_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create the event_nonces table
CREATE TABLE IF NOT EXISTS event_nonces (
    id INTEGER PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    index INTEGER NOT NULL,
    nonce BYTEA NOT NULL,
    outcome TEXT,
    signature BYTEA,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(event_id, index)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_event_nonces_event_id ON event_nonces(event_id);
CREATE INDEX IF NOT EXISTS idx_events_name ON events(name);