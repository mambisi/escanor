![logo](https://raw.githubusercontent.com/mambisi/escanor/master/static/logo.png)

## Welcome to the Escanor

Escanor is a high performance database with [sled](https://github.com/spacejam/sled) as the persistence layer and implement the [redis protocol](https://redis.io/topics/protocol) with useful additions for json data manipulations. This is a side project with the vision of making it into a major project any contributors are welcome.

## Features

- High Performance

- Non Blocking Key-Value

- Asynchronous Server 

- Support for Json Document Manipulations

- Works with Redis Clients and Libraries

- Client Cli included.

## [Download](https://github.com/mambisi/escanor/releases)
Download the latest release from the [release page](https://github.com/mambisi/escanor/releases), binaries are available for Windows, Mac and Linux.

## [Install](https://github.com/mambisi/escanor/wiki/Installation)
Installation instructions are available in the [wiki page](https://github.com/mambisi/escanor/wiki/Installation)

## [Commands](https://github.com/mambisi/escanor/wiki)
Supported commands:
``randomkey``,``info``,``dbsize``,``bgsave``,``auth``,``lastsave``,``persist``,``expire``,``expireat``,``set``,``get``,``getset``,``del``,``get``,``ttl``,``geoadd``,``geodel``,``georem``,``georadius``,``georadiusbymember``
 ,``geohash``,``geojson``,``jsetr``,``jset``,``jget``,``jpath``,``jmerge``,``jincrby``
 checkout the wiki page on how to use these commands
[WIKI PAGE](https://github.com/mambisi/escanor/wiki)

## Run

```shell script
git clone https://github.com/mambisi/escanor.git
```
```shell script
cd escanor
```
```shell script
cargo run --bin escanor-server
```
```shell script
cargo run --bin escanor-cli
```

## 0.1.5 Features

New cli with pretty formatted json and colored json output

![logo](https://raw.githubusercontent.com/mambisi/escanor/master/static/escanor-cli.png)


#### New Commands
- RANDOMKEY
- INFO
- DBSIZE
- AUTH
- LASTSAVE
- PERSIST
- EXPIRE
- EXPIREAT
- GETSET
- TTL
- JINCRBY
- JSETR

#### Changes

JSETR now set raw json 
```bash
JSETR user.0 `{"name" : "escanor"}`
```
JSET is used for builder style json creation making it easier to create json form the command line

Example :
```json
{
"name" : "escanor",
"todos" : [
    { "item" : "Wash",
       "completed" : false
    },
    { "item" : "Code",
       "completed" : false
    }
   ]
}
```
Can dynamical be built with JSET
```bash
JSET user name "escanor" todos.+.item "Wash" todos.>.completed false todos.+.item "Code" todos.>.completed false
```
JSET can be used to change json even inner array
```bash
JSET user todos.1.completed true
```

JGET 
```bash
JGET user
```
outputs 
```json
{
"name" : "escanor",
"todos" : [
    { "item" : "Wash",
       "completed" : false
    },
    { "item" : "Code",
       "completed" : true
    }
   ]
}
```

JGET can be used to select specific part of a json object
```bash
JGET user todos
```
outputs 
```json
[
  { "item" : "Wash",
       "completed" : false
   },
   { "item" : "Code",
       "completed" : true
   }
]
```