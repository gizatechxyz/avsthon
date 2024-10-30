############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# RPC URLs
HOLESKY_RPC_URL=https://eth-holesky.g.alchemy.com/v2/8lbq3evplhjE7rP48rxeMXcpDNTGz0Hf
ANVIL_RPC_URL=http://localhost:8545

# CONTRACTS ADDRESSES
GIZA_AVS_ADDRESS=0x9f06d855F70a99fcDcA4c9f26A6066499A93923d
TASK_REGISTRY_ADDRESS=0x56421D6AEb393C5361a3f262e5b94626B7E88aD7
CLIENT_APP_REGISTRY_ADDRESS=0x0D6D127A718A2d1BBFBD809D75048058A2830B8b
AVS_DIRECTORY_ADDRESS=0x055733000064333CaDDbC92763c58BF0192fFeBf
OPERATOR_UJI_ADDRESS=0x37893031A8484066232AcBE6bFe7E2a7A4411a7d
OPERATOR_FLOKI_ADDRESS=0x76cCAf70489a039947Fe104fe3Cc990f4270Aa5F
AGGREGATOR_ADDRESS=0xAd586960C1FBfFC2384be7ef5d5d12A9858e8C2f

#UTILS
DEPLOYER_PK=0x71e769d81989880a9237a3404793b463cae6a44d99c93ff1218de8ba159ed90f
OPERATOR_UJI_PK=0x2a7f875389f0ce57b6d3200fb88e9a95e864a2ff589e8b1b11e56faff32a1fc5
OPERATOR_FLOKI_PK=0x277268d9094a360c4dfa3cc538cde5a8759d59759fd0b6d6a80b709718208cd8
AGGREGATOR_PK=0x6e7912cf57b1cd9df1b05712e92a082c8c06511f62432abdaad503060822bc72
TASK_ID=0xc86aab04e8ef18a63006f43fa41a2a0150bae3dbe276d581fa8b5cde0ccbc966

-----------------------------: ##

___CONTRACTS___: ##

anvil: ## starts anvil
	anvil --ipc --fork-url $(HOLESKY_RPC_URL) --fork-block-number 2630129

build-contracts: ## builds all contracts
	cd contracts && forge clean && forge build

test-contracts: ## tests all contracts
	cd contracts && forge test

deploy-contracts-anvil: ## deploy contracts  (you need to run anvil first in a separate terminal and the contract deployed)
	cd contracts && forge script script/DeployContracts.s.sol --rpc-url $(ANVIL_RPC_URL) --broadcast --private-key $(DEPLOYER_PK)

deploy-contracts-holesky: ## deploy contracts  (you need to run anvil first in a separate terminal and the contract deployed)
	cd contracts && forge script script/DeployContracts.s.sol --rpc-url $(HOLESKY_RPC_URL) --broadcast --private-key $(DEPLOYER_PK) --verify --etherscan-api-key BQFM4NNBMFWYUVZGBRPNVW6316AW89I1Z9

__TASKS__: ##
create-task-anvil: ## create a task (you need to run anvil first in a separate terminal and the contract deployed)
	cast send $(TASK_REGISTRY_ADDRESS)  "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(ANVIL_RPC_URL)

spawn-task-anvil: ## spawn a task (you need to run anvil first in a separate terminal and the contract deployed)
	@bash -c 'while true; do \
		cast send $(TASK_REGISTRY_ADDRESS) "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(ANVIL_RPC_URL); \
		echo "Waiting 5 seconds before creating next task..."; \
		sleep 5; \
	done'

create-task-holesky: ## create a task
	cast send $(TASK_REGISTRY_ADDRESS)  "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(HOLESKY_RPC_URL)

spawn-task-holesky: ## spawn periodic tasks
	@bash -c 'while true; do \
		cast send $(TASK_REGISTRY_ADDRESS) "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(HOLESKY_RPC_URL); \
		echo "Waiting 30 seconds before creating next task..."; \
		sleep 30; \
	done'

__OPERATOR__: ##
run-operator-uji-anvil: ## run the operator
	cd operator && cargo run -- $(OPERATOR_UJI_PK) anvil

run-operator-floki-anvil: ## run the operator
	cd operator && cargo run -- $(OPERATOR_FLOKI_PK) anvil

run-operator-uji-holesky: ## run the operator
	cd operator && cargo run -- $(OPERATOR_UJI_PK) holesky

run-operator-floki-holesky: ## run the operator
	cd operator && cargo run -- $(OPERATOR_FLOKI_PK) holesky

__AGGREGATOR__: ##
run-aggregator-anvil: ## run the aggregator
	cd aggregator && cargo run -- anvil

run-aggregator-holesky: ## run the aggregator
	cd aggregator && cargo run -- holesky
