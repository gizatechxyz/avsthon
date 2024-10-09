// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {Test} from "forge-std/Test.sol";
import {TaskRegistry} from "src/TaskRegistry.sol";
import {ClientAppRegistry} from "src/ClientAppRegistry.sol";

contract TestState is Test {
    address public owner = makeAddr("owner");
    address public user = makeAddr("user");
    address public aggregatorNode = makeAddr("aggregatorNode");

    TaskRegistry public taskRegistry;
    ClientAppRegistry public clientAppRegistry;

    function setUp() public virtual {
        vm.startPrank(owner);
        clientAppRegistry = new ClientAppRegistry(owner);
        taskRegistry = new TaskRegistry(owner, aggregatorNode, address(clientAppRegistry));
        vm.stopPrank();
        vm.label(owner, "owner");
        vm.label(user, "user");
        vm.label(aggregatorNode, "aggregatorNode");
        vm.label(address(clientAppRegistry), "clientAppRegistry");
    }

    function test_initialState() public view {
        assertEq(taskRegistry.owner(), owner);
        assertEq(taskRegistry.aggregatorNode(), aggregatorNode);
        assertEq(taskRegistry.clientAppRegistry(), address(clientAppRegistry));
    }
}
