//! Registra um username num identity server, com uma identidade nova e um
//! bundle válido (inclui o vínculo da identidade Signal). Usado pelo E2E
//! Android para semear o contato que os flows procuram.
//!
//! ```text
//! cargo run -p zaplivre-core --example seed_username -- http://localhost:8083 maestro_e2e_peer
//! ```

use zaplivre_core::{identity::Identity, identity_client::IdentityClient};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(url), Some(username)) = (args.next(), args.next()) else {
        eprintln!("uso: seed_username <identity-server-url> <username>");
        std::process::exit(2);
    };

    let identity = Identity::generate(10);
    let peer_id = identity
        .keypair()
        .libp2p_peer_id()
        .expect("peer id da identidade gerada");

    let client = IdentityClient::new(url).expect("URL do identity server");
    match client
        .register_username(&identity, &username, &peer_id)
        .await
    {
        Ok(response) => println!("registrado @{} -> {}", response.username, response.peer_id),
        Err(e) => {
            eprintln!("falha ao registrar @{}: {}", username, e);
            std::process::exit(1);
        }
    }
}
