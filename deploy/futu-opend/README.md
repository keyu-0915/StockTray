# Futu OpenD container

This image downloads the official Futu Ubuntu command-line package during the
build and verifies its pinned SHA-256 checksum. It does not use or inherit from
a third-party OpenD image.

## Security defaults

- OpenD listens on container port `32179`.
- The host port is bound to `127.0.0.1` only. Use an SSH tunnel for remote tests.
- The Telnet and WebSocket listeners are disabled.
- Credentials are mounted as Docker Compose secrets and are not built into the image.
- The OpenD device identity and logs are stored in named volumes.
- Automatic quote-right takeover is disabled by default.

## Server setup

The server must be Linux x86-64 with Docker Engine and Docker Compose v2.

To allow direct access from a trusted LAN, create `.env` with a specific LAN
address rather than `0.0.0.0`:

```sh
FUTU_BIND_IP=192.168.10.33
FUTU_HOST_PORT=32179
```

Create the secret files interactively on the server; do not commit or send them in chat:

```sh
./configure-secrets.sh
```

The password is entered with terminal echo disabled and only its MD5 value is
written to disk, as required by OpenD.

Build and start:

```sh
docker compose build --pull
docker compose up -d
docker compose logs --tail=100 futu-opend
```

If OpenD asks for device verification, attach to its interactive console:

```sh
docker attach stocktray-futu-opend
```

Detach without stopping it by pressing `Ctrl-P`, then `Ctrl-Q`.

## Test through an SSH tunnel

From the client computer:

```sh
ssh -N -L 32179:127.0.0.1:32179 USER@SERVER
```

The OpenAPI client can then connect to `127.0.0.1:32179`. The port does not
need to be opened in the server's public firewall.
