-- Create tables for relations example

CREATE TABLE IF NOT EXISTS authors (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS posts (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS comments (
    id UUID PRIMARY KEY,
    text TEXT NOT NULL,
    commenter_name VARCHAR(255) NOT NULL,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_posts_author_id ON posts(author_id);
CREATE INDEX idx_comments_post_id ON comments(post_id);

-- Sample data
INSERT INTO authors (id, name, email) VALUES
    ('a1000000-0000-0000-0000-000000000001', 'John Doe', 'john@example.com'),
    ('a1000000-0000-0000-0000-000000000002', 'Jane Smith', 'jane@example.com');

INSERT INTO posts (id, title, content, author_id) VALUES
    ('b1000000-0000-0000-0000-000000000001', 'Hello World', 'My first post content', 'a1000000-0000-0000-0000-000000000001'),
    ('b1000000-0000-0000-0000-000000000002', 'Rust is Great', 'Why I love Rust...', 'a1000000-0000-0000-0000-000000000001'),
    ('b1000000-0000-0000-0000-000000000003', 'Web Development', 'Tips for web dev', 'a1000000-0000-0000-0000-000000000002');

INSERT INTO comments (id, text, commenter_name, post_id) VALUES
    ('c1000000-0000-0000-0000-000000000001', 'Great post!', 'Reader1', 'b1000000-0000-0000-0000-000000000001'),
    ('c1000000-0000-0000-0000-000000000002', 'Thanks for sharing', 'Reader2', 'b1000000-0000-0000-0000-000000000001'),
    ('c1000000-0000-0000-0000-000000000003', 'I agree!', 'Reader3', 'b1000000-0000-0000-0000-000000000002');
