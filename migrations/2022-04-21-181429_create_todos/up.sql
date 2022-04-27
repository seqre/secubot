-- Create TODO table

CREATE TABLE IF NOT EXISTS "todos" (
  "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	"channel_id" BIGINT NOT NULL,
	"todo"	TEXT NOT NULL,
	"creation_date"	TEXT NOT NULL,
	"completion_date"	TEXT
);
