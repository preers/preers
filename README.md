# Preers
[English](README.md) | [中文](README-zh.md)

**Preers** is a P2P (peer-to-peer) network-based proxy software designed to
allow devices within a P2P network to directly access services provided by each
other. Nodes can utilize relay nodes with public IP addresses in the network to
forward traffic, while also attempting to establish direct "hole-punching"
connections to improve connectivity and reduce the load on relay nodes.

## Project Structure

**`preers`**: This is the node program that constitutes the P2P network. Each node can act as a service provider or a service user.

`preers` provides RESTful API for real-time configuration at port 9843 by default.

The following tools are provided for configuration:
* `preers-ctl`: Command-line tool
* `preers-ui`: Web UI

## Build

- **preers**: `cargo build --release --bin preers`
- **preers-ctl**: `cargo build --release --bin preers-ctl`
- **preers-ui**: `cd preers-ui; npm run build`, the build result is in the `dist` directory, and a web server needs to be started to serve this directory.

## Usage

At least one node with a public IP address is needed to provide network information for newly connected nodes in the network. Such a node is called a **Rendezvous** (meaning "meeting" in French). Nodes in the Preers network will periodically register their addresses with the rendezvous node, inform whether they are relay nodes, and obtain information about other nodes.

```
$ preers --rendezvous
Opened database: /home/lucilius/src/preers/preers.db
Peer ID: 12D3KooWKatvhtLTMgwrMB1ocoifibvTgxg8wvNzrm9Dxa28bL39
Network initialized...
Database running...
HTTP listening on 9843
Listening on address: /ip4/127.0.0.1/tcp/46845
Listening on address: /ip6/::1/tcp/38945
Listening on address: /ip4/172.22.40.233/tcp/46845
Listening on address: /ip6/fe80::216:3eff:fe00:8352/tcp/38945
Listening on address: /ip4/127.0.0.1/udp/48099/quic-v1
Listening on address: /ip6/::1/udp/49465/quic-v1
Listening on address: /ip4/172.22.40.233/udp/48099/quic-v1
Listening on address: /ip6/fe80::216:3eff:fe00:8352/udp/49465/quic-v1
```

A relay node is also needed to accept incoming traffic for nodes without a public IP address. Use `preers-ctl` to add the address of the rendezvous node to the relay node, so that all nodes connected to the rendezvous will be aware of the relay's existence.

```
$ preers --relay
$ preers-ctl add rendezvous /ip4/172.22.40.233/tcp/46845
```

You can use `preers-ctl info` to view the network status, including your own Peer ID, known nodes, and connection status:

```
$ preers-ctl info
NetworkInfo {
    peer_id: PeerId(
        "12D3KooWNe6zMuh3Z5njFxfSFnoYHNErLgTNieBQ7MtVQVcEfYzA",
    ),
    peers: [
        PeerInfo {
            peer_id: PeerId(
                "12D3KooWKatvhtLTMgwrMB1ocoifibvTgxg8wvNzrm9Dxa28bL39",
            ),
            connected: true,
        },
    ],
}
```

On other computers, run `preers`:

```
$ preers
```

Then add the address of the previously mentioned rendezvous node. You can use `preers-ctl` or the Web UI we provide.

### Adding Services

Expose services to the Preers network, such as the local Remote Desktop Protocol (RDP) service at 10.0.0.4:3389:

```
$ preers-ctl add provide --host 10.0.0.4 --port 3389
```

### Using Services

On the computer where the service is needed, enter the Peer ID of the node providing the service and its service address (here, 10.0.0.4:3389), and then provide a local port (forwarder_port) to forward the service, such as 12345 here:

```
$ preers-ctl add use --peer-id --host 10.0.0.4 --port 3389 --forwarder-port 12345
```
