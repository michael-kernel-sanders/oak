/*
 * Copyright 2019 The Project Oak Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include "absl/flags/flag.h"
#include "absl/flags/parse.h"
#include "absl/memory/memory.h"
#include "asylo/util/logging.h"
#include "examples/hello_world/proto/hello_world.grpc.pb.h"
#include "examples/hello_world/proto/hello_world.pb.h"
#include "examples/utils/utils.h"
#include "include/grpcpp/grpcpp.h"
#include "oak/client/manager_client.h"
#include "oak/client/node_client.h"

ABSL_FLAG(std::string, manager_address, "127.0.0.1:8888",
          "Address of the Oak Manager to connect to");
ABSL_FLAG(std::string, module, "", "File containing the compiled WebAssembly module");

using ::oak::examples::hello_world::HelloRequest;
using ::oak::examples::hello_world::HelloResponse;
using ::oak::examples::hello_world::HelloWorld;

void say_hello(HelloWorld::Stub* stub, std::string name) {
  grpc::ClientContext context;
  HelloRequest request;
  request.set_greeting(name);
  LOG(INFO) << "Request: " << request.greeting();
  HelloResponse response;
  grpc::Status status = stub->SayHello(&context, request, &response);
  if (!status.ok()) {
    LOG(QFATAL) << "Could not call SayHello: " << status.error_code() << ": "
                << status.error_message();
  }
  LOG(INFO) << "Response: " << response.reply();
}

void lots_of_replies(HelloWorld::Stub* stub, std::string name) {
  grpc::ClientContext context;
  HelloRequest request;
  request.set_greeting(name);
  LOG(INFO) << "Request: " << request.greeting();
  auto reader = stub->LotsOfReplies(&context, request);
  if (reader == nullptr) {
    LOG(QFATAL) << "Could not call LotsOfReplies";
  }
  HelloResponse response;
  while (reader->Read(&response)) {
    LOG(INFO) << "Response: " << response.reply();
  }
}

int main(int argc, char** argv) {
  absl::ParseCommandLine(argc, argv);

  // Connect to the Oak Manager.
  std::unique_ptr<oak::ManagerClient> manager_client =
      absl::make_unique<oak::ManagerClient>(grpc::CreateChannel(
          absl::GetFlag(FLAGS_manager_address), grpc::InsecureChannelCredentials()));

  // Load the Oak Module to execute. This needs to be compiled from Rust to WebAssembly separately.
  std::string module_bytes = oak::utils::read_file(absl::GetFlag(FLAGS_module));
  oak::CreateNodeResponse create_node_response = manager_client->CreateNode(module_bytes);

  std::stringstream addr;
  addr << "127.0.0.1:" << create_node_response.port();
  LOG(INFO) << "Connecting to Oak Node: " << addr.str();

  oak::NodeClient::InitializeAssertionAuthorities();

  // Connect to the newly created Oak Node.
  auto stub = HelloWorld::NewStub(grpc::CreateChannel(
      addr.str(), asylo::EnclaveChannelCredentials(asylo::BidirectionalNullCredentialsOptions())));

  // Perform multiple invocations of the same Oak Node, with different parameters.
  say_hello(stub.get(), "WORLD");
  say_hello(stub.get(), "MONDO");
  say_hello(stub.get(), "世界");

  // TODO(#97): Uncomment this line when gRPC server streaming is implemented.
  // lots_of_replies(stub.get(), "WORLDS");

  return 0;
}
