############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

RPC_URL=http://localhost:8545
DEPLOYER_PK=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
TASK_REGISTRY_ADDRESS=0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
TASK_ID=0xc86aab04e8ef18a63006f43fa41a2a0150bae3dbe276d581fa8b5cde0ccbc966

-----------------------------: ## 

___CONTRACTS___: ## 

build-contracts: ## builds all contracts
	cd contracts && forge build

test-contracts: ## tests all contracts
	cd contracts && forge test

deploy-contracts: ## deploy contracts  (you need to run anvil first in a separate terminal and the contract deployed)
	cd contracts && forge script script/DeployTaskAndAppRegistry.s.sol --rpc-url $(RPC_URL) --broadcast --private-key $(DEPLOYER_PK)

__TASKS__: ##

create-task: ## create a task (you need to run anvil first in a separate terminal and the contract deployed)
	cast send $(TASK_REGISTRY_ADDRESS)  "createTask(bytes32)" $(TASK_ID) --private-key $(DEPLOYER_PK) --rpc-url $(RPC_URL)