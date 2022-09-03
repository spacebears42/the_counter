# observations
Thoughts and observations complementing the information in the brief. Depending on the point it would be raised to the team, manager, client for confirmation or clarification.

## input data
- `The client ID will be unique per client`
	- no feasible way to verify the statement
	- no feasible way to verify if a transaction is associated with the wrong client
- `not guaranteed to be ordered`
	- this affects the account management implementation
- `transaction IDs (tx) are globally unique`
	- would there be any requirements to verifying that a transaction is truly unique?
		- unique within the dataset
		- or unique against global source of truth
- `[transaction IDs] are also not guaranteed to be ordered`
	- should not add any further constraints to the design as it already has to consider previous grouping to be unordered
- `You can assume the transactions occur chronologically in the file`
	- transactions are not ordered but appear chronologically:
		- the authority issuing transactions ID's is presumably affected by async behavior
		- transaction ID's should possibly take on a non incremental ID scheme
- `Whitespaces and decimal precisions (up to four places past the decimal) must be accepted by your program`
	- the format carries parameters unnecessary to the purpose transporting data
		- would it be possible to "shift left" the issue and push for upstream to agree and adhere to a more appropriate format

## types of transaction
### deposit
- no remarks

### withdrawal
 - `If a client does not have sufficient available funds the withdrawal should fail and the total amount of funds should not change`
	 - the nature of this type of transaction requires the solution to be deterministic and to prove consistency in case of asynchronous processing

### dispute
- can you dispute both deposit and withdrawal?
- `the clients available funds should decrease by the amount disputed, their held funds should increase by the amount disputed`
	- key property when designing the account
- `a dispute references the transaction that is disputed by ID`
	- this requires an design which is able to look up previously processed transactions
- `If the tx specified by the dispute doesn't exist you can ignore it`
	- requires the transactions pertaining to a client ID to be processed sequentially as they appear in the dataset.
	- sensitive to asynchronous processing

### resolve
- affected by the same sequential ordering of a client's transaction as "dispute"
- `Instead they refer to a transaction that was under dispute by ID`
	- requires the system to be able to look up disputed transactions

### chargeback
- also affected by process ordering described in "dispute"
- there is no specifications around processing transactions for a frozen client
- `If a chargeback occurs the client's account should be immediately frozen.`
	- requires the ability to enable/disable a clients account


# Considerations
I am going at the brief with the mindset of producing operational software to evaluate the business requirements before applying any performance or memory optimisations.

The brief mentions data streaming but we are missing some context around scale and volume of data. Without employing an external database the memory usage is tied to the number of transactions processed to be able to handle dispute, resolve, and chargeback.

There has been no mention of resilience and if the software needs to be recoverable.
But judging by the data the upstream system employs some form of event sourcing
so should be able to re-run the events if required. No strategies will be implemented
for recoverability such as journaling or state files. The execution of the
software is regarded to be ephemeral, that there is no persistence or state
logic saved between runs.

Having evaluated the findings I presume a parallelised system is out of scope
for the challenge so all of the remarks considering asynchronous behavior is
moot for the current implementation.

# Implementation
Tried to keep the implementation as simple as possible with as few dependencies as possible. Although I wanted to use the brief as an excuse to make a toy implementation of a actor system using tokio it seemed a bit out of focus for the brief.

For the current implementation we are assuming that only withdrawals would be disputed.

# future work
- handle `.expect` scenarios according to requirements
- stronger data verification e.g. verify that deposit and withdrawal are associated with valid amount value upfront
- re-architect the solution into modular pipeline with greater visibility of transactional steps
- consistency checks e.g. no dispute entry for a transaction if it is already disputed etc
- utilise property based testing and more fuzzing of test data
