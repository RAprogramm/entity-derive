-- SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
-- SPDX-License-Identifier: MIT

-- Create documents table for soft delete example

CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    author VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

-- Index for soft delete queries
CREATE INDEX idx_documents_deleted_at ON documents(deleted_at);

-- Sample documents
INSERT INTO documents (id, title, content, author) VALUES
    ('d0000000-0000-0000-0000-000000000001', 'Getting Started', 'Welcome to our documentation...', 'Alice'),
    ('d0000000-0000-0000-0000-000000000002', 'API Reference', 'Full API documentation...', 'Bob'),
    ('d0000000-0000-0000-0000-000000000003', 'Best Practices', 'Development guidelines...', 'Charlie');
