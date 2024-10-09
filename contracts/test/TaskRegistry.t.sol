// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {TaskRegistry} from "../src/TaskRegistry.sol";
import {Ownable} from "../src/Ownable.sol";
import {TestState} from "./TestState.sol";

contract TaskRegistryTest is TestState {
    function setUp() public override {
        super.setUp();
    }

    function testSetAggregatorNode() public {
        address newAggregatorNode = address(5);

        vm.prank(owner);
        taskRegistry.setAggregatorNode(newAggregatorNode);

        assertEq(taskRegistry.aggregatorNode(), newAggregatorNode);
    }

    function testSetAggregatorNode_RevertWhen_NotOwner() public {
        address newAggregatorNode = address(5);

        vm.prank(user);
        vm.expectRevert(Ownable.Unauthorized.selector);
        taskRegistry.setAggregatorNode(newAggregatorNode);
    }

    function testSetClientAppRegistry() public {
        address newClientAppRegistry = address(6);

        vm.prank(owner);
        taskRegistry.setClientAppRegistry(newClientAppRegistry);

        assertEq(taskRegistry.clientAppRegistry(), newClientAppRegistry);
    }

    function testSetClientAppRegistry_RevertWhen_NotOwner() public {
        address newClientAppRegistry = address(6);

        vm.prank(user);
        vm.expectRevert(Ownable.Unauthorized.selector);
        taskRegistry.setClientAppRegistry(newClientAppRegistry);
    }

    function testCreateTask() public {
        bytes32 appId = bytes32(uint256(1));
        bytes32 taskId = keccak256(abi.encode(user, appId, block.timestamp));
        assertEq(uint256(taskRegistry.tasks(taskId)), uint256(TaskRegistry.TaskStatus.EMPTY));

        vm.prank(user);
        taskRegistry.createTask(appId);
        assertEq(uint256(taskRegistry.tasks(taskId)), uint256(TaskRegistry.TaskStatus.PENDING));
    }

    function testCreateTask_RevertWhen_TaskAlreadyExists() public {
        bytes32 appId = bytes32(uint256(1));

        vm.startPrank(user);
        taskRegistry.createTask(appId);

        vm.expectRevert(TaskRegistry.TaskAlreadyExists.selector);
        taskRegistry.createTask(appId);
        vm.stopPrank();
    }

    function testRespondToTask() public {
        bytes32 appId = bytes32(uint256(1));

        vm.prank(user);
        taskRegistry.createTask(appId);

        bytes32 taskId = keccak256(abi.encode(user, appId, block.timestamp));

        vm.prank(aggregatorNode);

        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.COMPLETED);
    }

    function testRespondToTask_RevertWhen_NotAggregatorNode() public {
        bytes32 appId = bytes32(uint256(1));

        vm.prank(user);
        taskRegistry.createTask(appId);

        bytes32 taskId = keccak256(abi.encode(user, appId, block.timestamp));

        vm.prank(user);
        vm.expectRevert(Ownable.Unauthorized.selector);
        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.COMPLETED);
    }

    function testRespondToTask_RevertWhen_InvalidStatus() public {
        bytes32 appId = bytes32(uint256(1));

        vm.prank(user);
        taskRegistry.createTask(appId);

        bytes32 taskId = keccak256(abi.encode(user, appId, block.timestamp));

        vm.prank(aggregatorNode);
        vm.expectRevert(TaskRegistry.InvalidTaskOperation.selector);
        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.EMPTY);

        vm.prank(aggregatorNode);
        vm.expectRevert(TaskRegistry.InvalidTaskOperation.selector);
        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.PENDING);
    }

    function testRespondToTask_RevertWhen_AlreadyRespondedCompleted() public {
        bytes32 appId = bytes32(uint256(1));

        vm.prank(user);
        taskRegistry.createTask(appId);

        bytes32 taskId = keccak256(abi.encode(user, appId, block.timestamp));

        vm.prank(aggregatorNode);
        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.COMPLETED);

        vm.prank(aggregatorNode);
        vm.expectRevert(TaskRegistry.InvalidTaskOperation.selector);
        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.FAILED);
    }

    function testRespondToTask_RevertWhen_AlreadyRespondedFailed() public {
        bytes32 appId = bytes32(uint256(1));

        vm.prank(user);
        taskRegistry.createTask(appId);

        bytes32 taskId = keccak256(abi.encode(user, appId, block.timestamp));

        vm.prank(aggregatorNode);
        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.FAILED);

        vm.prank(aggregatorNode);
        vm.expectRevert(TaskRegistry.InvalidTaskOperation.selector);
        taskRegistry.respondToTask(taskId, TaskRegistry.TaskStatus.COMPLETED);
    }
}
