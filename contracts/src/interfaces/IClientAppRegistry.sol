// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {ClientAppMetadata} from "src/ClientAppRegistry.sol";

interface IClientAppRegistry {
    function registerClientApp(bytes32 clientAppId, ClientAppMetadata calldata metadata) external;
    function isClientApp(bytes32 clientAppId) external view returns (bool);
}
