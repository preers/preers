# Preers
Preers 是一个基于 P2P 网络实现的代理软件，该网络组成的设备之间将能直接访问彼此
提供的服务。内网环境下，节点之间能够利用网络中具有公网IP的中转节点转发流量，同
时尝试“打洞”直连来提升连接性并减轻中转节点负担。

# 组成
`preers`: 构成网络的节点程序

preers 默认在 9843 提供 RESTful API 用于配置管理，我们提供了一下两种配置工具：
* `preers-ctl`: 命令行工具
* `preers-ui`: Web UI

# 构建方式
* preers: `cargo build --release --bin preers`
* preers-ctl: `cargo build --release --bin preers-ctl`
* preers-ui: `cd preers-ui; npm run build`，构建结果在 dist 目录下，需要启动一个Web服务器来提供此目录

# 使用方法
需要至少一个拥有公网 IP 的节点为网络中的刚刚启动的节点提供网络信息，这样的节点
叫做 Rendezvous （法语里面“相会”的意思）。Preers网络中的节点会定期向 rendezvous
节点注册自己的地址、告知自己是否为中转节点，以及获取其他节点的信息。
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

还需要一个中转节点（relay）来为没有公网地址的节点接受入向流量。通过
`preers-ctl`向中转节点添加 rendezvous 节点的地址，这样所有与 rendezvous 有连接的
节点都会知道 relay 的存在。
```
$ preers --relay
$ preers-ctl add rendezvous /ip4/172.22.40.233/tcp/46845
```

可以通过`preers-ctl info`查看网络情况，包括自己的Peer Id，已认识的节
点以及连接情况：
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

在其他电脑上，运行`preers`，
```
$ preers
```
然后添加刚刚的 rendezvous 节点的地址。可以使用 `preers-ctl`，也可以
使用我们提供的 Web UI。

## 添加服务
向 Preers 网络暴露的服务，比如这里本地的 10.0.0.4:3389 远程桌面 RDP 服务
```
$ preers-ctl add provide --host 10.0.0.4 --port 3389
```

## 使用服务
在需要使用服务的电脑上，输入提供服务的节点的 Peer Id 和其服务地址(这
里是 10.0.0.4:3389)，然后提供一个本地用于提供该服务的端口
(forwarder_port)，如这里的 12345。
```
$ preers-ctl add use --peer-id --host 10.0.0.4 --port 3389 --forwarder-port 12345
```
