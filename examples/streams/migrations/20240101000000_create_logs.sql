-- SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
-- SPDX-License-Identifier: MIT

-- Create logs table for streams example

CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY,
    action VARCHAR(50) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id UUID NOT NULL,
    user_id UUID,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient streaming queries
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);

-- Sample data (large dataset for streaming)
INSERT INTO audit_logs (id, action, resource_type, resource_id, user_id, details) VALUES
    ('10000000-0000-0000-0000-000000000001', 'create', 'user', 'u0000000-0000-0000-0000-000000000001', 'u0000000-0000-0000-0000-000000000001', '{"email": "alice@example.com"}'),
    ('10000000-0000-0000-0000-000000000002', 'update', 'user', 'u0000000-0000-0000-0000-000000000001', 'u0000000-0000-0000-0000-000000000001', '{"field": "name"}'),
    ('10000000-0000-0000-0000-000000000003', 'create', 'order', 'o0000000-0000-0000-0000-000000000001', 'u0000000-0000-0000-0000-000000000001', '{"total": 99.99}'),
    ('10000000-0000-0000-0000-000000000004', 'update', 'order', 'o0000000-0000-0000-0000-000000000001', 'u0000000-0000-0000-0000-000000000002', '{"status": "shipped"}'),
    ('10000000-0000-0000-0000-000000000005', 'delete', 'session', 's0000000-0000-0000-0000-000000000001', 'u0000000-0000-0000-0000-000000000001', null);
