############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

HOLESKY_RPC_URL=https://eth-holesky.g.alchemy.com/v2/8lbq3evplhjE7rP48rxeMXcpDNTGz0Hf
ANVIL_RPC_URL=http://localhost:8545
DEPLOYER_PK=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
GIZA_AVS_ADDRESS=0x6Da3D07a6BF01F02fB41c02984a49B5d9Aa6ea92
TASK_REGISTRY_ADDRESS=0xa8d297D643a11cE83b432e87eEBce6bee0fd2bAb
CLIENT_APP_REGISTRY_ADDRESS=0xb4e9A5BC64DC07f890367F72941403EEd7faDCbB
TASK_ID=0xc86aab04e8ef18a63006f43fa41a2a0150bae3dbe276d581fa8b5cde0ccbc966

-----------------------------: ##

___CONTRACTS___: ##

anvil: ## starts anvil
	anvil --ipc --fork-url $(HOLESKY_RPC_URL)

build-contracts: ## builds all contracts
	cd contracts && forge build

test-contracts: ## tests all contracts
	cd contracts && forge test

deploy-contracts: ## deploy contracts  (you need to run anvil first in a separate terminal and the contract deployed)
	cd contracts && forge script script/DeployContracts.s.sol --rpc-url $(ANVIL_RPC_URL) --broadcast --private-key $(DEPLOYER_PK)

__TASKS__: ##
create-task: ## create a task (you need to run anvil first in a separate terminal and the contract deployed)
	cast send $(TASK_REGISTRY_ADDRESS)  "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(ANVIL_RPC_URL)