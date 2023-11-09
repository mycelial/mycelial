# Tutorial

In this tutorial, we'll set up Mycelial to synchronize data from one SQLite
instance to another using [Mycelite](Mycelite.md).

## Setup

### Installation

Install the Mycelial CLI for your system.

<details>
  <summary>Mac</summary>

  ```sh
  brew install mycelial/tap/mycelial
  ```

</details>

<details>
  <summary>Linux</summary>

  <details>
  <summary>Debian Based Linux x86_64</summary>

  ```sh
  curl -L https://github.com/mycelial/cli/releases/download/v0.2.0/mycelial_0.2.0_amd64.deb --output mycelial_amd64.deb
  dpkg -i ./mycelial_amd64.deb
  ```

  </details>

  <details>
  <summary>Debian Based Linux ARM64</summary>

  ```sh
  curl -L https://github.com/mycelial/cli/releases/download/v0.2.0/mycelial_0.2.0_arm64.deb --output mycelial_arm64.deb
  dpkg -i ./mycelial_arm64.deb
  ```

  </details>

  <details>
  <summary>Debian Based Linux ARM</summary>

  ```sh
  curl -L https://github.com/mycelial/cli/releases/download/v0.2.0/mycelial_0.2.0_armhf.deb --output mycelial_armhf.deb
  dpkg -i ./mycelial_armhf.deb
  ```

  </details>

  <details>
  <summary>Redhat Based Linux x86_64</summary>

  ```sh
  yum install https://github.com/mycelial/cli/releases/download/v0.2.0/mycelial-v0.2.0-1.x86_64.rpm 
  ```

  </details>

  <details>
  <summary>Redhat Based Linux ARM64</summary>

  ```sh
  yum install https://github.com/mycelial/cli/releases/download/v0.2.0/mycelial-v0.2.0-1.arm64.rpm 
  ```

  </details>

  <details>
  <summary>Redhat Based Linux ARM</summary>

  ```sh
  yum install https://github.com/mycelial/cli/releases/download/v0.2.0/mycelial-v0.2.0-1.armhf.rpm
  ```

  </details>

</details>

### Initializing Mycelial

Open a shell, then create a directory named **demo** and navigate into the demo
directory.

```sh
mkdir demo
cd demo
```

Issue the following command:

```sh
mycelial init --local
```

The `init --local` command will download the appropriate Mycelial server and
client (myceliald) for your system, and it will guide you through the client
configuration process.

Each configuration question you receive in the CLI can be overridden, or you can 
accept the default value. However, the security token must be entered manually.
Make a note of the security token you enter because you'll need it in the
subsequent steps below. For demo purposes, you should accept the defaults
when provided by pressing enter.

When you are prompted `What would you like to do?`, first choose `Add source`,
then select the `Full SQLite replication source` and answer the prompts as
desired, or press enter to accept the default answers. For demo purposes, you 
should accept the default answers.

After you've added a source, choose `Add Destination`, then select `Full SQLite
replication destination` and answer the prompts. For demo purposes, you should
accept the default answers.

After you've added a source and a destination, choose the `Exit` option.

Upon exiting the CLI, a `config.toml` client configuration file is created which
will be used by the client in a moment.

### Starting the Mycelial server and client

Enter the following command to start the server and client:

```sh
mycelial start
```

When prompted for the security token, enter the one you used earlier.

At this point, the Mycelial server and client are running in the background.

### Download Mycelite

Download and unarchive Mycelite for your system:

<details>
  <summary>Mac</summary>
  <details>
    <summary>Mac Arm64</summary>

  ```sh
  curl -L https://github.com/mycelial/mycelite/releases/latest/download/aarch64-apple-darwin.tgz --output aarch64-apple-darwin.tgz
  tar -xvzf aarch64-apple-darwin.tgz
  ```

  </details>
  <details>
    <summary>Mac x86_64</summary>

  ```sh
  curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-apple-darwin.tgz --output x86_64-apple-darwin.tgz
  tar -xvzf x86_64-apple-darwin.tgz
  ```
  </details>
</details>

<details>
  <summary>Linux</summary>

<details>
  <summary>Linux x86_gnu</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-unknown-linux-gnu.tgz --output x86_64-unknown-linux-gnu.tgz
tar -xvzf x86_64-unknown-linux-gnu.tgz
```
</details>
<details>
  <summary>Linux x86_musl</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-unknown-linux-musl.tgz --output x86_64-unknown-linux-musl.tgz
tar -xvzf x86_64-unknown-linux-musl.tgz
```
</details>

<details>
  <summary>Linux arm_32</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/arm-unknown-linux-gnueabihf.tgz --output arm-unknown-linux-gnueabihf.tgz
tar -xvzf arm-unknown-linux-gnueabihf.tgz
```
</details>
<details>
  <summary>Linux arm_64</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/aarch64-unknown-linux-gnu.tgz --output arm-unknown-linux-gnueabihf.tgz
tar -xvzf arm-unknown-linux-gnueabihf.tgz
```
</details>

</details>

### Use Mycelite

After you've downloaded and unarchived the extension, you'll need to load the
extension and open your SQLite database. When the extension is loaded and the
SQLite database is opened, it will create a Mycelite journal file, a sibling
file to the SQLite database file. The journal file is used to synchronize SQLite
databases, and it was configured earlier during the `init` step.

_MacOS users_: The default SQLite that ships with MacOS does not have extensions
enabled. One alternative is to [install
SQLite](https://formulae.brew.sh/formula/sqlite) with
[Homebrew](https://brew.sh/). Be sure to adjust your PATH environmental variable
to point to the SQLite version you installed with Homebrew.

```sh
sqlite3
.load ./libmycelite mycelite_writer
.open data.db
```

**NOTE**: You must load the extension every time you open the source SQLite
database.

Next, create a table in your SQLite database and insert a record.

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO users(name) VALUES ('John');
```

## Create a data pipeline

Open your web browser and navigate to `http://localhost:7777`. When prompted for
a username and password, enter the security token you previously created into
the username field, leaving the password field blank.

In the upper left portion of the page, you'll see the `source` and `destination`
we configured for the client and the `Mycelial Server`.

Drag and drop the source node onto the canvas.

Next, drag and drop the `Mycelial Server` onto the canvas and set the token to
the value you entered earlier.

Next, drag and drop the destination node onto the canvas.

Lastly, connect the `source` to the `destination` via the `Mycelial Server`.

Now, push the `publish` button to start the synchronization workflow.

## Observe the synchronization

At this point, you can see a `destination.sqlite` SQLite database in the `demo`
directory.

Open a new terminal shell and query the database as follows:

```sh
cd demo
sqlite3 destination.sqlite
sqlite> .tables # shows database tables
users
sqlite> SELECT * FROM USERS; # you should see the user named 'John'
id  name
--  ----
1   John
```

## Stop the server and client

Enter the command `mycelial destroy` to stop the server and client.