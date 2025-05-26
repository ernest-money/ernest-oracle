CREATE TABLE numeric_attestation_outcome (
    id SERIAL PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE,
    combined_score DOUBLE PRECISION NOT NULL,
    attested_value INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_numeric_attestation_outcome_event_id ON numeric_attestation_outcome(event_id);

CREATE TABLE numeric_attestation_data_outcome (
    id SERIAL PRIMARY KEY,
    event_id TEXT NOT NULL,
    data_type TEXT NOT NULL,
    normalized_value DOUBLE PRECISION NOT NULL,
    original_value DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_numeric_attestation_data_outcome_event_id ON numeric_attestation_data_outcome(event_id);

ALTER TABLE numeric_attestation_data_outcome 
ADD CONSTRAINT fk_event_id 
FOREIGN KEY (event_id) REFERENCES numeric_attestation_outcome(event_id);