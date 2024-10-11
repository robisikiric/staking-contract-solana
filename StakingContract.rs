use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program_pack::{IsInitialized, Pack, Sealed},
    sysvar::{rent::Rent, Sysvar},
    program::{invoke, invoke_signed},
    system_instruction,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StakingManager {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub stake_token: Pubkey,
    pub reward_token: Pubkey,
    pub tokens_staked: u64,
    pub current_epoch_reward: u64,
    pub current_epoch_start_time: u64,
    pub current_epoch_end_time: u64,
    pub epoch_id: u16,
}

impl Sealed for StakingManager {}

impl IsInitialized for StakingManager {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for StakingManager {
    const LEN: usize = 1 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 2;
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, StakingManager::LEN];
        let (
            is_initialized_dst,
            owner_dst,
            stake_token_dst,
            reward_token_dst,
            tokens_staked_dst,
            current_epoch_reward_dst,
            current_epoch_start_time_dst,
            current_epoch_end_time_dst,
            epoch_id_dst,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 8, 8, 8, 8, 2];

        is_initialized_dst[0] = self.is_initialized as u8;
        owner_dst.copy_from_slice(self.owner.as_ref());
        stake_token_dst.copy_from_slice(self.stake_token.as_ref());
        reward_token_dst.copy_from_slice(self.reward_token.as_ref());
        *tokens_staked_dst = self.tokens_staked.to_le_bytes();
        *current_epoch_reward_dst = self.current_epoch_reward.to_le_bytes();
        *current_epoch_start_time_dst = self.current_epoch_start_time.to_le_bytes();
        *current_epoch_end_time_dst = self.current_epoch_end_time.to_le_bytes();
        *epoch_id_dst = self.epoch_id.to_le_bytes();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, StakingManager::LEN];
        let (
            is_initialized,
            owner,
            stake_token,
            reward_token,
            tokens_staked,
            current_epoch_reward,
            current_epoch_start_time,
            current_epoch_end_time,
            epoch_id,
        ) = array_refs![src, 1, 32, 32, 32, 8, 8, 8, 8, 2];

        Ok(StakingManager {
            is_initialized: is_initialized[0] != 0,
            owner: Pubkey::new_from_array(*owner),
            stake_token: Pubkey::new_from_array(*stake_token),
            reward_token: Pubkey::new_from_array(*reward_token),
            tokens_staked: u64::from_le_bytes(*tokens_staked),
            current_epoch_reward: u64::from_le_bytes(*current_epoch_reward),
            current_epoch_start_time: u64::from_le_bytes(*current_epoch_start_time),
            current_epoch_end_time: u64::from_le_bytes(*current_epoch_end_time),
            epoch_id: u16::from_le_bytes(*epoch_id),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserStakeInfo {
    pub is_initialized: bool,
    pub user: Pubkey,
    pub staked_amount: u64,
}

impl Sealed for UserStakeInfo {}

impl Pack for UserStakeInfo {
    const LEN: usize = 1 + 32 + 8;
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, UserStakeInfo::LEN];
        let (is_initialized_dst, user_dst, staked_amount_dst) = mut_array_refs![dst, 1, 32, 8];

        is_initialized_dst[0] = self.is_initialized as u8;
        user_dst.copy_from_slice(self.user.as_ref());
        *staked_amount_dst = self.staked_amount.to_le_bytes();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, UserStakeInfo::LEN];
        let (is_initialized, user, staked_amount) = array_refs![src, 1, 32, 8];

        Ok(UserStakeInfo {
            is_initialized: is_initialized[0] != 0,
            user: Pubkey::new_from_array(*user),
            staked_amount: u64::from_le_bytes(*staked_amount),
        })
    }
}

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let staking_manager_account = next_account_info(account_info_iter)?;

    if staking_manager_account.owner != program_id {
        msg!("Staking manager account does not have the correct program id");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut staking_manager = StakingManager::unpack_unchecked(&staking_manager_account.data.borrow())?;
    if !staking_manager.is_initialized {
        msg!("Staking manager is not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    match instruction_data[0] {
        0 => initialize(accounts, &mut staking_manager, instruction_data)?,
        1 => deposit(accounts, &mut staking_manager, instruction_data)?,
        2 => unstake(accounts, &mut staking_manager, instruction_data)?,
        3 => start_epoch(accounts, &mut staking_manager, instruction_data)?,
        4 => claim(accounts, &mut staking_manager, instruction_data)?,
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    StakingManager::pack(staking_manager, &mut staking_manager_account.data.borrow_mut())?;
    Ok(())
}

fn initialize(
    accounts: &[AccountInfo],
    staking_manager: &mut StakingManager,
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let owner_account = next_account_info(account_info_iter)?;

    if !owner_account.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    staking_manager.is_initialized = true;
    staking_manager.owner = *owner_account.key;
    // Parse additional initialization data if needed

    Ok(())
}

fn deposit(
    accounts: &[AccountInfo],
    staking_manager: &mut StakingManager,
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user_account = next_account_info(account_info_iter)?;
    let stake_token_account = next_account_info(account_info_iter)?;
    let user_stake_account = next_account_info(account_info_iter)?;

    if !user_account.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());

    let mut user_stake_info = UserStakeInfo::unpack_unchecked(&user_stake_account.data.borrow())?;
    if !user_stake_info.is_initialized {
        user_stake_info.is_initialized = true;
        user_stake_info.user = *user_account.key;
    }

    user_stake_info.staked_amount += amount;
    UserStakeInfo::pack(user_stake_info, &mut user_stake_account.data.borrow_mut())?;

    invoke(
        &system_instruction::transfer(user_account.key, stake_token_account.key, amount),
        &[user_account.clone(), stake_token_account.clone()],
    )?;

    staking_manager.tokens_staked += amount;
    msg!("Deposited {} tokens", amount);

    Ok(())
}

fn unstake(
    accounts: &[AccountInfo],
    staking_manager: &mut StakingManager,
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user_account = next_account_info(account_info_iter)?;
    let stake_token_account = next_account_info(account_info_iter)?;
    let user_stake_account = next_account_info(account_info_iter)?;

    if !user_account.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());

    let mut user_stake_info = UserStakeInfo::unpack(&user_stake_account.data.borrow())?;
    if user_stake_info.staked_amount < amount {
        msg!("Insufficient staked tokens");
        return Err(ProgramError::InsufficientFunds);
    }

    user_stake_info.staked_amount -= amount;
    UserStakeInfo::pack(user_stake_info, &mut user_stake_account.data.borrow_mut())?;

    invoke(
        &system_instruction::transfer(stake_token_account.key, user_account.key, amount),
        &[stake_token_account.clone(), user_account.clone()],
    )?;

    staking_manager.tokens_staked -= amount;
    msg!("Unstaked {} tokens", amount);

    Ok(())
}

