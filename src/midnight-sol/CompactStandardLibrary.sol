// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

library CompactStandardLibrary {
    struct MaybeBytes {
        bool isSome;
        bytes value;
    }

    struct MaybeOpString {
        bool isSome;
        string value;
    }

    function noneOpString() internal pure returns (MaybeOpString memory) {
        return MaybeOpString({ isSome: false, value: "" });
    }

    function someOpString(string memory value) internal pure returns (MaybeOpString memory) {
        return MaybeOpString({ isSome: true, value: value });
    }

    function someBytes(bytes memory value) internal pure returns (MaybeBytes memory) {
        return MaybeBytes({ isSome: true, value: value });
    }

    function noneBytes() internal pure returns (MaybeBytes memory) {
        return MaybeBytes({ isSome: false, value: "" });
    }

    function persistentHash(bytes32[3] memory elements) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(elements[0], elements[1], elements[2]));
    }

    function pad(uint256 n, string memory value) internal pure returns (bytes memory) {
        bytes memory src = bytes(value);
        bytes memory out = new bytes(n);
        uint256 len = src.length < n ? src.length : n;
        for (uint256 i = 0; i < len; i++) {
            out[i] = src[i];
        }
        return out;
    }

    function pad32(string memory value) internal pure returns (bytes32 padded) {
        bytes memory src = bytes(value);
        bytes memory out = new bytes(32);
        uint256 len = src.length < 32 ? src.length : 32;
        for (uint256 i = 0; i < len; i++) {
            out[i] = src[i];
        }
        return bytes32(out);
    }
}
