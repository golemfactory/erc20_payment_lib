// SPDX-License-Identifier: MIT
pragma solidity ^0.8;

// Do nothing contract
contract DoNothingContract {
    event CostlyTransactionEvent(uint256 init);

    //pass init value 0 as argument
    function costlyTransaction(uint256 loops2, uint256 init) public {
        uint256 loops = loops2;
        while(loops > 0) {
            init = init * init;
            loops -= 1;
        }
        emit CostlyTransactionEvent(loops2 + init);
    }
}
