use rand::RngCore;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::faker_rs::catalog_gen;
use crate::faker_rs::locales::LocaleKey;
use crate::generators::GeneratedValue;

const DEFAULT_LOCALE: LocaleKey = LocaleKey::EnUs;
const SUPPORTED_LOCALES: &[LocaleKey] = &[LocaleKey::EnUs, LocaleKey::PtBr];

pub struct FakeRsAdapter;

impl FakeRsAdapter {
    pub fn list_ids() -> &'static [&'static str] {
        catalog_gen::ALL_IDS
    }

    pub fn validate(
        id: &str,
        locale: Option<&str>,
        params: Option<&Value>,
    ) -> Result<(), GenerationError> {
        Self::resolve(id, locale, params).map(|_| ())
    }

    pub fn generate_value(
        id: &str,
        locale: Option<&str>,
        params: Option<&Value>,
        rng: &mut dyn RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let resolved = Self::resolve(id, locale, params)?;
        let value =
            catalog_gen::generate_value(resolved.id, resolved.locale, rng).ok_or_else(|| {
                GenerationError::InvalidPlan(format!(
                    "unsupported faker id '{}' for locale '{}'",
                    resolved.id,
                    resolved.locale.as_str()
                ))
            })?;
        Ok(value)
    }
}

struct ResolvedFaker<'a> {
    id: &'a str,
    locale: LocaleKey,
}

impl FakeRsAdapter {
    fn resolve<'a>(
        id: &'a str,
        locale: Option<&'a str>,
        params: Option<&Value>,
    ) -> Result<ResolvedFaker<'a>, GenerationError> {
        if !catalog_gen::ALL_IDS.contains(&id) {
            return Err(GenerationError::InvalidPlan(format!(
                "unsupported faker id '{}'",
                id
            )));
        }

        let locale_str = locale.unwrap_or_else(|| DEFAULT_LOCALE.as_str());
        let locale_key = LocaleKey::parse(locale_str).ok_or_else(|| {
            GenerationError::InvalidPlan(format!("unsupported faker locale '{}'", locale_str))
        })?;

        let (resolved_id, allowed_locales) = if let Some(alias) = catalog_gen::alias_entry(id) {
            let allowed = if alias.locales.is_empty() {
                SUPPORTED_LOCALES
            } else {
                alias.locales
            };
            (alias.target, allowed)
        } else {
            (id, SUPPORTED_LOCALES)
        };

        if !catalog_gen::GENERATED_IDS.contains(&resolved_id) {
            return Err(GenerationError::InvalidPlan(format!(
                "unsupported faker id '{}'",
                resolved_id
            )));
        }

        if !allowed_locales.contains(&locale_key) {
            return Err(GenerationError::InvalidPlan(format!(
                "unsupported faker locale '{}' for '{}'",
                locale_str, id
            )));
        }

        if catalog_gen::PARAMETERIZED_IDS.contains(&resolved_id) {
            return Err(GenerationError::InvalidPlan(format!(
                "faker id '{}' requires params (not supported yet)",
                id
            )));
        }

        match params {
            None => Ok(ResolvedFaker {
                id: resolved_id,
                locale: locale_key,
            }),
            Some(Value::Object(map)) if map.is_empty() => Ok(ResolvedFaker {
                id: resolved_id,
                locale: locale_key,
            }),
            Some(Value::Object(_)) => Err(GenerationError::InvalidPlan(format!(
                "params not supported for faker id '{}'",
                id
            ))),
            Some(_) => Err(GenerationError::InvalidPlan(format!(
                "params for faker id '{}' must be a JSON object",
                id
            ))),
        }
    }
}
