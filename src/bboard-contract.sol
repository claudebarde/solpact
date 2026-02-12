// SPDX-License-Identifier: MIT
// language_version 0.20
pragma solidity ^0.8.22;

import "./midnight-sol/counter-lib.sol";
import { CompactStandardLibrary as CSL } from "./midnight-sol/CompactStandardLibrary.sol";
import { Utils, Compact } from "./midnight-sol/Utils.sol";
import { WitnessUtils } from "./midnight-sol/Utils.sol";

// the "Witnesses" contract is only here to simulate the presence of a witness
// in a real deployment, the witness would be an off-chain entity
contract Witnesses {
    function localSecretKey() external pure returns (bytes32) {
        return WitnessUtils.returnsBytes32();
    }
}

contract BboardContract {
    using CounterLib for Counter;

    enum State {
        VACANT,
        OCCUPIED
    }

    State public state;
    CSL.MaybeOpString public message;
    Counter public round;
    bytes32 public owner;
    Witnesses witnesses;

    constructor() {
        state = State.VACANT;
        round.increment(1);
        owner = bytes32(0);
        message = CSL.noneOpString();
        witnesses = new Witnesses();
    }

    function publicKey(bytes32 sk, bytes32 sequence) pure private returns (bytes32 pk) {
        return CSL.persistentHash([CSL.pad32("bboard:pk:"), sequence, sk]);
    }

    function post(string memory newMessage) public {
        require(state == State.VACANT, "Attempted to post to an occupied board");
        owner = publicKey(Compact.disclose(witnesses.localSecretKey()), round.toBytes32());
        message = CSL.some(Compact.disclose(newMessage));
        state = State.OCCUPIED;
    }

    function takeDown() public returns (string memory formerMsg) {
        require(state == State.OCCUPIED, "Attempted to take down a vacant board");
        require(
            owner == publicKey(Compact.disclose(witnesses.localSecretKey()), round.toBytes32()),
            "Only the original poster can take down the message"
        );
        formerMsg = message.value;
        state = State.VACANT;
        message = CSL.noneOpString();
        round.increment(1);
        return formerMsg;
    }
}