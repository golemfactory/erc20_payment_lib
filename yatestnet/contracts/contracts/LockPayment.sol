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


/**
  * Actors:
  * - spender - the address that spends the funds
  * -  - the address that requested the funds

  */

    struct Allocation {
        address customer; //provides the funds locked in allocation
        address spender; //address that can spend the funds provided by customer
        uint128 amount; //remaining funds locked
        uint128 feeAmount; //fee amount locked for spender
        uint32 block_no; //after this block funds can be returned to customer
    }


/**
 * @dev This contract is part of GLM payment system. Visit https://golem.network for details.
 * Be careful when interacting with this contract, because it has no exit mechanism. Any assets sent directly to this contract will be lost.
 */
contract LockPayment {
    IERC20 public GLM;

    // allocation is stored using arbitrary id
    mapping(uint32 => Allocation) public lockedAmounts;

    // fees are stored using spender address
    mapping(address => uint128) public feesToClaim;
    constructor(IERC20 _GLM) {
        GLM = _GLM;
    }

    // createAllocation - Customer locks funds for usage by spender
    //
    // id - unique id (you should search for unused id, this will become id of allocation if succeeded)
    // spender - the address that is allowed to spend the funds regardless of time
    // amount - amount of GLM tokens to lock
    // feeAmount - amount of GLM tokens given to spender (non-refundable). Fee is claimed by spender when called payoutSingle or payoutMultiple first time.
    // blockNo - block number until which funds are guaranteed to be locked for spender.
    //           Spender still can use the funds after this block,
    //           but customer can request the funds to be returned clearing allocation after (or equal to) this block number.
    function createAllocation(uint32 id, address spender, uint128 amount, uint128 feeAmount, uint32 blockNo) external {
        //check if id is not used
        require(lockedAmounts[id].amount == 0, "lockedAmounts[id].amount == 0");
        require(amount > 0, "amount > 0");

        require(GLM.transferFrom(msg.sender, address(this), amount + feeAmount), "transferFrom failed");
        lockedAmounts[id] = Allocation(msg.sender, spender, amount, feeAmount, blockNo);
    }

    // only spender and customer can return funds after block_no
    // these are two parties interested in returning funds
    function returnFunds(uint32 id) external {
        Allocation memory allocation = lockedAmounts[id];
        // customer cannot return funds before block_no
        // sender can return funds at any time
        require((msg.sender == allocation.customer && allocation.block_no <= block.number) || msg.sender == allocation.spender);
        require(GLM.transfer(allocation.customer, allocation.amount + allocation.feeAmount), "transfer failed");
        delete lockedAmounts[id];
    }


    function payoutSingle(uint32 id, address recipient, uint128 amount) external {
        Allocation memory allocation = lockedAmounts[id];
        require(msg.sender == allocation.spender, "msg.sender == allocation.spender");
        require(allocation.amount >= amount, "allocation.amount >= amount");

        require(GLM.transfer(recipient, amount), "transfer failed");
        allocation.amount -= amount;

        if (allocation.feeAmount > 0) {
            require(GLM.transfer(allocation.spender, allocation.feeAmount), "transfer failed");
            allocation.feeAmount = 0;
        }

        if (allocation.amount == 0) {
            delete lockedAmounts[id];
        } else {
            lockedAmounts[id] = allocation;
        }
    }

    function payoutMultiple(uint32 id, bytes32[] calldata payments) external {
        Allocation memory allocation = lockedAmounts[id];
        require(msg.sender == allocation.spender, "msg.sender == allocation.spender");

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

        if (allocation.feeAmount > 0) {
            require(GLM.transfer(allocation.spender, allocation.feeAmount), "transfer failed");
            allocation.feeAmount = 0;
        }

        if (allocation.amount == 0) {
            delete lockedAmounts[id];
        } else {
            lockedAmounts[id] = allocation;
        }
    }
}
