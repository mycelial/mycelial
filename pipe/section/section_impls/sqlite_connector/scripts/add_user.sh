#!/usr/bin/env bash

set +x
set +e

help(){
  echo "Add a user to a db created with create_users_testdb.sh."
  echo ""
  echo "Syntax: add_user.sh <db filename> <username>"
}

add_user()
{
  sqlite3 <<EOF
.load ./libmycelite mycelite_writer
.open $1
INSERT INTO users(name) VALUES ('$2');
EOF
}

if [ -z "$1" ] || [ -z "$2" ]
then
  help
  exit 1
fi

add_user $1 $2
