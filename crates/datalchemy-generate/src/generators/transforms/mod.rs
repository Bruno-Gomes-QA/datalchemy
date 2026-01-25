use rand::Rng;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, GeneratorRegistry, Transform, TransformContext};

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_transform(Box::new(NullRateTransform));
    registry.register_transform(Box::new(TruncateTransform));
    registry.register_transform(Box::new(FormatTransform));
    registry.register_transform(Box::new(PrefixSuffixTransform));
    registry.register_transform(Box::new(CasingTransform));
    registry.register_transform(Box::new(WeightedChoiceTransform));
    registry.register_transform(Box::new(MaskTransform));
}

struct NullRateTransform;

impl Transform for NullRateTransform {
    fn id(&self) -> &'static str {
        "transform.null_rate"
    }

    fn apply(
        &self,
        input: GeneratedValue,
        ctx: &TransformContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        if matches!(input, GeneratedValue::Null) {
            return Ok(input);
        }
        let rate = params
            .and_then(|params| params.get("rate"))
            .and_then(|value| value.as_f64())
            .ok_or_else(|| {
                GenerationError::InvalidPlan("transform.null_rate requires params.rate".to_string())
            })?;
        if !(0.0..=1.0).contains(&rate) {
            return Err(GenerationError::InvalidPlan(
                "transform.null_rate rate must be between 0 and 1".to_string(),
            ));
        }
        if !ctx.column.is_nullable && rate > 0.0 {
            return Err(GenerationError::InvalidPlan(
                "transform.null_rate rate cannot be > 0 for NOT NULL column".to_string(),
            ));
        }
        if rng.gen_bool(rate) {
            Ok(GeneratedValue::Null)
        } else {
            Ok(input)
        }
    }
}

struct TruncateTransform;

impl Transform for TruncateTransform {
    fn id(&self) -> &'static str {
        "transform.truncate"
    }

    fn apply(
        &self,
        input: GeneratedValue,
        _ctx: &TransformContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let max_len = params
            .and_then(|params| params.get("max_len"))
            .and_then(|value| value.as_u64())
            .ok_or_else(|| {
                GenerationError::InvalidPlan("transform.truncate requires max_len".to_string())
            })?;
        let max_len = usize::try_from(max_len).map_err(|_| {
            GenerationError::InvalidPlan("transform.truncate max_len invalid".to_string())
        })?;

        match input {
            GeneratedValue::Text(mut value) => {
                value.truncate(max_len);
                Ok(GeneratedValue::Text(value))
            }
            GeneratedValue::Uuid(mut value) => {
                value.truncate(max_len);
                Ok(GeneratedValue::Uuid(value))
            }
            GeneratedValue::Null => Ok(input),
            other => Err(GenerationError::InvalidPlan(format!(
                "transform.truncate not supported for {}",
                value_kind(&other)
            ))),
        }
    }
}

struct FormatTransform;

impl Transform for FormatTransform {
    fn id(&self) -> &'static str {
        "transform.format"
    }

    fn apply(
        &self,
        input: GeneratedValue,
        _ctx: &TransformContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        if matches!(input, GeneratedValue::Null) {
            return Ok(input);
        }
        let template = params
            .and_then(|params| params.get("template"))
            .and_then(|value| value.as_str());
        let format = params
            .and_then(|params| params.get("format"))
            .and_then(|value| value.as_str());

        if let Some(format) = format {
            let formatted = match input {
                GeneratedValue::Date(value) => value.format(format).to_string(),
                GeneratedValue::Time(value) => value.format(format).to_string(),
                GeneratedValue::Timestamp(value) => value.format(format).to_string(),
                _ => {
                    return Err(GenerationError::InvalidPlan(
                        "transform.format format only supports date/time/timestamp".to_string(),
                    ));
                }
            };
            return Ok(GeneratedValue::Text(formatted));
        }

        if let Some(template) = template {
            let value = value_to_string(&input);
            let formatted = template.replace("{value}", &value);
            return Ok(GeneratedValue::Text(formatted));
        }

        Err(GenerationError::InvalidPlan(
            "transform.format requires template or format".to_string(),
        ))
    }
}

