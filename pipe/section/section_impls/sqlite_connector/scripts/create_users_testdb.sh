#!/usr/bin/env bash

set +x
set +e

help()
{
  echo "Create a new sqlite database using mycelite."
  echo ""
  echo "Syntax: create_users_testdb.sh <db filename>"
}

createdb()
{
  sqlite3 <<EOF
.load ./libmycelite mycelite_writer
.open $1
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO users(name) VALUES ('John');
EOF
}

if [ -z "$1" ]
then
  help
  exit 1
fi

createdb $1
