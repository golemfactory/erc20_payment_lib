// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/**
 * @dev Interface of the ERC20 standard as defined in the EIP.
 */
interface IERC20 {
    /**
     * @dev Emitted when `value` tokens are moved from one account (`from`) to
     * another (`to`).
     *
     * Note that `value` may be zero.
     */
    event Transfer(address indexed from, address indexed to, uint256 value);

    /**
     * @dev Emitted when the allowance of a `spender` for an `owner` is set by
     * a call to {approve}. `value` is the new allowance.
     */
    event Approval(address indexed owner, address indexed spender, uint256 value);

    /**
     * @dev Returns the amount of tokens in existence.
     */
    function totalSupply() external view returns (uint256);

    /**
     * @dev Returns the amount of tokens owned by `account`.
     */
    function balanceOf(address account) external view returns (uint256);

    /**
     * @dev Moves `amount` tokens from the caller's account to `to`.
     *
     * Returns a boolean value indicating whether the operation succeeded.
     *
     * Emits a {Transfer} event.
     */
    function transfer(address to, uint256 amount) external returns (bool);

    /**
     * @dev Returns the remaining number of tokens that `spender` will be
     * allowed to spend on behalf of `owner` through {transferFrom}. This is
     * zero by default.
     *
     * This value changes when {approve} or {transferFrom} are called.
     */
    function allowance(address owner, address spender) external view returns (uint256);

    /**
     * @dev Sets `amount` as the allowance of `spender` over the caller's tokens.
     *
     * Returns a boolean value indicating whether the operation succeeded.
     *
     * IMPORTANT: Beware that changing an allowance with this method brings the risk
     * that someone may use both the old and the new allowance by unfortunate
     * transaction ordering. One possible solution to mitigate this race
     * condition is to first reduce the spender's allowance to 0 and set the
     * desired value afterwards:
     * https://github.com/ethereum/EIPs/issues/20#issuecomment-263524729
     *
     * Emits an {Approval} event.
     */
    function approve(address spender, uint256 amount) external returns (bool);

    /**
     * @dev Moves `amount` tokens from `from` to `to` using the
     * allowance mechanism. `amount` is then deducted from the caller's
     * allowance.
     *
     * Returns a boolean value indicating whether the operation succeeded.
     *
     * Emits a {Transfer} event.
     */
    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) external returns (bool);
}


struct Allocation {
    uint128 amount;
    address requestor;
    uint32 block_no;
}


/**
 * @dev This contract is part of GLM payment system. Visit https://golem.network for details.
 * Be careful when interacting with this contract, because it has no exit mechanism. Any assets sent directly to this contract will be lost.
 */
contract LockPayment {
    IERC20 public GLM;

    //store amount
    mapping(uint32 => Allocation) public lockedAmounts;
    /**
     * @dev Contract works only on currency specified during contract deployment
     */
    constructor(IERC20 _GLM) {
        GLM = _GLM;
    }

    // lock funds for requestor
    // requestor is the address of requestor that is allowed to use the funds
    // allocation_id is unique id for requestor
    // amount is amount of GLM tokens to lock
    // block_no is block number until which funds are locked
    function lockForRequestor(address requestor, uint32 allocation_id, uint128 amount, uint32 block_no) external {
        //check if allocation_id is not used
        require(lockedAmounts[allocation_id].amount == 0, "lockedAmounts[allocation_id].amount == 0");
        require(GLM.transferFrom(msg.sender, address(this), amount), "transferFrom failed");
        //store the amount
        lockedAmounts[allocation_id] = Allocation(amount, requestor, block_no);
    }

    function returnRemainingFunds(uint32 allocation_id) external {
        Allocation memory allocation = lockedAmounts[allocation_id];
        require(allocation.block_no >= block.number, "allocation.block_no >= block.number");
        require(GLM.transfer(allocation.requestor, allocation.amount), "transfer failed");
        delete lockedAmounts[allocation_id];
    }

    function payoutSingle(uint32 allocation_id, address recipient, uint128 amount) external {
        Allocation memory allocation = lockedAmounts[allocation_id];
        require(msg.sender == allocation.requestor, "msg.sender == allocation.requestor");
        require(allocation.amount >= amount, "allocation.amount >= amount");
        require(GLM.transfer(recipient, amount), "transfer failed");
        //update allocation amount
        allocation.amount -= amount;
        //update allocation
        if (allocation.amount == 0) {
            delete lockedAmounts[allocation_id];
        } else {
            lockedAmounts[allocation_id] = allocation;
        }
    }

    function payoutMultiple(uint32 allocation_id, bytes32[] calldata payments) external {
        Allocation memory allocation = lockedAmounts[allocation_id];
        for (uint i = 0; i < payments.length; ++i) {
            // A payment contains compressed data:
            // first 160 bits (20 bytes) is an address.
            // following 96 bits (12 bytes) is a value,
            bytes32 payment = payments[i];
            address addr = address(bytes20(payment));
            uint128 amount = uint128(uint(payment) % 2**96);
            require(GLM.transferFrom(msg.sender, addr, amount), "transferFrom failed");
            require(allocation.amount >= amount, "allocation.amount >= amount");
            allocation.amount -= amount;
        }
        //update allocation
        if (allocation.amount == 0) {
            delete lockedAmounts[allocation_id];
        } else {
            lockedAmounts[allocation_id] = allocation;
        }
    }
}
