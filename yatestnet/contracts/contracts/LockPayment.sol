// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

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
  * - Spender - the address that spends the funds
  * - Funder - the address that deposits the funds
  */

    struct Deposit {
        address spender; //address that can spend the funds provided by customer
        uint128 amount; //remaining funds locked
        uint128 feeAmount; //fee amount locked for spender
        uint64 validTo; //after this timestamp funds can be returned to customer
    }

    struct DepositView {
        uint256 id;     //unique id
        uint64 nonce;  //nonce unique for each funder
        address funder; //address that can spend the funds provided by customer
        address spender; //address that can spend the funds provided by customer
        uint128 amount; //remaining funds locked
        uint128 feeAmount; //fee amount locked for spender
        uint64 validTo; //after this timestamp funds can be returned to customer
    }

/**
 * @dev This contract is part of GLM payment system. Visit https://golem.network for details.
 * Be careful when interacting with this contract, because it has no exit mechanism. Any assets sent directly to this contract will be lost.
 */
contract LockPayment {
    IERC20 public GLM;

    // deposit is stored using arbitrary id
    mapping(uint256 => Deposit) public deposits;

    // fees are stored using spender address
    mapping(address => uint128) public funds;
    constructor(IERC20 _GLM) {
        GLM = _GLM;
    }

    function idFromNonce(uint64 nonce) public view returns (uint256) {
        return idFromNonceAndFunder(nonce, msg.sender);
    }

    function idFromNonceAndFunder(uint64 nonce, address funder) public pure returns (uint256) {
        return (uint256(uint160(funder)) << 96) ^ uint256(nonce);
    }

    function nonceFromId(uint256 id) public pure returns (uint64) {
        return uint64(id);
    }

    function funderFromId(uint256 id) public pure returns (address) {
        return address(uint160(id >> 96));
    }

    function getMyDeposit(uint64 nonce) public view returns (DepositView memory) {
        Deposit memory deposit = deposits[idFromNonce(nonce)];
        return DepositView(idFromNonce(nonce), nonce, funderFromId(idFromNonce(nonce)), deposit.spender, deposit.amount, deposit.feeAmount, deposit.validTo);
    }

    function getDeposit(uint256 id) public view returns (DepositView memory) {
        Deposit memory deposit = deposits[id];
        return DepositView(id, nonceFromId(id), funderFromId(id), deposit.spender, deposit.amount, deposit.feeAmount, deposit.validTo);
    }

    function getDeposit2(uint64 nonce, address funder) public view returns (DepositView memory) {
        uint256 id = idFromNonceAndFunder(nonce, funder);
        Deposit memory deposit = deposits[id];
        return DepositView(id, nonceFromId(id), funderFromId(id), deposit.spender, deposit.amount, deposit.feeAmount, deposit.validTo);
    }

    // createDeposit - Customer locks funds for usage by spender
    //
    // id - unique id (build from Funder address and nonce)
    // spender - the address that is allowed to spend the funds regardless of time
    // amount - amount of GLM tokens to lock
    // feeAmount - amount of GLM tokens given to spender (non-refundable). Fee is claimed by spender when called payoutSingle or payoutMultiple first time.
    // blockNo - block number until which funds are guaranteed to be locked for spender.
    //           Spender still can use the funds after this block,
    //           but customer can request the funds to be returned clearing deposit after (or equal to) this block number.
    function createDeposit(uint64 nonce, address spender, uint128 amount, uint128 feeAmount, uint64 validToTimestamp) public {
        //check if id is not used
        uint256 id = idFromNonce(nonce);
        require(deposits[id].amount == 0, "deposits[id].amount == 0");
        require(amount > 0, "amount > 0");
        require(spender != address(0), "spender cannot be null address");
        require(msg.sender != spender, "spender cannot be funder");
        require(GLM.transferFrom(msg.sender, address(this), amount + feeAmount), "transferFrom failed");
        deposits[id] = Deposit(spender, amount, feeAmount, validToTimestamp);
    }

    function extendDeposit(uint64 nonce, uint128 extraAmount, uint128 extraFee, uint64 validToTimestamp) public {
        uint256 id = idFromNonce(nonce);
        Deposit memory deposit = deposits[id];
        require(GLM.transferFrom(msg.sender, address(this), extraAmount + extraFee), "transferFrom failed");
        require(deposit.validTo <= validToTimestamp, "deposit.validTo <= validTo");
        deposit.amount += extraAmount;
        deposit.feeAmount += extraFee;
        deposit.validTo = validToTimestamp;
        deposits[id] = deposit;
    }

    // Spender can close deposit anytime claiming fee and returning rest of funds to Funder
    function closeDeposit(uint256 id) public {
        Deposit memory deposit = deposits[id];
        // customer cannot return funds before block_no
        // sender can return funds at any time
        require(msg.sender == deposit.spender);
        require(GLM.transfer(funderFromId(id), deposit.amount + deposit.feeAmount), "return transfer failed");
        if (deposit.feeAmount > 0) {
            require(GLM.transfer(deposit.spender, deposit.feeAmount), "fee transfer failed");
            deposit.feeAmount = 0;
        }
        deposits[id].amount = 0;
        deposits[id].feeAmount = 0;
    }

    // funder can terminate deposit after validTo date elapses
    function terminateDeposit(uint64 nonce) public {
        uint256 id = idFromNonce(nonce);
        Deposit memory deposit = deposits[id];
        // customer cannot return funds before block_no
        // sender can return funds at any time
        require(deposit.validTo < block.timestamp);
        require(GLM.transfer(msg.sender, deposit.amount + deposit.feeAmount), "transfer failed");
        deposits[id].amount = 0;
        deposits[id].feeAmount = 0;
    }

    function depositTransfer(uint256 id, bytes32[] calldata payments) public {
        Deposit memory deposit = deposits[id];
        require(msg.sender == deposit.spender, "msg.sender == deposit.spender");

        for (uint32 i = 0; i < payments.length; ++i) {
            // A payment contains compressed data:
            // first 160 bits (20 bytes) is an address.
            // following 96 bits (12 bytes) is a value,
            bytes32 payment = payments[i];
            address addr = address(bytes20(payment));
            uint128 amount = uint128(uint256(payment) % 2 ** 96);
            require(addr != deposit.spender, "cannot transfer to spender");
            require(GLM.transferFrom(msg.sender, addr, amount), "transferFrom failed");
            require(deposit.amount >= amount, "deposit.amount >= amount");
            deposit.amount -= amount;
        }

        deposits[id].amount = deposit.amount;
    }

    function depositTransferAndClose(uint256 id, bytes32[] calldata payments) public {
        depositTransfer(id, payments);
        closeDeposit(id);
    }

}
