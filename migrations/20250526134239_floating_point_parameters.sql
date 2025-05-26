-- Add new column with DOUBLE PRECISION (f64 equivalent)
ALTER TABLE parlay_parameters ADD COLUMN threshold_new DOUBLE PRECISION;

-- Update new column with converted values
UPDATE parlay_parameters SET threshold_new = threshold::DOUBLE PRECISION;

-- Drop old column and rename new one
ALTER TABLE parlay_parameters DROP COLUMN threshold;
ALTER TABLE parlay_parameters RENAME COLUMN threshold_new TO threshold;

-- Add new column with DOUBLE PRECISION (f64 equivalent)
ALTER TABLE parlay_parameters ADD COLUMN range_new DOUBLE PRECISION;

-- Update new column with converted values
UPDATE parlay_parameters SET range_new = range::DOUBLE PRECISION;

-- Drop old column and rename new one
ALTER TABLE parlay_parameters DROP COLUMN range;
ALTER TABLE parlay_parameters RENAME COLUMN range_new TO range;