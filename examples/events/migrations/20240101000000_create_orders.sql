-- Create orders table for events example

CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    customer_name VARCHAR(255) NOT NULL,
    product VARCHAR(255) NOT NULL,
    quantity INTEGER NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Sample data
INSERT INTO orders (id, customer_name, product, quantity, status) VALUES
    (gen_random_uuid(), 'Alice', 'Laptop', 1, 'pending'),
    (gen_random_uuid(), 'Bob', 'Mouse', 2, 'shipped'),
    (gen_random_uuid(), 'Charlie', 'Keyboard', 1, 'delivered');
