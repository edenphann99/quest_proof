#![no_std]

//! # quest_proof
//!
//! A Soroban smart contract that anchors in-game quest completion on-chain.
//! A *quest master* publishes a quest with a short identifier, a
//! requirements code, and a reward tally. A *player* submits a 32-byte
//! hash of their evidence (screenshot, replay, game log). The master
//! reviews the proof and marks the quest verified; the contract
//! atomically increments the player's completed-quest counter.
//!
//! Unlike `delivery_proof` (which targets physical deliveries),
//! `quest_proof` is purpose-built for digital gaming quests.

use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Symbol};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Namespace for all persistent keys written by this contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// A published quest, keyed by its `quest_id`.
    Quest(Symbol),
    /// A player's submission against a specific quest.
    Submission(Symbol, Address),
    /// A player's running counter of verified quests.
    Completed(Address),
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

/// Static description of a quest as published by its master.
#[contracttype]
#[derive(Clone)]
pub struct Quest {
    /// Address of the master who issued the quest and may verify proofs.
    pub master: Address,
    /// Short code describing what the player must do (e.g. `"defeat_boss"`).
    pub requirements: Symbol,
    /// In-game reward tally granted to a player on successful verification.
    /// This is a bookkeeping counter only; no native asset is moved.
    pub reward: u32,
    /// Ledger timestamp at which the quest was issued.
    pub issued_at: u64,
}

/// A player's submission against a single quest.
#[contracttype]
#[derive(Clone)]
pub struct Submission {
    /// 32-byte hash of the player's evidence (SHA-256 of screenshot / log).
    pub proof_hash: BytesN<32>,
    /// Submission status, see the `STATUS_*` constants below.
    pub status: u32,
    /// Ledger timestamp at which the proof was submitted.
    pub submitted_at: u64,
}

// ---------------------------------------------------------------------------
// Status codes (also returned by `quest_status`)
// ---------------------------------------------------------------------------

/// Player has not submitted anything for this quest yet.
pub const STATUS_NOT_STARTED: u32 = 0;
/// Player has submitted a proof that is awaiting the master's verdict.
pub const STATUS_SUBMITTED: u32 = 1;
/// Master has verified the proof; the quest counts as complete.
pub const STATUS_VERIFIED: u32 = 2;
/// Master has rejected the proof; the quest is closed for this player.
pub const STATUS_REJECTED: u32 = 3;

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

/// Soroban contract that records on-chain quest-completion proofs.
#[contract]
pub struct QuestProof;

#[contractimpl]
impl QuestProof {
    // -----------------------------------------------------------------------
    // write paths
    // -----------------------------------------------------------------------

