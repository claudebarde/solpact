// SPDX-License-Identifier: MIT
// language_version >= 0.16 && <= 0.18
pragma solidity ^0.8.22;

import "./midnight-sol/counter-lib.sol";

contract CounterContract {
    using CounterLib for Counter;

    Counter public round;

    function increment() public {
        round.increment(1);
    }
}