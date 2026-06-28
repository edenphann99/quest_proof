# quest_proof

## Project Title
quest_proof

## Project Description
`quest_proof` is a Soroban smart contract that anchors in-game quest completion on a public ledger. A *quest master* publishes a quest with a short identifier, a requirements code, and a reward tally. A *player* submits a 32-byte hash of their evidence (a screenshot, a replay, or a game log) as proof of completion. The master reviews the proof and the contract records a tamper-proof `verified` status, atomically incrementing the player's completed-quest counter. Unlike `delivery_proof`, which targets physical deliveries, `quest_proof` is purpose-built for digital gaming quests where evidence is a hash and the workflow is "issue -> submit -> verify".

## Project Vision
The long-term goal is to make quest completion a portable, verifiable, player-owned credential. Today, achievements live inside walled-garden game servers and can be revoked at will by the operator. By anchoring the master-player-submission flow on Stellar, we move quest records onto a public ledger that any third-party game, tournament, scholarship program, or community hub can independently read. We envision a future where a single Stellar address carries a player's quest history across titles and platforms, and where quest masters (guilds, esports organizers, professors, bootcamps) can publish quests with confidence that the proof-of-completion ledger cannot be silently rewritten or faked.

## Key Features
- `issue_quest` — Quest masters publish a quest with a unique `quest_id`, a `requirements` code, and an in-game `reward` tally. Master identity is enforced via `require_auth`, and duplicate `quest_id` values are rejected.
- `submit_proof` — Players attach a 32-byte `proof_hash` (e.g. SHA-256 of a screenshot or game log) to a quest. A player may refine an unverified submission, but cannot resubmit after the quest is closed.
- `verify_quest` — The original master reviews the submission and marks the quest verified, atomically incrementing the player's completed-quest counter. The master identity is re-checked against the stored quest record.
- `reject_proof` — The original master may close a submission as rejected when the evidence is insufficient. No reward is granted.
- `quest_status` — A read-only view of any `(quest_id, player)` pair, returning one of: not started (0), submitted (1), verified (2), rejected (3).
- `completed_count` — A read-only view of a player's total verified quests, useful for leaderboards, eligibility checks, and on-chain achievement badges.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** gaming dApp — see `contracts/quest_proof/src/lib.rs` for the full quest_proof business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CAQ4BY74UFESKOHP4O52WUCN2YVJ63RZBZEYUNGLGVMKGHOXFMG3MEWY`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/ec1fecd67d30beaca69b252eb2075ff4948e0815513595a148525c8650dfc915`

## Future Scope
- **Multi-master co-signing.** Let a quest be issued with a list of masters and require a threshold of approvals before a quest counts as verified, useful for guilds and tournaments.
- **Native-asset reward escrow.** Pair the in-game `reward` counter with a Stellar native asset (XLM or USDC) so the contract can lock funds on `issue_quest` and release them on `verify_quest`.
- **Zero-knowledge proof attachments.** Accept a ZK proof in `submit_proof` so a player can demonstrate they completed the quest without revealing the raw evidence hash.
- **Guild and team rosters.** Aggregate `completed_count` across a list of addresses to score a guild, clan, or class on a shared leaderboard.
- **Replay protection per version.** Bind `proof_hash` to the ledger sequence and the issuing `quest_id` so the same screenshot cannot be reused against a re-issued version of a quest.
- **Frontend dApp.** A small HTML/JS UI that talks to the deployed contract via Freighter, with a master view (issue / verify) and a player view (submit / check status).
- **Indexed event log.** Emit Soroban events on every state transition so off-chain indexers (and the future frontend) can subscribe to quest activity without polling storage.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `quest_proof` (gaming)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
