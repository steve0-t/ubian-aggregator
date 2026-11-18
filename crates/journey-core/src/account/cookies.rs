use super::*;
use cookie_store::{CookieDomain, CookieStore};
use reqwest::{cookie::CookieStore as ReqwestCookieStore, header::HeaderValue};
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

const MAX_COOKIES: usize = 64;
const MAX_COOKIE: usize = 8192;
pub(super) const MAX_SESSION: usize = 128 * 1024;

#[derive(Default)]
pub(super) struct SessionJar {
    store: Mutex<CookieStore>,
    invalid: AtomicBool,
}
impl SessionJar {
    pub(super) fn valid(&self) -> Result<()> {
        if self.invalid.load(Ordering::Acquire) {
            Err(AccountError::ProviderChanged)
        } else {
            Ok(())
        }
    }
    pub(super) fn export(&self) -> Result<serde_json::Value> {
        self.valid()?;
        let store = self
            .store
            .lock()
            .map_err(|_| AccountError::ProviderUnavailable)?;
        // Preserve HttpOnly, Secure, SameSite, domain/host-only, path and expiry.
        // User opt-in may deliberately retain provider session cookies too.
        let cookies = store.iter_unexpired().collect::<Vec<_>>();
        serde_json::to_value(cookies).map_err(|_| AccountError::PersistenceInvalid)
    }
    pub(super) fn restore(value: serde_json::Value) -> Result<Self> {
        let serialized = Zeroizing::new(
            serde_json::to_vec(&value).map_err(|_| AccountError::PersistenceInvalid)?,
        );
        if serialized.len() > MAX_SESSION {
            return Err(AccountError::PersistenceInvalid);
        }
        let store = cookie_store::serde::json::load(serialized.as_slice())
            .map_err(|_| AccountError::PersistenceInvalid)?;
        let origin =
            Url::parse("https://www.ubian.sk/").map_err(|_| AccountError::PersistenceInvalid)?;
        let cookies = store.iter_any().collect::<Vec<_>>();
        if cookies.is_empty()||cookies.len()>MAX_COOKIES||cookies.iter().any(|cookie|{
            !cookie.domain.matches(&origin)||!matches!(&cookie.domain,
                CookieDomain::HostOnly(host) if host=="www.ubian.sk")
                && !matches!(&cookie.domain,CookieDomain::Suffix(host) if host=="www.ubian.sk"||host=="ubian.sk")
                ||cookie.to_string().len()>MAX_COOKIE
        }){return Err(AccountError::PersistenceInvalid);}
        Ok(Self {
            store: Mutex::new(store),
            invalid: AtomicBool::new(false),
        })
    }
}
fn exact_origin(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("www.ubian.sk")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}
impl ReqwestCookieStore for SessionJar {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        if !exact_origin(url) {
            return;
        }
        let Ok(mut store) = self.store.lock() else {
            self.invalid.store(true, Ordering::Release);
            return;
        };
        for header in headers {
            let Ok(raw) = header.to_str() else {
                self.invalid.store(true, Ordering::Release);
                continue;
            };
            if raw.len() > MAX_COOKIE {
                self.invalid.store(true, Ordering::Release);
                continue;
            }
            // Calling parse directly avoids cookie_store's optional diagnostics
            // that can print non-Secure cookie values in debug logging.
            match store.parse(raw, url) {
                Ok(_) | Err(cookie_store::CookieError::Expired) => {}
                Err(_) => self.invalid.store(true, Ordering::Release),
            }
        }
        if store.iter_any().count() > MAX_COOKIES {
            self.invalid.store(true, Ordering::Release);
        }
    }
    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        if !exact_origin(url) || self.invalid.load(Ordering::Acquire) {
            return None;
        }
        let store = self.store.lock().ok()?;
        let value = Zeroizing::new(
            store
                .get_request_values(url)
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; "),
        );
        if value.is_empty() || value.len() > 64 * 1024 {
            return None;
        }
        HeaderValue::from_str(&value).ok().map(|mut value| {
            value.set_sensitive(true);
            value
        })
    }
}
