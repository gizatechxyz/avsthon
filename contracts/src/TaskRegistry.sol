// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {Ownable} from "./Ownable.sol";
import {IClientAppRegistry} from "./interfaces/IClientAppRegistry.sol";

contract TaskRegistry is Ownable {
    /*//////////////////////////////////////////////////////////////
                            EVENTS
    //////////////////////////////////////////////////////////////*/

    event TaskRequested(bytes32 indexed taskId, TaskRequest taskRequest);
    event TaskResponded(bytes32 indexed taskId, TaskStatus status, uint256 taskResult);

    /*//////////////////////////////////////////////////////////////
                              ERROR
    //////////////////////////////////////////////////////////////*/

    error TaskAlreadyExists();
    error InvalidTaskOperation();
    error InvalidAppId();
    /*//////////////////////////////////////////////////////////////
                              STATE
    //////////////////////////////////////////////////////////////*/

    struct TaskRequest {
        bytes32 appId;
    }

    enum TaskStatus {
        EMPTY,
        PENDING,
        COMPLETED,
        FAILED
    }

    address public aggregatorNode;
    address public clientAppRegistry;

    mapping(bytes32 => TaskStatus) public tasks;

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTOR
    //////////////////////////////////////////////////////////////*/

    constructor(address _owner, address _aggregatorNode, address _clientAppRegistry) Ownable(_owner) {
        aggregatorNode = _aggregatorNode;
        clientAppRegistry = _clientAppRegistry;
    }

    /*//////////////////////////////////////////////////////////////
                              ADMIN
    //////////////////////////////////////////////////////////////*/

    function setAggregatorNode(address _aggregatorNode) external onlyOwner {
        aggregatorNode = _aggregatorNode;
    }

    function setClientAppRegistry(address _clientAppRegistry) external onlyOwner {
        clientAppRegistry = _clientAppRegistry;
    }

    /*//////////////////////////////////////////////////////////////
                              ENTRYPOINTS
    //////////////////////////////////////////////////////////////*/

    function createTask(bytes32 appId) external {
        // Check that appId is registered
        if (!IClientAppRegistry(clientAppRegistry).isClientApp(appId)) revert InvalidAppId();

        // We create a pseudo unique taskId in order to keep track of the task while minimizing gas cost
        bytes32 taskId = keccak256(abi.encode(msg.sender, appId, block.timestamp));

        // Verify taskId is not already in use
        if (tasks[taskId] != TaskStatus.EMPTY) revert TaskAlreadyExists();
        tasks[taskId] = TaskStatus.PENDING;
        TaskRequest memory taskRequest = TaskRequest(appId);
        emit TaskRequested(taskId, taskRequest);
    }

    function respondToTask(bytes32 taskId, TaskStatus status, uint256 taskResult) external onlyAggregatorNode {
        // Check that status is only Completed or Failed
        if (status == TaskStatus.EMPTY || status == TaskStatus.PENDING) revert InvalidTaskOperation();

        // Check that taskId have not been responded yet
        if (tasks[taskId] != TaskStatus.PENDING) revert InvalidTaskOperation();

        tasks[taskId] = status;
        emit TaskResponded(taskId, status, taskResult);
    }

    /*//////////////////////////////////////////////////////////////
                              MODIFIERS
    //////////////////////////////////////////////////////////////*/

    modifier onlyAggregatorNode() {
        if (msg.sender != aggregatorNode) revert Unauthorized();
        _;
    }
}
