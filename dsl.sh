#!/usr/bin/env bash

diesel --database-url "db.sqlite" migration --migration-dir migrations/sqlite "$@"