//
// Copyright 2018 The Project Oak Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

extern crate protobuf;

mod proto;

use proto::storage::{
    DeleteRequest, ReadRequest, StorageOperationRequest, StorageOperationResponse, WriteRequest,
};
use protobuf::Message;
use std::cell::RefCell;
use std::io::{Read, Write};

type Handle = u64;

// Keep in sync with /oak/server/oak_node.h.
pub const LOGGING_CHANNEL_HANDLE: Handle = 1;
pub const GRPC_CHANNEL_HANDLE: Handle = 2;
pub const GRPC_METHOD_NAME_CHANNEL_HANDLE: Handle = 3;
pub const STORAGE_CHANNEL_HANDLE: Handle = 4;

// TODO: Implement panic handler.

mod wasm {
    // See https://rustwasm.github.io/book/reference/js-ffi.html
    #[link(wasm_import_module = "oak")]
    extern "C" {
        pub fn channel_read(handle: u64, buf: *mut u8, size: usize) -> usize;
        pub fn channel_write(handle: u64, buf: *const u8, size: usize) -> usize;
    }
}

pub struct Channel {
    handle: Handle,
}

impl Channel {
    pub fn new(handle: Handle) -> Channel {
        Channel { handle }
    }
}

pub fn logging_channel() -> impl Write {
    let logging_channel = Channel::new(LOGGING_CHANNEL_HANDLE);
    // Only flush logging channel on newlines.
    std::io::LineWriter::new(logging_channel)
}

impl Read for Channel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(unsafe { wasm::channel_read(self.handle, buf.as_mut_ptr(), buf.len()) })
    }
}

impl Write for Channel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(unsafe { wasm::channel_write(self.handle, buf.as_ptr(), buf.len()) })
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Trait encapsulating the operations required for an Oak Node.
pub trait Node {
    fn new() -> Self
    where
        Self: Sized;
    fn invoke(&mut self, grpc_method_name: &str, grpc_channel: &mut Channel);
}

thread_local! {
    static NODE: RefCell<Option<Box<dyn Node>>> = RefCell::new(None);
}

/// Sets the Oak Node to execute in the current instance.
///
/// This function may only be called once, and only from an exported `oak_initialize` function:
///
/// ```rust
/// struct Node;
///
/// impl oak::Node for Node {
///     fn new() -> Self { Node }
///     fn invoke(&mut self, grpc_method_name: &str, grpc_channel: &mut oak::Channel) { /* ... */ }
/// }
///
/// #[no_mangle]
/// pub extern "C" fn oak_initialize() {
///     oak::set_node::<Node>();
/// }
/// ```
pub fn set_node<T: Node + 'static>() {
    NODE.with(|node| {
        match *node.borrow_mut() {
            Some(_) => {
                writeln!(logging_channel(), "attempt to set_node() when already set!").unwrap();
                panic!("attempt to set_node when already set");
            }
            None => {
                writeln!(logging_channel(), "setting current node instance").unwrap();
            }
        }
        *node.borrow_mut() = Some(Box::new(T::new()));
    });
}

#[no_mangle]
pub extern "C" fn oak_handle_grpc_call() {
    NODE.with(|node| match *node.borrow_mut() {
        Some(ref mut node) => {
            let mut grpc_method_channel = Channel::new(GRPC_METHOD_NAME_CHANNEL_HANDLE);
            let mut grpc_method_name = String::new();
            grpc_method_channel
                .read_to_string(&mut grpc_method_name)
                .unwrap();
            let mut grpc_channel = Channel::new(GRPC_CHANNEL_HANDLE);
            node.invoke(&grpc_method_name, &mut grpc_channel);
        }
        None => {
            writeln!(logging_channel(), "gRPC call with no loaded Node").unwrap();
            panic!("gRPC call with no loaded Node");
        }
    });
}

pub fn execute_storage_operation(
    operation_request: &StorageOperationRequest,
) -> StorageOperationResponse {
    writeln!(
        logging_channel(),
        "StorageOperationRequest: {}",
        protobuf::text_format::print_to_string(operation_request)
    )
    .unwrap();

    let mut storage_channel = Channel::new(STORAGE_CHANNEL_HANDLE);
    operation_request
        .write_to_writer(&mut storage_channel)
        .unwrap();
    let response: StorageOperationResponse =
        protobuf::parse_from_reader(&mut storage_channel).unwrap();
    writeln!(
        logging_channel(),
        "StorageOperationResponse: {}",
        protobuf::text_format::print_to_string(&response)
    )
    .unwrap();

    return response;
}

pub fn storage_read(storage_name: &Vec<u8>, name: &Vec<u8>) -> Vec<u8> {
    let mut read_request = ReadRequest::new();
    read_request.name = name.clone();

    let mut operation_request = StorageOperationRequest::new();
    operation_request.storage_name = storage_name.clone();
    operation_request.set_read_request(read_request);

    let operation_response = execute_storage_operation(&operation_request);

    return operation_response.get_read_response().get_value().to_vec();
}

pub fn storage_write(storage_name: &Vec<u8>, name: &Vec<u8>, value: &Vec<u8>) {
    let mut write_request = WriteRequest::new();
    write_request.name = name.clone();
    write_request.value = value.clone();

    let mut operation_request = StorageOperationRequest::new();
    operation_request.storage_name = storage_name.clone();
    operation_request.set_write_request(write_request);

    execute_storage_operation(&operation_request);
}

pub fn storage_delete(storage_name: &Vec<u8>, name: &Vec<u8>) {
    let mut delete_request = DeleteRequest::new();
    delete_request.name = name.clone();

    let mut operation_request = StorageOperationRequest::new();
    operation_request.storage_name = storage_name.clone();
    operation_request.set_delete_request(delete_request);

    execute_storage_operation(&operation_request);
}
