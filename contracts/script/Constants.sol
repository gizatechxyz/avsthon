// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

import {StdCheats} from "lib/forge-std/src/StdCheats.sol";

abstract contract Constants is StdCheats {
    address AGGREGATOR_NODE = makeAddr("aggregatorNode");
    address OPERATOR_1 = makeAddr("operator1");
    address OPERATOR_2 = makeAddr("operator2");

    // Holesky EigenLayer
    address HOLESKY_EIGENLAYER_AVS_DIRECTORY = 0x055733000064333CaDDbC92763c58BF0192fFeBf;
    address HOLESKY_UJI_OPERATOR = 0x37893031A8484066232AcBE6bFe7E2a7A4411a7d;
}
