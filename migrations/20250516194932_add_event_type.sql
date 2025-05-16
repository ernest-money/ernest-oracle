-- Create the new table with a foreign key reference
CREATE TABLE event_types (
    id SERIAL PRIMARY KEY,
    oracle_event_id TEXT NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    CONSTRAINT fk_event_id FOREIGN KEY (oracle_event_id) REFERENCES events(event_id) ON DELETE CASCADE
);

-- Add an index on the foreign key for better query performance
CREATE INDEX idx_event_types_oracle_event_id ON event_types(oracle_event_id);

-- Populate the new table with 'parlay' as the event_type for all existing events
INSERT INTO event_types (oracle_event_id, event_type)
SELECT event_id, 'parlay' FROM events;