//! Authorization-code flow with PKCE. Secret values never implement Debug.
use crate::auth::{AuthError, Http, Session, Transport};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use url::Url;
pub const REDIRECT: &str = "https://login.microsoftonline.com/common/oauth2/nativeclient";
pub struct Pending {
    verifier: String,
    state: String,
}
pub enum Callback {
    Ignore,
    Code(String),
    Error(AuthError),
}
impl Pending {
    pub fn new() -> Result<Self, AuthError> {
        let mut verifier = [0u8; 32];
        let mut state = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut verifier)
            .map_err(|_| AuthError::InvalidResponse)?;
        OsRng
            .try_fill_bytes(&mut state)
            .map_err(|_| AuthError::InvalidResponse)?;
        Ok(Self {
            verifier: URL_SAFE_NO_PAD.encode(verifier),
            state: URL_SAFE_NO_PAD.encode(state),
        })
    }
    pub fn authorize_url(&self) -> String {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(self.verifier.as_bytes()));
        let mut url =
            Url::parse("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")
                .expect("constant URL");
        url.query_pairs_mut().extend_pairs([
            ("client_id", crate::auth::CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", REDIRECT),
            ("scope", "XboxLive.signin offline_access"),
            ("response_mode", "query"),
            ("code_challenge_method", "S256"),
            ("code_challenge", &challenge),
            ("state", &self.state),
            ("prompt", "select_account"),
        ]);
        url.into()
    }
    pub fn callback(&self, raw: &str) -> Callback {
        let Ok(url) = Url::parse(raw) else {
            return Callback::Ignore;
        };
        if url.scheme() != "https"
            || url.host_str() != Some("login.microsoftonline.com")
            || url.port_or_known_default() != Some(443)
            || url.path() != "/common/oauth2/nativeclient"
        {
            return Callback::Ignore;
        }
        if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
            return Callback::Error(AuthError::InvalidResponse);
        }
        let fields: Vec<_> = url.query_pairs().collect();
        let states: Vec<_> = fields.iter().filter(|(k, _)| k == "state").collect();
        if states.len() != 1 || states[0].1 != self.state {
            return Callback::Error(AuthError::Rejected);
        }
        if fields.iter().any(|(k, _)| k == "error") {
            return Callback::Error(AuthError::Declined);
        }
        let codes: Vec<_> = fields.iter().filter(|(k, _)| k == "code").collect();
        if codes.len() != 1 || codes[0].1.is_empty() || codes[0].1.len() > 16384 {
            return Callback::Error(AuthError::InvalidResponse);
        }
        Callback::Code(codes[0].1.to_string())
    }
    pub fn exchange(self, http: &Http, code: &str) -> Result<Session, AuthError> {
        let r = http.form(
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/token",
            &[
                ("client_id", crate::auth::CLIENT_ID),
                ("grant_type", "authorization_code"),
                ("redirect_uri", REDIRECT),
                ("code", code),
                ("code_verifier", &self.verifier),
            ],
        )?;
        crate::auth::finish_authorization_code(http, r)
    }
}
pub fn allowed_navigation(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "login.microsoftonline.com"
                    | "login.live.com"
                    | "login.microsoft.com"
                    | "account.live.com"
                    | "account.microsoft.com"
                    | "signup.live.com"
            )
        )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pending() -> Pending {
        Pending {
            verifier: "v".repeat(43),
            state: "expected".into(),
        }
    }
    #[test]
    fn pkce_s256_and_state() {
        let p = pending();
        let u = Url::parse(&p.authorize_url()).unwrap();
        let q: std::collections::HashMap<_, _> = u.query_pairs().collect();
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(
            q["code_challenge"],
            URL_SAFE_NO_PAD.encode(Sha256::digest(p.verifier.as_bytes()))
        );
        assert_eq!(q["state"], "expected");
        assert_eq!(q["redirect_uri"], REDIRECT);
    }
    #[test]
    fn valid_callback() {
        assert!(
            matches!(pending().callback(&format!("{REDIRECT}?state=expected&code=test")),Callback::Code(c) if c=="test")
        );
    }
    #[test]
    fn rejects_bad_state_duplicate_and_error() {
        for q in [
            "state=bad&code=x",
            "state=expected&state=expected&code=x",
            "state=expected&code=x&code=y",
            "state=expected&error=access_denied",
            "code=x",
        ] {
            assert!(matches!(
                pending().callback(&format!("{REDIRECT}?{q}")),
                Callback::Error(_)
            ));
        }
    }
    #[test]
    fn exact_redirect_only() {
        for u in [
            "https://attacker.invalid/common/oauth2/nativeclient?code=x",
            "https://login.microsoftonline.com.evil.invalid/common/oauth2/nativeclient",
            "http://login.microsoftonline.com/common/oauth2/nativeclient",
            "https://login.microsoftonline.com/common/oauth2/nativeclient/extra",
        ] {
            assert!(matches!(pending().callback(u), Callback::Ignore));
        }
    }
    #[test]
    fn allowlist_blocks_other_sites() {
        assert!(allowed_navigation(
            "https://login.live.com/oauth20_authorize.srf"
        ));
        for u in [
            "http://login.live.com",
            "file:///etc/passwd",
            "https://login.live.com.evil.invalid",
            "https://user@login.live.com",
            "https://login.live.com:444/",
        ] {
            assert!(!allowed_navigation(u));
        }
    }
    #[test]
    fn fresh_random_values() {
        let a = Pending::new().unwrap();
        let b = Pending::new().unwrap();
        assert_ne!(a.state, b.state);
        assert_ne!(a.verifier, b.verifier);
        assert_eq!(a.verifier.len(), 43);
    }
}
