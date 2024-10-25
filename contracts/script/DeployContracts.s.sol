// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {Constants} from "./Constants.sol";
import {TaskRegistry} from "../src/TaskRegistry.sol";
import {ClientAppRegistry, ClientAppMetadata} from "../src/ClientAppRegistry.sol";
import {GizaAvs} from "../src/GizaAvs.sol";

contract DeployContracts is Script, Constants {
    TaskRegistry public taskRegistry;
    ClientAppRegistry public clientAppRegistry;
    GizaAvs public gizaAvs;

    function run() public {
        console2.log("Deploying contracts...");
        console2.log("Deployer address :", msg.sender);

        vm.startBroadcast();

        // Deploy contracts
        clientAppRegistry = new ClientAppRegistry(msg.sender);
        taskRegistry = new TaskRegistry(msg.sender, HOLESKY_AGGREGATOR_NODE, address(clientAppRegistry));
        gizaAvs = new GizaAvs(msg.sender, HOLESKY_EIGENLAYER_AVS_DIRECTORY, address(clientAppRegistry));

        console2.log("ClientAppRegistry deployed at %s", address(clientAppRegistry));
        console2.log("TaskRegistry deployed at %s", address(taskRegistry));
        console2.log("GizaAvs deployed at %s", address(gizaAvs));

        // Register tasks
        bytes32 clientAppId = keccak256("ethereum-block-number");
        console2.logBytes32(clientAppId);
        clientAppRegistry.registerClientApp(
            clientAppId,
            ClientAppMetadata({
                name: "Ethereum Block Number",
                description: "This task returns the current block number of the Ethereum network.",
                logoUrl: "",
                dockerUrl: "https://hub.docker.com/layers/chalex443/ethereum-block-number/latest/images/sha256:b0aba1ccde00369af2432a13e3412a9ac037a521c25e94da91c8c7b5f30b6544"
            })
        );

        vm.stopBroadcast();
    }
}
