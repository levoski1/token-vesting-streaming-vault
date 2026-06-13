use soroban_sdk::{contracttype, Address};

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Stream(Address),
}

#[contracttype]
#[derive(Clone)]
pub struct StreamState {
    pub recipient: Address,
    pub total_amount: i128,
    pub claimed_amount: i128,
    pub start_time: u64,
    pub end_time: u64,
}
