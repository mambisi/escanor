![logo](https://raw.githubusercontent.com/mambisi/escanor/master/static/logo.png)

## Welcome to the Escanor

Escanor is a high performance in memory database written in [rust](http://rust-lang.org/) it offers performance similar to redis and implement the [redis protocol](https://redis.io/topics/protocol) with useful additions for json data manipulations. This is a side project with the vision of making it into a major project any contributors are welcome.

## Features

- High Performance

- Non Blocking Key-Value

- Asynchronous Server 

- Great support for Json Storage

- Support for Redis Clients and Libraries

- Client Cli included.

## Installation
- You can [Download](https://github.com/mambisi/escanor/releases) the prebuilt binary for (Windows,Linux and Mac) on the release page.
- Build from the source.

### Installing on Ubuntu
[Download](https://github.com/mambisi/escanor/releases) the Debian package.
```sh
$ dpkg -i path/escanor_0.1.0_amd64.deb
```
Run the server
```sh
$ escaper-server
```
Run the server in background and enable able after system reboot.
```sh
$ systemctl start escanor-server
```
```sh
$ systemctl enable escanor-server
```

Run the Cli
```sh
$ escanor-cli
```
### Installing on Windows
Download the ``escanor_0.1.0_win64.zip`` there are two files ``escanor-server.exe`` and ``escanor-cli.exe`` open ``escanor-server.exe`` then ``escanor-cli.exe``.

### Installing on Mac
Download the ``escanor_0.1.0_osx.zip`` there are two files ``escanor-server`` and ``escanor-cli`` open ``escanor-server`` then ``escanor-cli``.

## Run from Source

```git
git clone https://github.com/mambisi/escanor.git
```
```git
cd escanor
```
```git
cargo run --bin escanor-server
```
```git
cargo run --bin escanor-cli
```

