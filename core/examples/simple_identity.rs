//! Simple Identity Example
//!
//! Demonstrates basic usage of ZapLivre Core identity module:
//! - Generating keypairs
//! - Creating identities with prekey pools
//! - Signing and verifying messages
//! - Getting prekey bundles for key exchange
//!
//! Run with: cargo run --example simple_identity

use zaplivre_core::identity::{Identity, Keypair};

fn main() {
    println!("=== ZapLivre Core - Identity Example ===\n");

    // 1. Generate a new identity with 100 prekeys
    println!("1. Generating identity with 100 prekeys...");
    let identity = Identity::generate(100);
    println!("   ✅ Identity generated!");
    println!("   Peer ID: {}", identity.peer_id());
    println!();

    // 2. Sign a message
    println!("2. Signing a message...");
    let message = b"Hello, ZapLivre!";
    let signature = identity.keypair().sign(message);
    println!("   ✅ Message signed!");
    println!("   Signature: {}...", hex::encode(&signature[..16]));
    println!();

    // 3. Verify the signature
    println!("3. Verifying signature...");
    let public_key = identity.keypair().public_key();
    match public_key.verify(message, &signature) {
        Ok(()) => println!("   ✅ Signature is valid!"),
        Err(e) => println!("   ❌ Signature verification failed: {}", e),
    }
    println!();

    // 4. Get prekey bundle for key exchange
    println!("4. Getting prekey bundle...");
    let mut identity_mut = identity.clone();
    if let Some(pool) = identity_mut.prekey_pool_mut() {
        let bundle = pool.get_bundle().expect("get bundle");
        println!("   ✅ Prekey bundle created!");
        println!("   Signed prekey ID: {}", bundle.signed_prekey_id);
        println!(
            "   One-time prekey: {}",
            if bundle.one_time_prekey.is_some() {
                "Yes"
            } else {
                "No"
            }
        );
        println!("   Remaining prekeys: {}", pool.prekey_count());
    }
    println!();

    // 5. Demonstrate two peers performing key exchange
    println!("5. Demonstrating key exchange between two peers...");

    // Alice generates identity
    let alice = Identity::generate(10);
    println!("   Alice peer ID: {}", alice.peer_id());

    // Bob generates identity
    let bob = Identity::generate(10);
    println!("   Bob peer ID: {}", bob.peer_id());

    // Alice gets Bob's prekey bundle
    let mut bob_mut = bob.clone();
    if let Some(bob_pool) = bob_mut.prekey_pool_mut() {
        let _bob_bundle = bob_pool.get_bundle().expect("get bundle");

        // Alice can now use Bob's bundle to establish a shared secret
        // (This would be done via Signal Protocol X3DH in crypto module)
        println!("   ✅ Alice received Bob's prekey bundle");
        println!("   ✅ Key exchange ready (X3DH would be performed in crypto module)");
    }
    println!();

    // 6. Generate standalone keypair
    println!("6. Generating standalone keypair...");
    let keypair = Keypair::generate();
    let peer_id = keypair.peer_id();
    println!("   ✅ Keypair generated!");
    println!("   Peer ID: {}", peer_id);
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("Next steps:");
    println!("  - Implement Signal Protocol (crypto module)");
    println!("  - Implement P2P networking (network module)");
    println!("  - Implement local storage (storage module)");
}
