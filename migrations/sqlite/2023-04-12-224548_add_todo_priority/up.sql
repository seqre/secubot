-- Your SQL goes here

ALTER TABLE "todos" ADD COLUMN "priority" INT DEFAULT 0 NOT NULL;

UPDATE "todos" SET "priority" = 0;
