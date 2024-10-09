// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import "forge-std/Test.sol";
import {Ownable} from "src/Ownable.sol";

contract MockOwnable is Ownable(msg.sender) {
    bool public flag;

    function updateFlag() public virtual onlyOwner {
        flag = true;
    }
}

contract TestOwnable is Test {
    MockOwnable mockOwnable;
    address alice;
    address bob;

    function setUp() public {
        alice = vm.addr(1);
        vm.label(alice, "Alice");
        bob = vm.addr(2);
        vm.label(bob, "Bob");

        vm.prank(alice);
        mockOwnable = new MockOwnable();
    }

    function test_TransherOwnership() public {
        vm.prank(alice);
        mockOwnable.transferOwnership(bob);
        assertEq(mockOwnable.pendingOwner(), bob);

        vm.prank(bob);
        mockOwnable.acceptOwnership();
        assertEq(mockOwnable.owner(), bob);
        assertEq(mockOwnable.pendingOwner(), address(0x0));

        vm.prank(bob);
        mockOwnable.updateFlag();
    }

    function test_cancelTransferOwnership() public {
        vm.startPrank(alice);
        mockOwnable.transferOwnership(bob);
        assertEq(mockOwnable.pendingOwner(), bob);

        mockOwnable.cancelTransferOwnership();
        assertEq(mockOwnable.pendingOwner(), address(0x0));

        mockOwnable.updateFlag();
    }

    function test_RevertWhen_Ownable() public {
        vm.startPrank(bob);
        vm.expectRevert(Ownable.Unauthorized.selector);
        mockOwnable.updateFlag();
    }

    function test_RevertWhen_OnlyPendingOwner() public {
        vm.prank(alice);
        mockOwnable.transferOwnership(bob);
        assertEq(mockOwnable.pendingOwner(), bob);

        vm.prank(alice);
        vm.expectRevert(Ownable.Unauthorized.selector);
        mockOwnable.acceptOwnership();
    }
}
