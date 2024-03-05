# Server Documentation

## Introduction

This is the server for our application. It is responsible for handling all the backend operations such as database interactions, API calls, etc.

## Getting Started

1. Install PostgreSQL, e.g. on macOS:
    ```
    brew install postgresql
    ```
1. After installation, you can start PostgreSQL using:
    ```
    brew services start postgresql
    ```
1. Launch the PostgreSQL command line interface:
    ```
    psql postgres
    ```
1. To use the default database connection, create the following new database and user.
    ```
    CREATE DATABASE mycelial_server_dev;
    CREATE USER mycelial;
    ```
1. Grant all privileges on the database to your user:
    ```
    GRANT ALL PRIVILEGES ON DATABASE mycelial_server_dev TO mycelial;
    ```
