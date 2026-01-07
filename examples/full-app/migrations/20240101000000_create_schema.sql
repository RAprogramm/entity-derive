-- Full Application Schema
-- Demonstrates all entity-derive features in one application

-- ============================================================================
-- Users (soft_delete, events, hooks)
-- ============================================================================

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'customer',
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_deleted_at ON users(deleted_at);

-- ============================================================================
-- Categories (basic CRUD)
-- ============================================================================

CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- Products (relations, filtering, soft_delete)
-- ============================================================================

CREATE TABLE IF NOT EXISTS products (
    id UUID PRIMARY KEY,
    category_id UUID NOT NULL REFERENCES categories(id),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price BIGINT NOT NULL,
    stock INTEGER NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_price ON products(price);
CREATE INDEX idx_products_deleted_at ON products(deleted_at);

-- ============================================================================
-- Orders (transactions, events, relations)
-- ============================================================================

CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    total BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orders_user ON orders(user_id);
CREATE INDEX idx_orders_status ON orders(status);

-- ============================================================================
-- Order Items (relations)
-- ============================================================================

CREATE TABLE IF NOT EXISTS order_items (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL,
    unit_price BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_order_items_order ON order_items(order_id);
CREATE INDEX idx_order_items_product ON order_items(product_id);

-- ============================================================================
-- Audit Logs (streams)
-- ============================================================================

CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY,
    entity_type VARCHAR(100) NOT NULL,
    entity_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL,
    user_id UUID REFERENCES users(id),
    old_data JSONB,
    new_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);

-- ============================================================================
-- Sample Data
-- ============================================================================

-- Categories
INSERT INTO categories (id, name, description) VALUES
    ('c0000000-0000-0000-0000-000000000001', 'Electronics', 'Electronic devices and accessories'),
    ('c0000000-0000-0000-0000-000000000002', 'Books', 'Physical and digital books'),
    ('c0000000-0000-0000-0000-000000000003', 'Clothing', 'Apparel and fashion items');

-- Users
INSERT INTO users (id, email, name, role) VALUES
    ('u0000000-0000-0000-0000-000000000001', 'admin@example.com', 'Admin User', 'admin'),
    ('u0000000-0000-0000-0000-000000000002', 'alice@example.com', 'Alice Johnson', 'customer'),
    ('u0000000-0000-0000-0000-000000000003', 'bob@example.com', 'Bob Smith', 'customer');

-- Products
INSERT INTO products (id, category_id, name, description, price, stock) VALUES
    ('p0000000-0000-0000-0000-000000000001', 'c0000000-0000-0000-0000-000000000001', 'Laptop Pro', '15-inch professional laptop', 149999, 50),
    ('p0000000-0000-0000-0000-000000000002', 'c0000000-0000-0000-0000-000000000001', 'Wireless Mouse', 'Ergonomic wireless mouse', 4999, 200),
    ('p0000000-0000-0000-0000-000000000003', 'c0000000-0000-0000-0000-000000000002', 'Rust Programming', 'Learn Rust programming language', 3999, 100),
    ('p0000000-0000-0000-0000-000000000004', 'c0000000-0000-0000-0000-000000000003', 'Developer T-Shirt', 'Comfortable cotton t-shirt', 2499, 150);
