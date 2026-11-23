use std::fmt::{self, Debug, Formatter};

use domain::{APPDATA_SCOPE, AuthError, Identity};

use crate::gis::{self, Granted};
use crate::token::Tokens;

pub(crate) fn client_id() -> Option<&'static str> {
    option_env!("GOOGLE_CLIENT_ID").filter(|value| !value.is_empty())
}

#[derive(Clone)]
pub(crate) struct GoogleIdentity {
    tokens: Tokens,
}

impl GoogleIdentity {
    pub(crate) fn new(tokens: Tokens) -> Self {
        Self { tokens }
    }
}

impl Debug for GoogleIdentity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("GoogleIdentity")
    }
}

impl Identity for GoogleIdentity {
    fn remembered(&self) -> bool {
        self.tokens.remembered()
    }

    async fn sign_in(&mut self, scopes: Vec<String>) -> Result<(), AuthError> {
        let granted = request_token(&scopes.join(" ")).await?;
        self.tokens.store(granted.access_token, granted.expires_in);
        Ok(())
    }

    async fn ensure_access(&mut self) -> Result<(), AuthError> {
        if self.tokens.current().is_some() {
            return Ok(());
        }
        if !self.tokens.begin_prompt() {
            return Err(AuthError::NeedsConfirmation);
        }
        match request_token(APPDATA_SCOPE).await {
            Ok(granted) => {
                self.tokens.store(granted.access_token, granted.expires_in);
                Ok(())
            }
            Err(error) => {
                self.tokens.arm_prompt();
                Err(error)
            }
        }
    }

    fn sign_out(&mut self) {
        self.tokens.forget();
    }
}

async fn request_token(scope: &str) -> Result<Granted, AuthError> {
    let Some(client_id) = client_id() else {
        return Err(AuthError::Denied);
    };
    gis::request_access_token(client_id, scope).await
}