struct PrefixSuffixTransform;

impl Transform for PrefixSuffixTransform {
    fn id(&self) -> &'static str {
        "transform.prefix_suffix"
    }

    fn apply(
        &self,
        input: GeneratedValue,
        _ctx: &TransformContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        if matches!(input, GeneratedValue::Null) {
            return Ok(input);
        }
        let prefix = params
            .and_then(|params| params.get("prefix"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let suffix = params
            .and_then(|params| params.get("suffix"))
            .and_then(|value| value.as_str())
            .unwrap_or("");

        let value = match input {
            GeneratedValue::Text(value) => value,
            GeneratedValue::Uuid(value) => value,
            _ => {
                return Err(GenerationError::InvalidPlan(
                    "transform.prefix_suffix supports text values only".to_string(),
                ));
            }
        };

        Ok(GeneratedValue::Text(format!("{prefix}{value}{suffix}")))
    }
}

struct CasingTransform;

impl Transform for CasingTransform {
    fn id(&self) -> &'static str {
        "transform.casing"
    }

    fn apply(
        &self,
        input: GeneratedValue,
        _ctx: &TransformContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        if matches!(input, GeneratedValue::Null) {
            return Ok(input);
        }
        let mode = params
            .and_then(|params| params.get("mode"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                GenerationError::InvalidPlan("transform.casing requires mode".to_string())
            })?;

        let value = match input {
            GeneratedValue::Text(value) => value,
            GeneratedValue::Uuid(value) => value,
            _ => {
                return Err(GenerationError::InvalidPlan(
                    "transform.casing supports text values only".to_string(),
                ));
            }
        };

        let transformed = match mode {
            "upper" => value.to_uppercase(),
            "lower" => value.to_lowercase(),
            "title" => to_title_case(&value),
            _ => {
                return Err(GenerationError::InvalidPlan(
                    "transform.casing mode must be upper, lower, or title".to_string(),
                ));
            }
        };
        Ok(GeneratedValue::Text(transformed))
    }
}

struct WeightedChoiceTransform;

impl Transform for WeightedChoiceTransform {
    fn id(&self) -> &'static str {
        "transform.weighted_choice"
    }

    fn apply(
        &self,
        input: GeneratedValue,
        _ctx: &TransformContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        if matches!(input, GeneratedValue::Null) {
            return Ok(input);
        }
        let choices = params
            .and_then(|params| params.get("choices"))
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                GenerationError::InvalidPlan(
                    "transform.weighted_choice requires choices array".to_string(),
                )
            })?;

        let mut total_weight = 0.0;
        let mut entries = Vec::new();
        for choice in choices {
            let value = choice
                .get("value")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    GenerationError::InvalidPlan(
                        "transform.weighted_choice choices require value".to_string(),
                    )
                })?;
            let weight = choice
                .get("weight")
                .and_then(|value| value.as_f64())
                .ok_or_else(|| {
                    GenerationError::InvalidPlan(
                        "transform.weighted_choice choices require weight".to_string(),
                    )
                })?;
            if weight <= 0.0 {
                return Err(GenerationError::InvalidPlan(
                    "transform.weighted_choice weight must be > 0".to_string(),
                ));
            }
            total_weight += weight;
            entries.push((value.to_string(), weight));
        }

        if total_weight <= 0.0 {
            return Err(GenerationError::InvalidPlan(
                "transform.weighted_choice total weight must be > 0".to_string(),
            ));
        }

        let mut roll = rng.gen_range(0.0..total_weight);
        for (value, weight) in entries {
            if roll <= weight {
                return Ok(GeneratedValue::Text(value));
            }
            roll -= weight;
        }

        Ok(GeneratedValue::Text(String::new()))
    }
}

