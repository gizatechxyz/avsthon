############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

DEPLOYER_PK=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

-----------------------------: ## 

___CONTRACTS___: ## 

build-contracts: ## builds all contracts
	cd contracts && forge build

test-contracts: ## tests all contracts
	cd contracts && forge test

deploy-contracts: ## deploy contracts (you need to run anvil first in a separate terminal)
	cd contracts && forge script script/DeployTaskAndAppRegistry.s.sol --rpc-url http://localhost:8545 --broadcast --private-key $(DEPLOYER_PK)
