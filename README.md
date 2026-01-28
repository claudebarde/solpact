**SOLPACT**

A simple Solidity to Compact compiler

_UNDER CONSTRUCTION 🚧_

The goal of the transpiler is to allow developers who are familiar with Solidity to write smart contracts that will then be transpiled to Compact and can be deployed to Midnight.

_WARNING ⚠️_

Solpact cannot transpile any Solidity contract to Compact. Although developers can use Solidity syntax and the contracts they write are 100% valid Solidity contracts, they must use different libraries (included in the package) to work and be transpiled correctly. These libraries are necessary to represent Compact types (e.g. `Counter`) or specific features of Compact (e.g. the `CompactStandardLibrary`).

In addition to that, there are also certain conventions to follow to help the transpiler produce the correct Compact code:

- the `pragma language_version` for Compact must be added as a comment above the Solidity `pragma solidity`
- a Compact `witness` is represented as a `contract` in Solidity, cf. the `bboard-contract.sol` example
