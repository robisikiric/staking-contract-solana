Staking Contract on Solana


Data Structures:
- StakingManager holds the global state of the staking program.
- UserStakeInfo holds individual user staking data.


Serialization: Implemented using the Pack trait to handle serialization and deserialization of account data.
- Entry Point: The process_instruction function serves as the entry point, dispatching calls to specific functions based on the instruction data.


Functions:
- initialize: Sets up the staking manager.
- deposit: Allows users to stake tokens and updates their staking account.
- unstake: Allows users to withdraw staked tokens.
- start_epoch: Starts a new epoch with specified parameters.
- claim: Allows users to claim their rewards based on their staked amount.
- get_user_staked_amount: Retrieves the staked amount for a user.
- calculate_rewards: Calculates the rewards for a user based on their staked amount and the total rewards for the epoch.
