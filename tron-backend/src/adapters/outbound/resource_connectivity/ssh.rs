use std::{sync::Arc, time::Duration};

use russh::{
    Disconnect, client,
    keys::{PrivateKeyWithHashAlg, decode_secret_key, ssh_key},
};

use crate::application::resource::{SshAuthenticationTarget, SshConnectionTarget};

struct AcceptServerKey;

impl client::Handler for AcceptServerKey {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub(super) async fn verify(target: SshConnectionTarget) -> Result<(), ()> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    });
    let address = (target.endpoint.host.as_str(), target.endpoint.port);
    let mut session = client::connect(config, address, AcceptServerKey)
        .await
        .map_err(|_| ())?;
    let authenticated = match target.authentication {
        SshAuthenticationTarget::Password { password } => session
            .authenticate_password(target.username, password)
            .await
            .map_err(|_| ())?
            .success(),
        SshAuthenticationTarget::PrivateKey {
            private_key,
            passphrase,
        } => {
            let private_key =
                decode_secret_key(&private_key, passphrase.as_deref()).map_err(|_| ())?;
            let hash = session
                .best_supported_rsa_hash()
                .await
                .map_err(|_| ())?
                .flatten();
            session
                .authenticate_publickey(
                    target.username,
                    PrivateKeyWithHashAlg::new(Arc::new(private_key), hash),
                )
                .await
                .map_err(|_| ())?
                .success()
        }
    };
    let _ = session
        .disconnect(Disconnect::ByApplication, "connectivity verified", "en")
        .await;
    authenticated.then_some(()).ok_or(())
}