struct MaskTransform;

impl Transform for MaskTransform {
    fn id(&self) -> &'static str {
        "transform.mask"
    }

    fn apply(
        &self,
        input: GeneratedValue,
        _ctx: &TransformContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        if matches!(input, GeneratedValue::Null) {
            return Ok(input);
        }
        let mode = params
            .and_then(|params| params.get("mode"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                GenerationError::InvalidPlan("transform.mask requires mode".to_string())
            })?;
        let mask_char = params
            .and_then(|params| params.get("mask_char"))
            .and_then(|value| value.as_str())
            .and_then(|value| value.chars().next())
            .unwrap_or('*');

        let value = value_to_string(&input);

        let masked = match mode {
            "hash" => {
                let mut hasher = Sha256::new();
                hasher.update(value.as_bytes());
                let digest = hasher.finalize();
                hex::encode(digest)
            }
            "redact" => "***".to_string(),
            "format_preserving" => format_preserving(&value, mask_char),
            _ => {
                return Err(GenerationError::InvalidPlan(
                    "transform.mask mode must be hash, redact, or format_preserving".to_string(),
                ));
            }
        };

        Ok(GeneratedValue::Text(masked))
    }
}

fn value_to_string(value: &GeneratedValue) -> String {
    match value {
        GeneratedValue::Null => String::new(),
        GeneratedValue::Bool(value) => value.to_string(),
        GeneratedValue::Int(value) => value.to_string(),
        GeneratedValue::Float(value) => value.to_string(),
        GeneratedValue::Text(value) | GeneratedValue::Uuid(value) => value.clone(),
        GeneratedValue::Date(value) => value.format("%Y-%m-%d").to_string(),
        GeneratedValue::Time(value) => value.format("%H:%M:%S").to_string(),
        GeneratedValue::Timestamp(value) => value.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }
}

fn value_kind(value: &GeneratedValue) -> &'static str {
    match value {
        GeneratedValue::Null => "null",
        GeneratedValue::Bool(_) => "bool",
        GeneratedValue::Int(_) => "int",
        GeneratedValue::Float(_) => "float",
        GeneratedValue::Text(_) => "text",
        GeneratedValue::Uuid(_) => "uuid",
        GeneratedValue::Date(_) => "date",
        GeneratedValue::Time(_) => "time",
        GeneratedValue::Timestamp(_) => "timestamp",
    }
}

fn to_title_case(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut next_upper = true;
    for ch in value.chars() {
        if ch.is_whitespace() {
            next_upper = true;
            out.push(ch);
        } else if next_upper {
            out.extend(ch.to_uppercase());
            next_upper = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

fn format_preserving(value: &str, mask_char: char) -> String {
    if let Some((user, domain)) = value.split_once('@') {
        let masked_user = mask_keep_edges(user, mask_char);
        return format!("{masked_user}@{domain}");
    }

    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.len() == 11 {
        let last = &digits[digits.len() - 2..];
        return format!(
            "{mask}{mask}{mask}.{mask}{mask}{mask}.{mask}{mask}{mask}-{last}",
            mask = mask_char
        );
    }
    if digits.len() == 14 {
        let last = &digits[digits.len() - 2..];
        return format!(
            "{mask}{mask}.{mask}{mask}{mask}.{mask}{mask}{mask}/{mask}{mask}{mask}{mask}-{last}",
            mask = mask_char
        );
    }

    mask_keep_edges(value, mask_char)
}

fn mask_keep_edges(value: &str, mask_char: char) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 2 {
        return chars.iter().map(|_| mask_char).collect();
    }
    let mut out = String::with_capacity(chars.len());
    out.push(chars[0]);
    for _ in 1..(chars.len() - 1) {
        out.push(mask_char);
    }
    out.push(chars[chars.len() - 1]);
    out
}
