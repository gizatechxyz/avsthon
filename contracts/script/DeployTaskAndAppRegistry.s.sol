// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {Constants} from "./Constants.sol";
import {TaskRegistry} from "../src/TaskRegistry.sol";
import {ClientAppRegistry} from "../src/ClientAppRegistry.sol";

contract DeployTaskAndAppRegistry is Script, Constants {
    TaskRegistry public taskRegistry;
    ClientAppRegistry public clientAppRegistry;

    function run() public {
        console2.log("Deploying contracts...");
        console2.log("Deployer address :", msg.sender);

        vm.startBroadcast();

        clientAppRegistry = new ClientAppRegistry(msg.sender);
        taskRegistry = new TaskRegistry(msg.sender, AGGREGATOR_NODE, address(clientAppRegistry));

        console2.log("ClientAppRegistry deployed at %s", address(clientAppRegistry));
        console2.log("TaskRegistry deployed at %s", address(taskRegistry));

        vm.stopBroadcast();
    }
}
