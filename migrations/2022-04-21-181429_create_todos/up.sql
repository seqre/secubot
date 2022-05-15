-- Create TODO table

CREATE TABLE IF NOT EXISTS "todos" (
	"channel_id" BIGINT NOT NULL,
  "id" INTEGER NOT NULL,
	"todo"	TEXT NOT NULL,
	"creation_date"	TEXT NOT NULL,
	"completion_date"	TEXT,

	PRIMARY KEY("channel_id", "id")
);
