// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {ClientAppRegistry, ClientAppMetadata} from "../src/ClientAppRegistry.sol";
import {Ownable} from "src/Ownable.sol";
import {TestState} from "./TestState.sol";

contract ClientAppRegistryTest is TestState {
    function setUp() public override {
        super.setUp();
    }

    function testRegisterClientApp() public {
        bytes32 clientAppId = bytes32(uint256(1));
        ClientAppMetadata memory metadata = ClientAppMetadata({
            name: "Test App",
            description: "A test client application",
            logoUrl: "https://example.com/logo.png"
        });

        vm.prank(owner);
        clientAppRegistry.registerClientApp(clientAppId, metadata);

        assertTrue(clientAppRegistry.isClientApp(clientAppId));

        ClientAppMetadata memory storedMetadata = clientAppRegistry.getClientAppMetadata(clientAppId);
        assertEq(storedMetadata.name, metadata.name);
        assertEq(storedMetadata.description, metadata.description);
        assertEq(storedMetadata.logoUrl, metadata.logoUrl);
    }

    function testRegisterClientApp_RevertWhen_NotOwner() public {
        bytes32 clientAppId = bytes32(uint256(1));
        ClientAppMetadata memory metadata = ClientAppMetadata({
            name: "Test App",
            description: "A test client application",
            logoUrl: "https://example.com/logo.png"
        });

        vm.prank(user);
        vm.expectRevert(Ownable.Unauthorized.selector);
        clientAppRegistry.registerClientApp(clientAppId, metadata);
    }

    function testRegisterClientApp_RevertWhen_ClientAppAlreadyExists() public {
        bytes32 clientAppId = bytes32(uint256(1));
        ClientAppMetadata memory metadata = ClientAppMetadata({
            name: "Test App",
            description: "A test client application",
            logoUrl: "https://example.com/logo.png"
        });

        vm.startPrank(owner);
        clientAppRegistry.registerClientApp(clientAppId, metadata);

        vm.expectRevert(ClientAppRegistry.ClientAppAlreadyExists.selector);
        clientAppRegistry.registerClientApp(clientAppId, metadata);
        vm.stopPrank();
    }

    function testGetClientAppMetadata() public {
        bytes32 clientAppId = bytes32(uint256(1));
        ClientAppMetadata memory metadata = ClientAppMetadata({
            name: "Test App",
            description: "A test client application",
            logoUrl: "https://example.com/logo.png"
        });

        vm.prank(owner);
        clientAppRegistry.registerClientApp(clientAppId, metadata);

        ClientAppMetadata memory retrievedMetadata = clientAppRegistry.getClientAppMetadata(clientAppId);
        assertEq(retrievedMetadata.name, metadata.name);
        assertEq(retrievedMetadata.description, metadata.description);
        assertEq(retrievedMetadata.logoUrl, metadata.logoUrl);
    }

    function testIsClientApp() public {
        bytes32 clientAppId = bytes32(uint256(1));
        ClientAppMetadata memory metadata = ClientAppMetadata({
            name: "Test App",
            description: "A test client application",
            logoUrl: "https://example.com/logo.png"
        });

        vm.prank(owner);
        clientAppRegistry.registerClientApp(clientAppId, metadata);

        assertTrue(clientAppRegistry.isClientApp(clientAppId));
        assertFalse(clientAppRegistry.isClientApp(bytes32(uint256(2))));
    }
}
