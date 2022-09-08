Implements a toy enginefor processing transactions on a single asset account. Deals with 5 types of transactions:

- **Deposits:** Increases the available balance of the account
- **Withdrawal:** Decreases available balance of the account
- **Dispute:** Puts the funds added through a past deposit on hold and decreases the client's available funds. The requirements for this (and the other two) were a bit vague so I made some assumptions:
    - **Only deposits can be disputed:**
        - The language in the specification makes it sound like this is the case. I was tempted to extend this to withdrawals (i.e. reverse the operations on the balances), but I realised that this might allow malicious users to do double spend (e.g. withdraw -> dispute -> withdraw) and increase the customers total funds as well as lead to weird states (e.g. negative held funds). Attempts to dispute a withdrawal are no-ops
        - Disputes, Chargebacks and Resolves reference past transctions but don't have their own transaction IDs so they can't be disputed
    - **Transactions can only be disputed once**
        - After a deposit is Resolved or Chargedback, it can't be disputed again. Attempts to dispute transactions that are in either of those states or are already in dispute will be no-ops
    - **Disputes can lead to negative available balances:** e.g. deposit -> withdraw -> dispute
- **Chargeback:** Removes the held funds from the relevant dispute and "freeze" the account
    - **Frozen accounts can perform any transaction expect withdrawals**
- **Resolve:** Makes the held funds from the relevant dispute availble again

# Known issues

- **Transaction idempotency is not handled in all cases:** For example, we do not enforce uniqueness of withdrawal transaction IDs. However, by virtue of how disputes are implemented, attempting to double deposit with the same transaction ID is a no-op.
- **Error messages are hard to trace back to specific transactions:** Eror messages printed to stderr do not have a way to reference which line of the input file triggered the error either because of serisation issues or transaction processing errors
- **Serialisation for dispute/chargeback/resolves needs a trailing comma:** I initially tried to use a tagged `enum` to represent transactions to account for the fact that only withdrawals and deposits have the amount field set. However, due to an [issue in the csv library](https://github.com/BurntSushi/rust-csv/issues/278) deserialisation didn't work so I resorted to making the `amount` field optional which requires that all records for dispute, chargeback and resolves to have a trailing comma to repreent the optional `amount` field
- **Precision is only enforced at serialisation time:** We operate with whatever input we get and only truncate to 4 decimal digis when serialising the output. This should be fine for this toy example given that we're guaranteed to have at most 4 decimal digits, but this is not ideal

# Multithreading

I managed to implement a rudimentary multithreading mechanism where we use a fixed number of `mpsc::channel`s (defaults to 8 but configurable via a CLI arg) to distribute the computaional load. We only require that transactions for the same client go to the thread so we use the modulo operator (`transaction.client % num_threads`) to ensure this.

In the current implementation there's a single "input" channel that's fed data from the input file. However, this abstraction would serve to sequence input from multiple sources (e.g. concurrent TCP streams).

I tested out this works by generating random inputs, but of course this has the fallback of not taking into account hot keys. That would require some smart way of detecing them and moving accounts across threads, smarter scheduling, which felt way beyond the scope of this.

# Using Decimal to represent amounts

I initially used `f32`s, but that felt questionable given that precision is very important. We can't have information loss because of floating point precision errors. So I opted to use [the rust-decimal crate](https://github.com/paupino/rust-decimal).

I initially considered implementing my own deserialisation function and use `i64` and express funds in the smallest possible unit (in this case 10 thousandths given the 4 precision digits), but I opted for reusing existing trusted code. Depending on the use I might reconsider this approach given that vanilla integer operations might have some performance benefits over a decimal implementation.

# The Funds type

This implementation focuses on enforcing "safe" fund handling by leveraging the type system. Although arguably not the most practical example, I used overflow safety to illustrate this point. I implemented a `Funds` wrapper type that forces the user to explicitly handle overflows on every operation.

We do this by keeping the inner value private and not implementing convenience traits such as `Deref`, `Add` or `Sub`, instead exposing our own checked versions of arithmetic operations that explicitly throw on overflows

# The Balance type

Similarly to `Funds` we have a wrapper `Balance` type for keeping track of an account's available and held funds.

We use an immutable `Copy` type in order make sure operations are atomic and "rollbacks" are trivial compared to what we would need with a mutable type. For example:

```rust
...
// add takes a &mut self and can fail
self.balance.add(available_diff, held_diff)?
...
```
In this case either we can catch the error, but do not have enough information to determine what part of the operation did succed (did we fail to add to the available funds? the held funds? both?) so we can't trivially do a rollback without taking a snapshot of the state before the change

In contrast, with the immutable API, the rollback is a no-op
```rust
...
self.balance = balance.apply(BalanceDiff::(available_diff, held_diff))?
...
```

While the only way in which balance operations can fail in the current implementation is overflows (which are arguably to enforce on every operation), this same abstraction can help enforce other constraints such as a minimum balance or held funds never going negative.

# Error Handling

This implementation leans towards being very fault tolerant in that no single error should prevent the program from making process, for example:

- Failure to process a single transaction (e.g. insufficient funds, overflows, etc.) will be caught and logged, but future transactions on the same account or other clients' transactions shouldn't be affected 
- (De)serialisation errors (e.g. malformed input or output) are treated similary: we simply log them to stderr and move on

In theory, the program shouldn't crash short of a catastrophic error such as a panic or OOM