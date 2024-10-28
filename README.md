# What is Giza?

Giza is a decentralized protocol enabling AI-powered agents to operate within Web3 ecosystems. It provides infrastructure for creating, deploying, and running autonomous agents that interact with blockchain protocols and execute complex strategies.

Decentralized applications present significant challenges for users and developers. They require constant attention, frequent interactions, and deep technical knowledge, creating barriers to entry, inefficiencies, and security risks. The multitude of protocols and chains demands continuous monitoring to manage positions and optimize returns; a task beyond most users' capabilities. Moreover, the steep learning curve limits broader adoption, while managing cross-chain activities adds further complexity. Traditional smart contract-based automation, constrained by blockchain throughput and gas costs, falls short of addressing these issues comprehensively.

*Enter Giza Agents*

Giza Agents address these challenges by providing verifiable automation of complex processes, enabling 24/7 monitoring and execution, and making Web3 more accessible, efficient, and engaging.

These autonomous software entities execute complex strategies, interact with multiple blockchain protocols, and make intelligent decisions based on real-time data and predefined algorithms. They can be customized for various use cases, from optimizing DeFi strategies to managing cross-chain operations, while maintaining transparency and verifiability.

# Building Giza as an AVS

The Giza Protocol aims to provide decentralized execution, validation, and result verification of complex off-chain applications in a trust-minimized manner. We are building Giza as an AVS to leverage Ethereum's security for:

**Decentralized execution**: Ensuring a distributed execution environment for Agents.
**Decentralized validation and verification**: Ensuring user operations from Agents' executions are validated and verified across the network.
**Trustless operations**: Implementing mechanisms that guarantee tamper resistance, preventing single-entity manipulation.

# AVSthon Scope

For the AVSthon, we have simplified the main components of the Giza protocol to deliver a functional proof of concept combining on-chain and off-chain components. The scope includes:

1. **Giza AVS** 
   - [GizaAvs](./contracts/src/GizaAvs.sol): Implements simplified operator registration to the AVS.
   - [ClientAppRegistry](./contracts/src/ClientAppRegistry.sol): Handles registration of client applications.
   - [TaskRegistry](./contracts/src/TaskRegistry.sol): Manages task registration and operator execution requests.
2. **Demo-App**
   - [DemoApp](./app/src/main.rs): A simple Rust binary that fetches the latest Ethereum block, packaged as a Docker image.
3. **Operator**
   - [Operator](./operator/src/main.rs): A Rust binary that monitors requested tasks, retrieves tasks from Docker images, executes them, and forwards results to the aggregator node.
4. **Aggregator**
   - [Aggregator](./aggregator/src/main.rs): A Rust binary that processes operator results, verifies signatures, performs consensus validation, and broadcasts results to the `TaskRegistry`.

![Overview](./assets/overview.png)

## Operator Registration

Two operators run on this AVS, both registered on Eigenlayer ([Operator 1](https://holesky.eigenlayer.xyz/operator/0x37893031A8484066232AcBE6bFe7E2a7A4411a7d) and [Operator 2](https://holesky.eigenlayer.xyz/operator/0x76cCAf70489a039947Fe104fe3Cc990f4270Aa5F)). After Eigenlayer registration, operators can register with GizaAVS using the `registerOperator` function. Once registered, operators can opt-in to run the `DemoApp` by calling the `optInClientAppId` function.

```mermaid
sequenceDiagram
    participant Eigenlayer
    participant Operator
    participant GizaAVS

    Operator->>Eigenlayer: Register
    Operator->>GizaAVS: Register in AVS
    Operator->>GizaAVS: Opt-in for running ClientApp
    Operator->Operator: Download associated docker image
```

## AVS Flow

When a task is requested, the `TaskRegistry` emits a `TaskRequested` event. Operators monitor these events and execute tasks using the associated Docker image. After execution, operators send signed results to the Aggregator, which waits for a quorum before verifying signatures and broadcasting consensus results to the `TaskRegistry`.

```mermaid
sequenceDiagram
    participant TaskRegistry
    participant Operator
    participant Aggregator

    TaskRegistry->>Operator: Listen for TaskRequested event
    Operator->>Operator: Run local image associated to the ClientApp
    Operator->> Operator: Sign the result
    Operator->>Aggregator: Send signed result
    Aggregator->>Aggregator: Wait for Operator results
    Aggregator->>Aggregator: Once quorum reached perform signature verification
    Aggregator->>TaskRegistry: Broadcast the consensus result
```

## Design Decisions and Future Improvements

To maintain simplicity in this proof of concept while focusing on core off-chain/on-chain interactions, we simplified several components that will be enhanced in future iterations:

- **Operator Registration**: Currently simplified to basic Eigenlayer registration. Future versions will implement a `RegistryCoordinator` to verify operator status before GizaAVS registration.
  
- **Operator Consensus**: Currently uses simple majority consensus. Future versions will implement more sophisticated consensus mechanisms for enhanced security.
  
- **Signature Scheme**: Currently uses ECDSA for simplicity. Future versions will implement BLS signatures for secure and efficient signature aggregation.

# Running the Code

## Prerequisites

The following tools are required:
- [Foundry](https://book.getfoundry.sh/getting-started/installation)
- [Rust toolchain](https://www.rust-lang.org/tools/install)

### Installing Foundry
```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### Installing Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Setup and Execution

A `Makefile` is provided for easy execution. Follow these steps in separate terminals:

1. Build contracts: `make build-contracts`
2. Start local blockchain: `make anvil`
3. Deploy contracts: `make deploy-contracts`
4. Launch first operator: `make run-operator-uji`
5. Launch second operator: `make run-operator-floki`
6. Start aggregator: `make run-aggregator`
7. Create a test task: `make create-task`

This setup creates multiple parallel processes: a local blockchain instance, two operator nodes, and an aggregator node. Creating a task triggers the demo-app execution across operators.
