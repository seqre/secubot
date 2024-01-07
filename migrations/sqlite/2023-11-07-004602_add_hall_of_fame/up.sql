-- Your SQL goes here

CREATE TABLE IF NOT EXISTS "hall_of_fame_tables"
(
    "id"            INTEGER PRIMARY KEY NOT NULL,
    "guild_id"      BIGINT              NOT NULL,
    "title"         TEXT                NOT NULL,
    "description"   TEXT,
    "creation_date" TEXT                NOT NULL,

    UNIQUE ("guild_id", "title")
);

CREATE TABLE IF NOT EXISTS "hall_of_fame_entries"
(
    "id"            INTEGER PRIMARY KEY NOT NULL,
    "hof_id"        INTEGER             NOT NULL,
    "user_id"       BIGINT              NOT NULL,
    "description"   TEXT,
    "creation_date" TEXT                NOT NULL,

    FOREIGN KEY ("hof_id") REFERENCES "hall_of_fame_tables" ("id") ON DELETE CASCADE
);

