# Life of an Oak Module

<!--* freshness: { owner: 'mks' reviewed: '2019-07-09' }
tag: 'codelab' *-->

<!-- TODO: Update the go link under doc title. -->

g.co/sample-codelab

**WARNING:** This codelab has not yet been reviewed.
Read at your own risk.

Project Oak is an open source infrastructure for the verifiably secure storage,
processing and exchange of data.

A trusted program is compiled to an Oak Module which runs on an Oak Node inside
a secure hardware enclave.

This codelab shows you how to create and run an Oak Module.

gRPC, Rust

## Getting started

Follow the instructions for [installing Oak](
https://github.com/michael-kernel-sanders/oak/blob/mks_storage_module_api/INSTALL.md)
with the following modification for checking out the git repository:

```shell
$ git clone https://github.com/michael-kernel-sanders/oak.git
$ cd oak
$ git checkout mks_storage_module_api
```

## Writing the module

An Oak Module is a WebAssembly binary compiled from a source language using the
Oak Module API, which in this case is Rust.  Instead of a regular program with a
main function we'll be creating a library that uses special Oak Module
functions.

### Hello, is anyone there?

Before we write the module itself, we should define a service for the module to
implement.  Since the module runs in a secure enclave, it wouldn't be very
useful if it couldn't communicate with anyone!  It would also get very lonely.

> An Oak Module should implement a gRPC service to securely communicate outside of
> the enclave.

`proto/hello.proto`:
```protocol-buffer
syntax = "proto3";

package oak.codelab.hello;

message HelloRequest {
  string greeting = 1;
}

message HelloResponse {
  string reply = 1;
}

service HelloWorld {
  rpc SayHello(HelloRequest) returns (HelloResponse);
}
```

> Put the proto in a proto dir, or it gets the hose again.
> Create build.rs to compile it.

`build.rs`:
```rust
extern crate protoc_rust;

fn main() {
    protoc_rust::run(protoc_rust::Args {
        out_dir: "src/proto",
        input: &["proto/hello.proto"],
        includes: &[],
        customize: protoc_rust::Customize::default(),
    })
    .expect("protoc");
}

```

### Is it a module or a node?

The program itself is an Oak Module, but the instance of the module running in
the enclave is an Oak Node.  If that's too confusing, you can just call it a nodule.

Let's start with some basic imports:

`src/lib.rs`:
```rust
#[macro_use]
extern crate oak;
extern crate oak_derive;
extern crate oak_log;
extern crate protobuf;

mod proto;

use oak_derive::OakNode;
use proto::hello::{HelloRequest, HelloResponse};
use protobuf::Message;
use std::io::Write;
```

**WARNING:** oak_log bad, mmm'kay

This part is magic:

```rust
#[derive(OakNode)]
struct Node;
```
You could say it's [cargo-culted](https://en.wikipedia.org/wiki/Cargo_cult_programming).

Some boilerplate we need for now:

```rust
// TODO: Generate this code via a macro or code generation (e.g. a protoc plugin).
trait HelloWorldNode {
    fn say_hello(&self, req: &HelloRequest) -> HelloResponse;
}

// TODO: Generate this code via a macro or code generation (e.g. a protoc plugin).
impl oak::Node for Node {
    fn new() -> Self {
        oak_log::init(log::Level::Debug).unwrap();
        Node
    }
    fn invoke(&mut self, grpc_method_name: &str, grpc_channel: &mut oak::Channel) {
        let mut logging_channel = oak::logging_channel();
        match grpc_method_name {
            "/oak.examples.hello_world.HelloWorld/SayHello" => {
                let req = protobuf::parse_from_reader(grpc_channel).unwrap();
                let res = (self as &mut HelloWorldNode).say_hello(&req);
                res.write_to_writer(grpc_channel).unwrap();
            }
            _ => {
                writeln!(logging_channel, "unknown method name: {}", grpc_method_name).unwrap();
                panic!("unknown method name");
            }
        };
    }
}

```

Finally we have the actual implementation:

```rust
impl HelloWorldNode for Node {
    fn say_hello(&self, req: &HelloRequest) -> HelloResponse {
        let mut res = HelloResponse::new();
        info!("Say hello to {}", req.greeting);
        res.reply = format!("HELLO {}!", req.greeting);
        res
    }
}
```

## More than just a service

Come for the friendly service, stay for the persistent storage!
Another way to communicate outside the enclave is with the Storage Channel.

### Module API

```rust
pub fn storage_read(storage_name: &Vec<u8>, name: &Vec<u8>) -> Vec<u8>  {
}

pub fn storage_write(storage_name: &Vec<u8>, name: &Vec<u8>, value: &Vec<u8>) {
}

pub fn storage_delete(storage_name: &Vec<u8>, name: &Vec<u8>) {
}
```

### StorageProvider

```c++
// StorageProvider is an abstract interface for implementations of a
// persistent data store for StorageService.  This is an opaque name-value
// store that should handle only encrypted blobs of data.
class StorageProvider {
 public:
  StorageProvider() {}

  virtual grpc::Status Read(const ReadRequest* request, ReadResponse* response) = 0;
  virtual grpc::Status Write(const WriteRequest* request, WriteResponse* response) = 0;
  virtual grpc::Status Delete(const DeleteRequest* request, DeleteResponse* response) = 0;
  virtual grpc::Status Begin(const BeginRequest* request, BeginResponse* response) = 0;
  virtual grpc::Status Commit(const CommitRequest* request, CommitResponse* response) = 0;
  virtual grpc::Status Rollback(const RollbackRequest* request, RollbackResponse* response) = 0;
};
```
