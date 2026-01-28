// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

library Utils {
    function ownPublicKey(address addr) internal pure returns (bytes32 pk) {
        return bytes32(uint256(uint160(addr)));
    }
}

library WitnessUtils {
    function returnsBytes32() internal pure returns (bytes32) {
        return bytes32(uint256(42));
    }
}