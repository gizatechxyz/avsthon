// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {Ownable} from "./Ownable.sol";

struct ClientAppMetadata {
    string name;
    string description;
    string dockerUrl;
    string logoUrl;
}

contract ClientAppRegistry is Ownable {
    /*//////////////////////////////////////////////////////////////
                            EVENTS
    //////////////////////////////////////////////////////////////*/

    event ClientAppRegistered(bytes32 indexed clientAppId);

    /*//////////////////////////////////////////////////////////////
                              ERROR
    //////////////////////////////////////////////////////////////*/

    error ClientAppAlreadyExists();

    /*//////////////////////////////////////////////////////////////
                              STATE
    //////////////////////////////////////////////////////////////*/

    mapping(bytes32 => ClientAppMetadata) public clientApps;
    mapping(bytes32 => bool) public isClientApp;

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTOR
    //////////////////////////////////////////////////////////////*/

    constructor(address _owner) Ownable(_owner) {}

    /*//////////////////////////////////////////////////////////////
                              ADMIN
    //////////////////////////////////////////////////////////////*/

    function registerClientApp(bytes32 clientAppId, ClientAppMetadata calldata metadata) public onlyOwner {
        if (isClientApp[clientAppId]) revert ClientAppAlreadyExists();
        clientApps[clientAppId] = metadata;
        isClientApp[clientAppId] = true;
        emit ClientAppRegistered(clientAppId);
    }

    /*//////////////////////////////////////////////////////////////
                              ENTRYPOINTS
    //////////////////////////////////////////////////////////////*/

    function getClientAppMetadata(bytes32 clientAppId) public view returns (ClientAppMetadata memory) {
        return clientApps[clientAppId];
    }

    /*//////////////////////////////////////////////////////////////
                              MODIFIERS
    //////////////////////////////////////////////////////////////*/
}
