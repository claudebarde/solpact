// SPDX-License-Identifier: MIT
// language_version >= 0.16 && <= 0.18
pragma solidity ^0.8.22;

import "./midnight-sol/counter-lib.sol";
import { CompactStandardLibrary as CSL } from "./midnight-sol/CompactStandardLibrary.sol";
import { Utils } from "./midnight-sol/Utils.sol";
import { WitnessUtils } from "./midnight-sol/Utils.sol";

contract Witnesses {
    function localSecretKey() external pure returns (bytes32) {
        return WitnessUtils.returnsBytes32();
    }
}

contract BboardContract {
    using CounterLib for Counter;

    enum State {
        Vacant,
        Occupied
    }

    State public state;
    CSL.MaybeOpString public message;
    Counter public round;
    bytes32 public owner;
    Witnesses witnesses;

    constructor() {
        state = State.Vacant;
        round.increment(1);
        owner = bytes32(0);
        message = CSL.noneOpString();
        witnesses = new Witnesses();
    }

    function post(string memory newMessage) public {
        require(state == State.Vacant, "Attempted to post to an occupied board");
        owner = publicKey(witnesses.localSecretKey(), round.toBytes32());
        message = CSL.someOpString(newMessage);
        state = State.Occupied;
    }

    function takeDown() public returns (string memory formerMsg) {
        require(state == State.Occupied, "Attempted to take down a vacant board");
        require(
            owner == publicKey(Utils.ownPublicKey(msg.sender), round.toBytes32()),
            "Only the original poster can take down the message"
        );
        formerMsg = message.value;
        state = State.Vacant;
        message = CSL.noneOpString();
        round.increment(1);
        return formerMsg;
    }

    function publicKey(bytes32 sk, bytes32 sequence) pure private returns (bytes32 pk) {
        return CSL.persistentHash([CSL.pad32("bboard:pk:"), sequence, sk]);
    }
}
