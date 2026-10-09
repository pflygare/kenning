//! Sign in with Google (OpenID Connect, authorization code flow with PKCE).

use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest,
};
use tokio::sync::OnceCell;

use crate::config::GoogleConfig;

const ISSUER: &str = "https://accounts.google.com";

/// What Google tells us about the person after a successful sign-in.
#[derive(Debug)]
pub struct GoogleProfile {
    pub subject: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
}

/// The pieces of an in-flight sign-in we keep until Google redirects back.
pub struct Started {
    pub authorize_url: String,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

pub struct Google {
    config: GoogleConfig,
    redirect_url: RedirectUrl,
    http: reqwest::Client,
    metadata: OnceCell<CoreProviderMetadata>,
}

impl Google {
    pub fn new(config: GoogleConfig, redirect_url: String) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            redirect_url: RedirectUrl::new(redirect_url)?,
            // No redirects: following them during the token exchange is an SSRF risk.
            http: reqwest::ClientBuilder::new()
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            metadata: OnceCell::new(),
        })
    }

    async fn metadata(&self) -> anyhow::Result<&CoreProviderMetadata> {
        self.metadata
            .get_or_try_init(|| async {
                let issuer = IssuerUrl::new(ISSUER.to_string())?;
                Ok::<_, anyhow::Error>(
                    CoreProviderMetadata::discover_async(issuer, &self.http).await?,
                )
            })
            .await
    }

    pub async fn start(&self) -> anyhow::Result<Started> {
        let client = CoreClient::from_provider_metadata(
            self.metadata().await?.clone(),
            ClientId::new(self.config.client_id.clone()),
            Some(ClientSecret::new(self.config.client_secret.clone())),
        )
        .set_redirect_uri(self.redirect_url.clone());
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(challenge)
            .url();
        Ok(Started {
            authorize_url: url.to_string(),
            state: state.secret().clone(),
            nonce: nonce.secret().clone(),
            pkce_verifier: verifier.secret().clone(),
        })
    }

    /// Exchange the code Google sent back and verify the ID token.
    pub async fn finish(
        &self,
        code: String,
        nonce: String,
        pkce_verifier: String,
    ) -> anyhow::Result<GoogleProfile> {
        let client = CoreClient::from_provider_metadata(
            self.metadata().await?.clone(),
            ClientId::new(self.config.client_id.clone()),
            Some(ClientSecret::new(self.config.client_secret.clone())),
        )
        .set_redirect_uri(self.redirect_url.clone());
        let response = client
            .exchange_code(AuthorizationCode::new(code))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
            .request_async(&self.http)
            .await?;
        let id_token = response
            .id_token()
            .ok_or_else(|| anyhow::anyhow!("Google returned no ID token"))?;
        let claims = id_token.claims(&client.id_token_verifier(), &Nonce::new(nonce))?;

        if claims.email_verified() != Some(true) {
            anyhow::bail!("Google account email is not verified");
        }
        let email = claims
            .email()
            .ok_or_else(|| anyhow::anyhow!("Google returned no email"))?
            .to_string();
        let name = claims
            .name()
            .and_then(|name| name.get(None))
            .map(|name| name.to_string())
            .unwrap_or_else(|| email.split('@').next().unwrap_or_default().to_string());
        let picture = claims
            .picture()
            .and_then(|picture| picture.get(None))
            .map(|url| url.to_string());
        Ok(GoogleProfile {
            subject: claims.subject().to_string(),
            email,
            name,
            picture,
        })
    }
}
