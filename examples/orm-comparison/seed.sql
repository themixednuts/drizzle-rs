INSERT INTO users (name, email, age) VALUES
    ('Alex Smith', 'alex@example.com', 26),
    ('Jordan Lee', 'jordan@example.com', 30),
    ('Alice', 'alice@example.com', 28),
    ('Bob', 'bob@example.com', 32);

INSERT INTO posts (title, content, author_id) VALUES
    ('Hello', 'first post', 1),
    ('World', 'second post', 1);

INSERT INTO comments (body, post_id) VALUES
    ('nice post', 1);
