-- Add new column with BIGINT
ALTER TABLE parlay_parameters ADD COLUMN threshold_new BIGINT;

-- Update new column with converted values (rounded to nearest integer)
UPDATE parlay_parameters SET threshold_new = ROUND(threshold)::BIGINT;

-- Drop old column and rename new one
ALTER TABLE parlay_parameters DROP COLUMN threshold;
ALTER TABLE parlay_parameters RENAME COLUMN threshold_new TO threshold;

-- Add new column with BIGINT
ALTER TABLE parlay_parameters ADD COLUMN range_new BIGINT;

-- Update new column with converted values (rounded to nearest integer)
UPDATE parlay_parameters SET range_new = ROUND(range)::BIGINT;

-- Drop old column and rename new one
ALTER TABLE parlay_parameters DROP COLUMN range;
ALTER TABLE parlay_parameters RENAME COLUMN range_new TO range;