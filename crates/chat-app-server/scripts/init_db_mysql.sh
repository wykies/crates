#!/usr/bin/env bash
# set -x # Only turn on if debugging otherwise it just seems annoying
set -eo pipefail

if ! [ -x "$(command -v mysql)" ]; then
  echo >&2 "Error: mysql client is not installed."
  echo >&2 "Use:"
  echo >&2 "     sudo apt install mysql-client"
  echo >&2 "to install it."
  exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version='~0.8' sqlx-cli --no-default-features --features rustls,mysql"
  echo >&2 "to install it."
  exit 1
fi

# Check if a custom parameter has been set, otherwise use default values
DB_USER="${MYSQL_USER:=db_user}"
DB_PASSWORD="${MYSQL_PASSWORD:=password}"
DB_NAME="${MYSQL_DB:=chat_demo}"
DB_PORT="${MYSQL_PORT:=3306}"
DB_HOST="${MYSQL_HOST:=localhost}"

# Allow to skip Docker if a MySql database is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
  # if a mysql container is running, print instructions to kill it and exit
  RUNNING_MYSQL_CONTAINER=$(docker ps --filter 'name=mysql' --format '{{.ID}}')
  if [[ -n $RUNNING_MYSQL_CONTAINER ]]; then
    echo >&2 "there is a mysql container already running, kill it with"
    echo >&2 "    docker kill ${RUNNING_MYSQL_CONTAINER}"
    exit 1
  fi
  # Launch mysql using Docker (Root needed for testing to create databases, database only used for testing)
  docker run \
      -e MYSQL_USER=${DB_USER} \
      -e MYSQL_PASSWORD=${DB_PASSWORD} \
      -e MYSQL_DATABASE=${DB_NAME} \
      -e MYSQL_ROOT_PASSWORD=${DB_PASSWORD} \
      -e MYSQL_ROOT_HOST=% \
      -p "${DB_PORT}":3306 \
      -d \
      --name "mysql_$(date '+%s')" \
      mysql
      # TODO 4: Increase number of connections for testing
fi

#Keep pinging MySql until it's ready to accept commands
max_retries=120
until MYSQL_PWD="${DB_PASSWORD}" mysql --protocol=TCP -h "${DB_HOST}" -u "root" -P "${DB_PORT}" -D "mysql" -e '\q'; do
  >&2 echo "MySql is still unavailable - sleeping ($max_retries attempts left)"
  if [ $max_retries -lt 1 ];  then
    >&2 echo "Exceeded attempts to connect to DB"
    exit 1
  fi
  sleep 1
  ((max_retries--))
done

>&2 echo "MySql is up and running on port ${DB_PORT} - running migrations now!"

export DATABASE_URL=mysql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run --source migrations_mysql

>&2 echo "MySql has been migrated, ready to go!"
