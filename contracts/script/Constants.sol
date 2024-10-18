// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {StdCheats} from "lib/forge-std/src/StdCheats.sol";

abstract contract Constants is StdCheats {
    address AGGREGATOR_NODE = makeAddr("aggregatorNode");
    address OPERATOR_1 = makeAddr("operator1");
    address OPERATOR_2 = makeAddr("operator2");
}
