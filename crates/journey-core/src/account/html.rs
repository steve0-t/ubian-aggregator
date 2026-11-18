use super::*;
use scraper::{Html, Selector};

pub(super) struct LoginForm {
    pub hidden: Vec<(String, String)>,
}
fn selector(value: &str) -> Result<Selector> {
    Selector::parse(value).map_err(|_| AccountError::ProviderChanged)
}
fn document(bytes: &[u8]) -> Result<Html> {
    if bytes.len() > 1024 * 1024 {
        return Err(AccountError::ProviderChanged);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| AccountError::ProviderChanged)?;
    if text.contains('\0') {
        return Err(AccountError::ProviderChanged);
    }
    Ok(Html::parse_document(text))
}
pub(super) fn login_form(bytes: &[u8]) -> Result<LoginForm> {
    let doc = document(bytes)?;
    if challenge(&doc)? {
        return Err(AccountError::ChallengeRequired);
    }
    let forms = doc
        .select(&selector("form#login-form")?)
        .collect::<Vec<_>>();
    if forms.len() != 1 {
        return Err(AccountError::ProviderChanged);
    }
    let form = forms[0];
    let value = form.value();
    if !value
        .attr("method")
        .is_some_and(|method| method.eq_ignore_ascii_case("post"))
        || !matches!(
            value.attr("action"),
            Some("/login" | "https://www.ubian.sk/login")
        )
    {
        return Err(AccountError::ProviderChanged);
    }
    let email = form
        .select(&selector("input[name=email]")?)
        .collect::<Vec<_>>();
    let password = form
        .select(&selector("input[name=password][type=password]")?)
        .collect::<Vec<_>>();
    if email.len() != 1 || password.len() != 1 {
        return Err(AccountError::ProviderChanged);
    }
    let mut hidden = Vec::<(String, String)>::new();
    for input in form.select(&selector("input[type=hidden]")?) {
        let name = input
            .value()
            .attr("name")
            .ok_or(AccountError::ProviderChanged)?;
        let value = input.value().attr("value").unwrap_or("");
        if !valid_text(name, 128)
            || value.len() > 16 * 1024
            || value.chars().any(char::is_control)
            || matches!(name, "email" | "password" | "rememberme")
            || hidden.iter().any(|(key, _)| key == name)
            || hidden.len() >= 16
        {
            return Err(AccountError::ProviderChanged);
        }
        hidden.push((name.into(), value.into()));
    }
    Ok(LoginForm { hidden })
}
fn challenge(doc: &Html) -> Result<bool> {
    Ok(doc
        .select(&selector(
            ".g-recaptcha,.h-captcha,#challenge-form,iframe[src*='captcha']",
        )?)
        .next()
        .is_some())
}
pub(super) fn login_body(form: LoginForm, request: &LoginRequest) -> Zeroizing<String> {
    let mut encoded = url::form_urlencoded::Serializer::new(String::new());
    for (name, value) in form.hidden {
        encoded.append_pair(&name, &value);
    }
    encoded
        .append_pair("email", &request.email)
        .append_pair("password", &request.password);
    if request.remember_session {
        encoded.append_pair("rememberme", "on");
    }
    Zeroizing::new(encoded.finish())
}

/// The authorized account probe verified this protected document and input
/// schema. Only login identity and optional display name are retained; contact
/// email, phone, password-reset controls and provider scripts are discarded.
pub(super) fn profile(bytes: &[u8]) -> Result<AccountProfile> {
    let doc = document(bytes)?;
    if challenge(&doc)? {
        return Err(AccountError::ChallengeRequired);
    }
    let forms = doc
        .select(&selector("form#js-profile-form")?)
        .collect::<Vec<_>>();
    if forms.len() != 1 {
        return Err(AccountError::ProviderChanged);
    }
    let form = forms[0];
    if !form
        .value()
        .attr("method")
        .is_some_and(|value| value.eq_ignore_ascii_case("post"))
        || form
            .value()
            .attr("action")
            .is_some_and(|value| !value.is_empty())
    {
        return Err(AccountError::ProviderChanged);
    }
    let input = |selection: &str, max: usize, allow_empty: bool| -> Result<String> {
        let matched = form.select(&selector(selection)?).collect::<Vec<_>>();
        if matched.len() != 1 {
            return Err(AccountError::ProviderChanged);
        }
        let value = matched[0]
            .value()
            .attr("value")
            .ok_or(AccountError::ProviderChanged)?;
        if (!allow_empty && !valid_text(value, max))
            || value.chars().count() > max
            || value.chars().any(char::is_control)
        {
            return Err(AccountError::ProviderChanged);
        }
        Ok(value.into())
    };
    let email = input("input#profile-login[name=login][type=text]", 254, false)?;
    if email.trim() != email || !email.contains('@') || email.chars().any(char::is_whitespace) {
        return Err(AccountError::ProviderChanged);
    }
    // A contact email may differ from the login identity. Require its observed
    // field for schema recognition but never replace the login with it.
    input("input#profile-email[name=email]", 254, true)?;
    let name = input("input#profile-name[name=name]", 128, true)?;
    let surname = input("input#profile-surname[name=surname]", 128, true)?;
    let display = [name.trim(), surname.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if display.chars().count() > 256 {
        return Err(AccountError::ProviderChanged);
    }
    Ok(AccountProfile {
        email,
        display_name: (!display.is_empty()).then_some(display),
    })
}
pub(super) fn login_error(bytes: &[u8]) -> Result<AccountError> {
    let doc = document(bytes)?;
    if challenge(&doc)? {
        return Ok(AccountError::ChallengeRequired);
    }
    // The authorized provider probe returned this inline invocation after an
    // incorrect password. Its argument echoes email/password: never parse,
    // execute, render, serialize or log those argument values.
    if doc.select(&selector("script:not([src])")?).any(|script| {
        script
            .text()
            .any(|text| text.contains("ubian.wrongPassword("))
    }) {
        return Ok(AccountError::InvalidCredentials);
    }
    Ok(AccountError::ProviderChanged)
}
pub(super) fn is_login(bytes: &[u8]) -> bool {
    login_form(bytes).is_ok()
}
