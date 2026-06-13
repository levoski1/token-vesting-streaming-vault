#![no_std]

mod types;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, Env};
use types::StreamState;

// Storage keys
const ADMIN: &str = "admin";
const TOKEN: &str = "token";

fn stream_key(recipient: &Address) -> Address {
    recipient.clone()
}

#[contract]
pub struct VestingVault;

#[contractimpl]
impl VestingVault {
    /// Issue #1 — Initialize contract with admin and token addresses.
    pub fn init(env: Env, admin: Address, token: Address) {
        let storage = env.storage().instance();
        // Prevent re-initialization
        if storage.has(&ADMIN) {
            panic!("already initialized");
        }
        storage.set(&ADMIN, &admin);
        storage.set(&TOKEN, &token);
    }

    /// Issue #1 — Create a new linear vesting stream for a recipient.
    pub fn create_stream(
        env: Env,
        recipient: Address,
        total_amount: i128,
        start_time: u64,
        end_time: u64,
    ) {
        // Only admin can create streams
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        assert!(total_amount > 0, "amount must be positive");
        assert!(end_time > start_time, "end_time must be after start_time");

        let key = stream_key(&recipient);
        assert!(
            !env.storage().persistent().has(&key),
            "stream already exists"
        );

        let stream = StreamState {
            recipient: recipient.clone(),
            total_amount,
            claimed_amount: 0,
            start_time,
            end_time,
        };
        env.storage().persistent().set(&key, &stream);
        env.storage().persistent().extend_ttl(&key, 100_000, 100_000);

        // Transfer tokens from admin into the contract
        let token_addr: Address = env.storage().instance().get(&TOKEN).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&admin, env.current_contract_address(), &total_amount);
    }

    /// Issue #2 — Return the currently unlocked (claimable) token amount for a recipient.
    pub fn claimable_amount(env: Env, recipient: Address) -> i128 {
        let key = stream_key(&recipient);
        let stream: StreamState = env.storage().persistent().get(&key).unwrap();
        Self::unlocked(&env, &stream) - stream.claimed_amount
    }

    /// Issue #3 — Withdraw all unlocked tokens to the recipient.
    pub fn withdraw(env: Env, recipient: Address) {
        recipient.require_auth();

        let key = stream_key(&recipient);
        let mut stream: StreamState = env.storage().persistent().get(&key).unwrap();

        let claimable = Self::unlocked(&env, &stream) - stream.claimed_amount;
        assert!(claimable > 0, "nothing to withdraw");

        stream.claimed_amount += claimable;
        env.storage().persistent().set(&key, &stream);
        env.storage().persistent().extend_ttl(&key, 100_000, 100_000);

        let token_addr: Address = env.storage().instance().get(&TOKEN).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &recipient, &claimable);
    }

    // ── Internal helper ──────────────────────────────────────────────────────

    /// Linear unlock: proportional to elapsed time, capped at total_amount.
    fn unlocked(env: &Env, stream: &StreamState) -> i128 {
        let now = env.ledger().timestamp();
        if now <= stream.start_time {
            return 0;
        }
        if now >= stream.end_time {
            return stream.total_amount;
        }
        let elapsed = (now - stream.start_time) as i128;
        let duration = (stream.end_time - stream.start_time) as i128;
        stream.total_amount * elapsed / duration
    }
}
