ALTER TABLE branch
ADD short_name VARCHAR(2) NOT NULL DEFAULT '';
-- Postgres does not support specifying column order