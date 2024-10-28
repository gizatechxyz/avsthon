############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# RPC URLs
HOLESKY_RPC_URL=https://eth-holesky.g.alchemy.com/v2/8lbq3evplhjE7rP48rxeMXcpDNTGz0Hf
ANVIL_RPC_URL=http://localhost:8545

# CONTRACTS ADDRESSES
GIZA_AVS_ADDRESS=0x8B64968F69E669faCc86FA3484FD946f1bBE7c91
TASK_REGISTRY_ADDRESS=0xa68E430060f74F9821D2dC9A9E2CE3aF7d842EBe
CLIENT_APP_REGISTRY_ADDRESS=0xB2ff9d5e60d68A52cea3cd041b32f1390A880365
AVS_DIRECTORY_ADDRESS=0x055733000064333CaDDbC92763c58BF0192fFeBf
OPERATOR_UJI_ADDRESS=0x37893031A8484066232AcBE6bFe7E2a7A4411a7d
OPERATOR_FLOKI_ADDRESS=0x76cCAf70489a039947Fe104fe3Cc990f4270Aa5F
AGGREGATOR_ADDRESS=0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65

#UTILS
DEPLOYER_PK=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
OPERATOR_UJI_PK=0x2a7f875389f0ce57b6d3200fb88e9a95e864a2ff589e8b1b11e56faff32a1fc5
OPERATOR_FLOKI_PK=0x277268d9094a360c4dfa3cc538cde5a8759d59759fd0b6d6a80b709718208cd8
AGGREGATOR_PK=0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a
TASK_ID=0xc86aab04e8ef18a63006f43fa41a2a0150bae3dbe276d581fa8b5cde0ccbc966

-----------------------------: ##

___CONTRACTS___: ##

anvil: ## starts anvil
	anvil --ipc --fork-url $(HOLESKY_RPC_URL) --fork-block-number 2605144

build-contracts: ## builds all contracts
	cd contracts && forge clean && forge build

test-contracts: ## tests all contracts
	cd contracts && forge test

deploy-contracts: ## deploy contracts  (you need to run anvil first in a separate terminal and the contract deployed)
	cd contracts && forge script script/DeployContracts.s.sol --rpc-url $(ANVIL_RPC_URL) --broadcast --private-key $(DEPLOYER_PK)

__TASKS__: ##
create-task: ## create a task (you need to run anvil first in a separate terminal and the contract deployed)
	cast send $(TASK_REGISTRY_ADDRESS)  "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(ANVIL_RPC_URL)

spawn-task: ## spawn a task (you need to run anvil first in a separate terminal and the contract deployed)
	@bash -c 'while true; do \
		cast send $(TASK_REGISTRY_ADDRESS) "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(ANVIL_RPC_URL); \
		echo "Waiting 5 seconds before creating next task..."; \
		sleep 5; \
	done'
__OPERATOR__: ##
run-operator-uji: ## run the operator
	cd operator && cargo run -- $(OPERATOR_UJI_PK)

run-operator-floki: ## run the operator
	cd operator && cargo run -- $(OPERATOR_FLOKI_PK)

__AGGREGATOR__: ##
run-aggregator: ## run the aggregator
	cd aggregator && cargo run
