-- V002: richer assessments (summary, recommendation, steps, suggested commands) stored as JSON.
ALTER TABLE risk_assessments ADD COLUMN details TEXT NOT NULL DEFAULT '{}';
