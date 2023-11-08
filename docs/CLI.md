# Mycelial Command Line Interface (CLI)

The CLI bootstraps the Mycelial server and client setup process.

## Installation


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
  curl -L https://github.com/mycelial/cli/releases/download/v0.1.5/mycelial_0.1.5_amd64.deb --output mycelial_amd64.deb
  dpkg -i ./mycelial_amd64.deb
  ```

  </details>

  <details>
  <summary>Debian Based Linux ARM64</summary>

  ```sh
  curl -L https://github.com/mycelial/cli/releases/download/v0.1.5/mycelial_0.1.5_arm64.deb --output mycelial_arm64.deb
  dpkg -i ./mycelial_arm64.deb
  ```

  </details>

  <details>
  <summary>Debian Based Linux ARM</summary>

  ```sh
  curl -L https://github.com/mycelial/cli/releases/download/v0.1.5/mycelial_0.1.5_armhf.deb --output mycelial_armhf.deb
  dpkg -i ./mycelial_armhf.deb
  ```

  </details>

  <details>
  <summary>Redhat Based Linux x86_64</summary>

  ```sh
  yum install https://github.com/mycelial/cli/releases/download/v0.1.5/mycelial-v0.1.5-1.x86_64.rpm 
  ```

  </details>

  <details>
  <summary>Redhat Based Linux ARM64</summary>

  ```sh
  yum install https://github.com/mycelial/cli/releases/download/v0.1.5/mycelial-v0.1.5-1.arm64.rpm 
  ```

  </details>

  <details>
  <summary>Redhat Based Linux ARM</summary>

  ```sh
  yum install https://github.com/mycelial/cli/releases/download/v0.1.5/mycelial-v0.1.5-1.armhf.rpm
  ```

  </details>

</details>

## Initialization

Run the following command to download and configure Mycelial.

```sh
mycelial init --local
```

After downloading the server and client, you will be prompted with a series of
configuration options. For demo purposes, you should accept the default values
when available.

Upon exiting, the `init` step will create a `config.toml` file, which will be
used by the Mycelial client (myceliald).

## Starting

Run the following command to start the server and client.

```sh
mycelial start
```

When prompted for a token, use the one you used in the `init` step.

After completing this step, you should be able to open the web interface 
`http://localhost:7777`.

## Shutdown

Run the following command to terminate the server and client.

```sh
mycelial destroy
```