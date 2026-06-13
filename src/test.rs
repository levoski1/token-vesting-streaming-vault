#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{VestingVault, VestingVaultClient};

// ── helpers ──────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (VestingVaultClient<'_>, Address, Address, Address) {
    env.mock_all_auths();

    let admin = Address::generate(env);
    let recipient = Address::generate(env);

    // Deploy a native SAC token (test utility)
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
    let token_client = token::StellarAssetClient::new(env, &token_id);

    // Mint enough tokens to admin so create_stream transfers work
    token_client.mint(&admin, &1_000_000);

    let contract_id = env.register(VestingVault, ());
    let client = VestingVaultClient::new(env, &contract_id);
    client.init(&admin, &token_id);

    (client, admin, recipient, token_id)
}

fn set_time(env: &Env, ts: u64) {
    env.ledger().with_mut(|l| l.timestamp = ts);
}

// ── init ─────────────────────────────────────────────────────────────────────

#[test]
fn test_init_ok() {
    let env = Env::default();
    setup(&env); // must not panic
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_init_twice_panics() {
    let env = Env::default();
    let (client, admin, _, token_id) = setup(&env);
    client.init(&admin, &token_id);
}

// ── create_stream ─────────────────────────────────────────────────────────────

#[test]
fn test_create_stream_ok() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    set_time(&env, 0);
    client.create_stream(&recipient, &1_000, &100, &200);
    // claimable at start == 0
    assert_eq!(client.claimable_amount(&recipient), 0);
}

#[test]
#[should_panic(expected = "end_time must be after start_time")]
fn test_create_stream_bad_times() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    client.create_stream(&recipient, &1_000, &200, &100);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_create_stream_zero_amount() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    client.create_stream(&recipient, &0, &100, &200);
}

// ── claimable_amount ──────────────────────────────────────────────────────────

#[test]
fn test_claimable_before_start() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    set_time(&env, 50);
    client.create_stream(&recipient, &1_000, &100, &200);
    set_time(&env, 99);
    assert_eq!(client.claimable_amount(&recipient), 0);
}

#[test]
fn test_claimable_midpoint() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    client.create_stream(&recipient, &1_000, &0, &200);
    set_time(&env, 100); // 50% elapsed
    assert_eq!(client.claimable_amount(&recipient), 500);
}

#[test]
fn test_claimable_after_end() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    client.create_stream(&recipient, &1_000, &0, &100);
    set_time(&env, 200);
    assert_eq!(client.claimable_amount(&recipient), 1_000);
}

// ── withdraw ──────────────────────────────────────────────────────────────────

#[test]
fn test_withdraw_partial() {
    let env = Env::default();
    let (client, _, recipient, token_id) = setup(&env);
    client.create_stream(&recipient, &1_000, &0, &200);

    set_time(&env, 100); // 50% = 500 tokens
    client.withdraw(&recipient);

    let bal = token::Client::new(&env, &token_id).balance(&recipient);
    assert_eq!(bal, 500);

    // claimable resets to 0 just after withdrawal
    assert_eq!(client.claimable_amount(&recipient), 0);
}

#[test]
fn test_withdraw_full() {
    let env = Env::default();
    let (client, _, recipient, token_id) = setup(&env);
    client.create_stream(&recipient, &1_000, &0, &100);

    set_time(&env, 100);
    client.withdraw(&recipient);

    let bal = token::Client::new(&env, &token_id).balance(&recipient);
    assert_eq!(bal, 1_000);
}

#[test]
#[should_panic(expected = "nothing to withdraw")]
fn test_withdraw_nothing() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    set_time(&env, 0);
    client.create_stream(&recipient, &1_000, &100, &200);
    set_time(&env, 50); // before start
    client.withdraw(&recipient);
}

#[test]
fn test_withdraw_twice_accumulates_correctly() {
    let env = Env::default();
    let (client, _, recipient, token_id) = setup(&env);
    client.create_stream(&recipient, &1_000, &0, &1000);

    set_time(&env, 250); // 25% = 250
    client.withdraw(&recipient);

    set_time(&env, 500); // 50% total = 500; already claimed 250 → claimable 250
    assert_eq!(client.claimable_amount(&recipient), 250);
    client.withdraw(&recipient);

    let bal = token::Client::new(&env, &token_id).balance(&recipient);
    assert_eq!(bal, 500);
}

// ── additional edge cases ─────────────────────────────────────────────────────

#[test]
fn test_create_stream_duplicate_recipient_panics() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    set_time(&env, 0);
    client.create_stream(&recipient, &1_000, &100, &200);
    set_time(&env, 150);
    assert_eq!(client.claimable_amount(&recipient), 500);
}

#[test]
#[should_panic(expected = "stream already exists")]
fn test_create_stream_duplicate_recipient_create_again_panics() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    client.create_stream(&recipient, &1_000, &100, &200);
    client.create_stream(&recipient, &500, &100, &200);
}

#[test]
fn test_claimable_before_any_stream() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    set_time(&env, 100);
    client.create_stream(&recipient, &1_000, &200, &400);
    set_time(&env, 150);
    assert_eq!(client.claimable_amount(&recipient), 0);
}

#[test]
fn test_withdraw_exact_at_end_time() {
    let env = Env::default();
    let (client, _, recipient, token_id) = setup(&env);
    client.create_stream(&recipient, &1_000, &0, &100);

    set_time(&env, 100);
    client.withdraw(&recipient);

    let bal = token::Client::new(&env, &token_id).balance(&recipient);
    assert_eq!(bal, 1_000);
    assert_eq!(client.claimable_amount(&recipient), 0);
}

#[test]
fn test_claimable_fractional_rounding() {
    let env = Env::default();
    let (client, _, recipient, _) = setup(&env);
    // 7 tokens over 3 seconds → each second yields 2 tokens (floor), remainder lost
    client.create_stream(&recipient, &7, &0, &3);

    set_time(&env, 1);
    assert_eq!(client.claimable_amount(&recipient), 2);

    set_time(&env, 2);
    assert_eq!(client.claimable_amount(&recipient), 4);

    set_time(&env, 3);
    assert_eq!(client.claimable_amount(&recipient), 7);
}

#[test]
#[should_panic]
fn test_admin_cannot_withdraw_recipient_stream() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, recipient, _) = setup(&env);
    client.create_stream(&recipient, &1_000, &0, &100);
    set_time(&env, 50);
    client.withdraw(&admin);
}
