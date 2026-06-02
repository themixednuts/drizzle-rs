CREATE TABLE users (
    id    INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name  TEXT    NOT NULL,
    email TEXT,
    age   INTEGER NOT NULL
);

CREATE TABLE posts (
    id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    title      TEXT    NOT NULL,
    content    TEXT,
    author_id  INTEGER NOT NULL REFERENCES users (id)
);

CREATE TABLE comments (
    id      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    body    TEXT    NOT NULL,
    post_id INTEGER NOT NULL REFERENCES posts (id)
);