    /// Issue a new quest. Only the quest master may issue a quest with the
    /// given `quest_id`. The `requirements` symbol describes what the
    /// player must do, and `reward` is the in-game tally (no XLM is moved).
    pub fn issue_quest(
        env: Env,
        master: Address,
        quest_id: Symbol,
        requirements: Symbol,
        reward: u32,
    ) {
        // Authorization: only the master may publish under their identity.
        master.require_auth();

        let key = DataKey::Quest(quest_id.clone());
        if env.storage().instance().has(&key) {
            panic!("quest already issued");
        }

        let quest = Quest {
            master,
            requirements,
            reward,
            issued_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(&key, &quest);
    }

    /// Player submits a 32-byte `proof_hash` of their evidence for
    /// `quest_id`. A player may refine an unverified submission, but
    /// cannot resubmit once the quest is verified.
    pub fn submit_proof(
        env: Env,
        player: Address,
        quest_id: Symbol,
        proof_hash: BytesN<32>,
    ) {
        // Authorization: only the player can attach evidence to their record.
        player.require_auth();

        // The quest must exist.
        let quest_key = DataKey::Quest(quest_id.clone());
        if !env.storage().instance().has(&quest_key) {
            panic!("quest not found");
        }

        // Block re-submission after the quest is closed for this player.
        let sub_key = DataKey::Submission(quest_id.clone(), player.clone());
        if let Some(existing) = env
            .storage()
            .instance()
            .get::<DataKey, Submission>(&sub_key)
        {
            if existing.status == STATUS_VERIFIED {
                panic!("quest already completed");
            }
        }

        let submission = Submission {
            proof_hash,
            status: STATUS_SUBMITTED,
            submitted_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(&sub_key, &submission);
    }

    /// Quest master reviews the player's submitted proof and marks the
    /// quest as verified. The player's completed-quest counter is
    /// incremented exactly once per verified quest. Returns the player's
    /// new completed count.
    pub fn verify_quest(
        env: Env,
        master: Address,
        quest_id: Symbol,
        player: Address,
    ) -> u32 {
        // Authorization: only the master signs the verification.
        master.require_auth();

        // The quest must exist and the caller must be its master.
        let quest_key = DataKey::Quest(quest_id.clone());
        let quest: Quest = env
            .storage()
            .instance()
            .get(&quest_key)
            .expect("quest not found");
        if quest.master != master {
            panic!("only the quest master can verify");
        }

        // The player must have a submission that is still pending.
        let sub_key = DataKey::Submission(quest_id.clone(), player.clone());
        let mut submission: Submission = env
            .storage()
            .instance()
            .get(&sub_key)
            .expect("no submission for this player");
        if submission.status == STATUS_VERIFIED {
            panic!("quest already verified");
        }
        if submission.status != STATUS_SUBMITTED {
            panic!("submission is not awaiting verification");
        }

        submission.status = STATUS_VERIFIED;
        env.storage().instance().set(&sub_key, &submission);

        // Bump the player's running completed-quest counter.
        let count_key = DataKey::Completed(player.clone());
        let prev: u32 = env
            .storage()
            .instance()
            .get(&count_key)
            .unwrap_or(0u32);
        let next = prev
            .checked_add(1)
            .expect("completed-quest counter overflow");
        env.storage().instance().set(&count_key, &next);

        next
    }

    /// Quest master rejects a player's submitted proof. The quest is
    /// closed for that player (they cannot resubmit), but no completion
    /// counter is incremented.
    pub fn reject_proof(
        env: Env,
        master: Address,
        quest_id: Symbol,
        player: Address,
    ) {
        master.require_auth();

        let quest_key = DataKey::Quest(quest_id.clone());
        let quest: Quest = env
            .storage()
            .instance()
            .get(&quest_key)
            .expect("quest not found");
        if quest.master != master {
            panic!("only the quest master can reject");
        }

        let sub_key = DataKey::Submission(quest_id, player);
        let mut submission: Submission = env
            .storage()
            .instance()
            .get(&sub_key)
            .expect("no submission for this player");
        if submission.status == STATUS_VERIFIED {
            panic!("quest already verified");
        }
        submission.status = STATUS_REJECTED;
        env.storage().instance().set(&sub_key, &submission);
    }

    // -----------------------------------------------------------------------
    // read paths
    // -----------------------------------------------------------------------

    /// Returns the submission status for a `(quest_id, player)` pair:
    /// `0` = not started, `1` = submitted, `2` = verified, `3` = rejected.
    pub fn quest_status(env: Env, quest_id: Symbol, player: Address) -> u32 {
        let sub_key = DataKey::Submission(quest_id, player);
        env.storage()
            .instance()
            .get::<DataKey, Submission>(&sub_key)
            .map(|s| s.status)
            .unwrap_or(STATUS_NOT_STARTED)
    }

    /// Returns the number of distinct quests the player has had verified.
    /// Useful for leaderboards, eligibility checks, and achievement badges.
    pub fn completed_count(env: Env, player: Address) -> u32 {
        let key = DataKey::Completed(player);
        env.storage()
            .instance()
            .get::<DataKey, u32>(&key)
            .unwrap_or(0u32)
    }

    /// Returns the in-game reward tally for a given quest. Returns `0` if
    /// the quest does not exist.
    pub fn quest_reward(env: Env, quest_id: Symbol) -> u32 {
        let key = DataKey::Quest(quest_id);
        env.storage()
            .instance()
            .get::<DataKey, Quest>(&key)
            .map(|q| q.reward)
            .unwrap_or(0u32)
    }
}