fn start_epoch(
    accounts: &[AccountInfo],
    staking_manager: &mut StakingManager,
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let owner_account = next_account_info(account_info_iter)?;

    if !owner_account.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let start_time = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
    let end_time = u64::from_le_bytes(instruction_data[9..17].try_into().unwrap());
    let reward_amount = u64::from_le_bytes(instruction_data[17..25].try_into().unwrap());

    if start_time <= staking_manager.current_epoch_end_time {
        msg!("Epoch start time must be after the current epoch end time");
        return Err(ProgramError::InvalidArgument);
    }

    if end_time <= start_time {
        msg!("End time must be after start time");
        return Err(ProgramError::InvalidArgument);
    }

    staking_manager.current_epoch_start_time = start_time;
    staking_manager.current_epoch_end_time = end_time;
    staking_manager.current_epoch_reward = reward_amount;
    staking_manager.epoch_id += 1;

    msg!("Started new epoch with ID {}", staking_manager.epoch_id);

    Ok(())
}

fn claim(
    accounts: &[AccountInfo],
    staking_manager: &mut StakingManager,
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user_account = next_account_info(account_info_iter)?;
    let reward_token_account = next_account_info(account_info_iter)?;
    let user_stake_account = next_account_info(account_info_iter)?;

    if !user_account.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let user_staked_amount = get_user_staked_amount(user_stake_account)?;

    let rewards = calculate_rewards(staking_manager, user_staked_amount)?;

    invoke(
        &system_instruction::transfer(reward_token_account.key, user_account.key, rewards),
        &[reward_token_account.clone(), user_account.clone()],
    )?;

    msg!("Claimed {} rewards", rewards);

    Ok(())
}

fn get_user_staked_amount(user_stake_account: &AccountInfo) -> Result<u64, ProgramError> {
    let user_stake_info = UserStakeInfo::unpack(&user_stake_account.data.borrow())?;
    if !user_stake_info.is_initialized {
        msg!("User account is not initialized");
        return Err(ProgramError::UninitializedAccount);
    }
    Ok(user_stake_info.staked_amount)
}

fn calculate_rewards(
    staking_manager: &StakingManager,
    user_staked_amount: u64,
) -> Result<u64, ProgramError> {
    if staking_manager.tokens_staked == 0 {
        return Ok(0);
    }

    let user_share = user_staked_amount as u128 * staking_manager.current_epoch_reward as u128;
    let total_staked = staking_manager.tokens_staked as u128;

    let user_reward = user_share / total_staked;

    Ok(user_reward as u64)
}