-- Table for ParlayContract
CREATE TABLE parlay_contracts (
    id TEXT PRIMARY KEY,
    combination_method TEXT NOT NULL,  -- Stored as string representation of enum
    max_normalized_value BIGINT NOT NULL
);

-- Join table for ParlayParameter
CREATE TABLE parlay_parameters (
    contract_id TEXT NOT NULL REFERENCES parlay_contracts(id),
    parameter_id SERIAL,  -- Auto-incrementing ID for each parameter
    data_type TEXT NOT NULL,  -- Assuming EventType is an enum/string
    threshold BIGINT NOT NULL,
    range BIGINT NOT NULL,
    is_above_threshold BOOLEAN NOT NULL,
    transformation TEXT NOT NULL,  -- String representation of TransformationFunction
    weight DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (contract_id, parameter_id)
);