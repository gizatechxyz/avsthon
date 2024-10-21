// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {Ownable} from "./Ownable.sol";
import {IClientAppRegistry} from "./interfaces/IClientAppRegistry.sol";
import {IAVSDirectory} from "eigenlayer-contracts/src/contracts/interfaces/IAVSDirectory.sol";
import {ISignatureUtils} from "eigenlayer-contracts/src/contracts/interfaces/ISignatureUtils.sol";

contract GizaAvs is Ownable {
    /*//////////////////////////////////////////////////////////////
                            EVENTS
    //////////////////////////////////////////////////////////////*/

    event OperatorRegistered(address indexed operator);
    event OperatorDeregistered(address indexed operator);
    event ClientAppIdRegistered(address indexed operator, bytes32 clientAppId);
    event ClientAppIdDeregistered(address indexed operator, bytes32 clientAppId);

    /*//////////////////////////////////////////////////////////////
                              ERROR
    //////////////////////////////////////////////////////////////*/

    error ClientAppIdInvalid();
    error OperatorNotRegistered();

    /*//////////////////////////////////////////////////////////////
                              STATE
    //////////////////////////////////////////////////////////////*/

    IAVSDirectory public immutable avsDirectory;
    IClientAppRegistry public immutable clientAppRegistry;
    mapping(address operator => bool isRegistered) public isOperatorRegistered;
    mapping(address operator => mapping(bytes32 clientAppId => bool isRegistered)) public
        operatorClientAppIdRegistrationStatus;

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTOR
    //////////////////////////////////////////////////////////////*/

    constructor(address _owner, address _avsDirectory, address _clientAppRegistry) Ownable(_owner) {
        avsDirectory = IAVSDirectory(_avsDirectory);
        clientAppRegistry = IClientAppRegistry(_clientAppRegistry);
    }

    /*//////////////////////////////////////////////////////////////
                              ADMIN
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Updates the metadata URI for the AVS
     * @param _metadataURI is the metadata URI for the AVS
     * @dev only callable by the owner
     */
    function updateAVSMetadataURI(string memory _metadataURI) external onlyOwner {
        avsDirectory.updateAVSMetadataURI(_metadataURI);
    }

    /**
     * @notice Forwards a call to EigenLayer's AVSDirectory contract to confirm operator registration with the AVS
     * @param operator The address of the operator to register.
     * @param operatorSignature The signature, salt, and expiry of the operator's signature.
     */
    function registerOperatorToAVS(
        address operator,
        ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature
    ) external {
        avsDirectory.registerOperatorToAVS(operator, operatorSignature);
        isOperatorRegistered[operator] = true;
        emit OperatorRegistered(operator);
    }

    /**
     * @notice Forwards a call to EigenLayer's AVSDirectory contract to confirm operator deregistration from the AVS
     * @param operator The address of the operator to deregister.
     */
    function deregisterOperatorFromAVS(address operator) external {
        avsDirectory.deregisterOperatorFromAVS(operator);
        isOperatorRegistered[operator] = false;
        emit OperatorDeregistered(operator);
    }

    /**
     * @notice Registers a client app id with the AVS
     * @param clientAppId The client app id to register.
     */
    function optInClientAppId(bytes32 clientAppId) external {
        // Check if the client app id is registered
        if (!clientAppRegistry.isClientApp(clientAppId)) revert ClientAppIdInvalid();

        // Check if the operator is registered inside EL and our AVS
        if (!isOperatorRegistered[msg.sender]) revert OperatorNotRegistered();

        operatorClientAppIdRegistrationStatus[msg.sender][clientAppId] = true;
        emit ClientAppIdRegistered(msg.sender, clientAppId);
    }

    /*//////////////////////////////////////////////////////////////
                              ENTRYPOINTS
    //////////////////////////////////////////////////////////////*/

    /*//////////////////////////////////////////////////////////////
                              MODIFIERS
    //////////////////////////////////////////////////////////////*/
}
