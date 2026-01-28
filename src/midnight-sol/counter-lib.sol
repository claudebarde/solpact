// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

struct Counter {
    uint64 value;
}

library CounterLib {
    function read(Counter storage self) internal view returns (uint64) {
        return self.value;
    }

    function resetToDefault(Counter storage self) internal {
        self.value = 0;
    }

    function increment(Counter storage self, uint16 amount) internal {
        // amount is uint16 to mirror Compact increment(Uint<16>) :contentReference[oaicite:2]{index=2}
        uint64 v = self.value;
        uint64 nv = v + uint64(amount);
        // Overflow is already checked in Solidity 0.8+, but keeping it explicit is fine:
        require(nv >= v, "Counter: overflow");
        self.value = nv;
    }

    function decrement(Counter storage self, uint16 amount) internal {
        // Compact says decrementing below zero is a runtime error :contentReference[oaicite:3]{index=3}
        uint64 v = self.value;
        require(v >= amount, "Counter: below zero");
        self.value = v - uint64(amount);
    }

    function lessThan(Counter storage self, uint64 threshold) internal view returns (bool) {
        return self.value < threshold;
    }

    function toBytes32(Counter storage self) internal view returns (bytes32) {
        return bytes32(uint256(self.value));
    }
}