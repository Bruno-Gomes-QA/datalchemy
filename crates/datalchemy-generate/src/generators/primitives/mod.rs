use chrono::{NaiveDateTime, NaiveTime, Timelike};
use rand::Rng;
use rand_regex::Regex as RandRegex;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};
use crate::params::{
    ParamKind, ParamSpec, TextLimits, parse_date_value, parse_time_value, parse_timestamp_value,
    text_limits, validate_params, validate_text_constraints,
};

const DEFAULT_INT_MIN: i64 = 0;
const DEFAULT_INT_MAX: i64 = 10000;
const DEFAULT_FLOAT_MIN: f64 = 0.0;
const DEFAULT_FLOAT_MAX: f64 = 10000.0;
const DEFAULT_TEXT_MAX: usize = 32;
const DEFAULT_MAX_REPEAT: u32 = 32;
const DEFAULT_CHARSET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

const INT_RANGE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min", ParamKind::Int, false),
    ParamSpec::new("max", ParamKind::Int, false),
];
const INT_SEQUENCE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("start", ParamKind::Int, false),
    ParamSpec::new("step", ParamKind::Int, false),
    ParamSpec::new("max", ParamKind::Int, false),
];
const FLOAT_RANGE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min", ParamKind::Float, false),
    ParamSpec::new("max", ParamKind::Float, false),
];
const DECIMAL_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min", ParamKind::Float, false),
    ParamSpec::new("max", ParamKind::Float, false),
    ParamSpec::new("scale", ParamKind::Int, false),
];
const TEXT_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min_len", ParamKind::Int, false),
    ParamSpec::new("max_len", ParamKind::Int, false),
    ParamSpec::new("pattern", ParamKind::String, false),
    ParamSpec::new("charset", ParamKind::String, false),
    ParamSpec::new("allow_empty", ParamKind::Bool, false),
    ParamSpec::new("max_repeat", ParamKind::Int, false),
];
const TEXT_PATTERN_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("pattern", ParamKind::String, true),
    ParamSpec::new("max_repeat", ParamKind::Int, false),
    ParamSpec::new("min_len", ParamKind::Int, false),
    ParamSpec::new("max_len", ParamKind::Int, false),
    ParamSpec::new("allow_empty", ParamKind::Bool, false),
];
const TEXT_LOREM_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("words", ParamKind::Int, false),
    ParamSpec::new("min_words", ParamKind::Int, false),
    ParamSpec::new("max_words", ParamKind::Int, false),
    ParamSpec::new("min_len", ParamKind::Int, false),
    ParamSpec::new("max_len", ParamKind::Int, false),
    ParamSpec::new("allow_empty", ParamKind::Bool, false),
];
const DATE_RANGE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min", ParamKind::Date, false),
    ParamSpec::new("max", ParamKind::Date, false),
];
const TIME_RANGE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min", ParamKind::Time, false),
    ParamSpec::new("max", ParamKind::Time, false),
];
const TIMESTAMP_RANGE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min", ParamKind::Timestamp, false),
    ParamSpec::new("max", ParamKind::Timestamp, false),
];

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_generator(Box::new(BoolGenerator));
    registry.register_generator(Box::new(IntRangeGenerator {
        id: "primitive.int",
    }));
    registry.register_generator(Box::new(IntRangeGenerator {
        id: "primitive.int.range",
    }));
    registry.register_generator(Box::new(IntSequenceHintGenerator));
    registry.register_generator(Box::new(FloatRangeGenerator {
        id: "primitive.float",
    }));
    registry.register_generator(Box::new(FloatRangeGenerator {
        id: "primitive.float.range",
    }));
    registry.register_generator(Box::new(DecimalNumericGenerator));
    registry.register_generator(Box::new(TextGenerator));
    registry.register_generator(Box::new(TextPatternGenerator));
    registry.register_generator(Box::new(TextLoremGenerator));
    registry.register_generator(Box::new(UuidGenerator {
        id: "primitive.uuid",
    }));
    registry.register_generator(Box::new(UuidGenerator {
        id: "primitive.uuid.v4",
    }));
    registry.register_generator(Box::new(DateRangeGenerator {
        id: "primitive.date",
    }));
    registry.register_generator(Box::new(DateRangeGenerator {
        id: "primitive.date.range",
    }));
    registry.register_generator(Box::new(TimeRangeGenerator {
        id: "primitive.time",
    }));
    registry.register_generator(Box::new(TimeRangeGenerator {
        id: "primitive.time.range",
    }));
    registry.register_generator(Box::new(TimestampRangeGenerator {
        id: "primitive.timestamp",
    }));
    registry.register_generator(Box::new(TimestampRangeGenerator {
        id: "primitive.timestamp.range",
    }));
    registry.register_generator(Box::new(EnumGenerator));
}

struct BoolGenerator;

impl Generator for BoolGenerator {
    fn id(&self) -> &'static str {
        "primitive.bool"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        validate_params(params, &[], "primitive.bool")?;
        Ok(GeneratedValue::Bool(rng.random_bool(0.5)))
    }
}

struct IntRangeGenerator {
    id: &'static str,
}

impl Generator for IntRangeGenerator {
    fn id(&self) -> &'static str {
        self.id
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, INT_RANGE_PARAMS, self.id)?;
        let min = params.get_i64("min").unwrap_or(DEFAULT_INT_MIN);
        let max = params.get_i64("max").unwrap_or(DEFAULT_INT_MAX);
        if min > max {
            return Err(GenerationError::InvalidPlan(format!(
                "{} min must be <= max",
                self.id
            )));
        }
        Ok(GeneratedValue::Int(rng.random_range(min..=max)))
    }
}

struct IntSequenceHintGenerator;

impl Generator for IntSequenceHintGenerator {
    fn id(&self) -> &'static str {
        "primitive.int.sequence_hint"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, INT_SEQUENCE_PARAMS, "primitive.int.sequence_hint")?;
        let start = params.get_i64("start").unwrap_or(1);
        let step = params.get_i64("step").unwrap_or(1);
        if step == 0 {
            return Err(GenerationError::InvalidPlan(
                "primitive.int.sequence_hint step must be non-zero".to_string(),
            ));
        }
        let value = start.saturating_add((ctx.row_index as i64).saturating_mul(step));
        let value = if let Some(max) = params.get_i64("max") {
            value.min(max)
        } else {
            value
        };
        Ok(GeneratedValue::Int(value))
    }
}

struct FloatRangeGenerator {
    id: &'static str,
}

impl Generator for FloatRangeGenerator {
    fn id(&self) -> &'static str {
        self.id
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, FLOAT_RANGE_PARAMS, self.id)?;
        let min = params.get_f64("min").unwrap_or(DEFAULT_FLOAT_MIN);
        let max = params.get_f64("max").unwrap_or(DEFAULT_FLOAT_MAX);
        if min > max {
            return Err(GenerationError::InvalidPlan(format!(
                "{} min must be <= max",
                self.id
            )));
        }
        Ok(GeneratedValue::Float(rng.random_range(min..=max)))
    }
}

struct DecimalNumericGenerator;

impl Generator for DecimalNumericGenerator {
    fn id(&self) -> &'static str {
        "primitive.decimal.numeric"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, DECIMAL_PARAMS, "primitive.decimal.numeric")?;
        let min = params.get_f64("min").unwrap_or(DEFAULT_FLOAT_MIN);
        let max = params.get_f64("max").unwrap_or(DEFAULT_FLOAT_MAX);
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "primitive.decimal.numeric min must be <= max".to_string(),
            ));
        }
        let scale = if let Some(scale) = params.get_i64("scale") {
            if scale < 0 {
                return Err(GenerationError::InvalidPlan(
                    "primitive.decimal.numeric scale must be >= 0".to_string(),
                ));
            }
            i32::try_from(scale).map_err(|_| {
                GenerationError::InvalidPlan(
                    "primitive.decimal.numeric scale must fit i32".to_string(),
                )
            })?
        } else {
            ctx.column.column_type.numeric_scale.unwrap_or(2).max(0)
        };
        let value = rng.random_range(min..=max);
        let factor = 10_f64.powi(scale);
        let rounded = (value * factor).round() / factor;
        Ok(GeneratedValue::Float(rounded))
    }
}

struct TextGenerator;

impl Generator for TextGenerator {
    fn id(&self) -> &'static str {
        "primitive.text"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, TEXT_PARAMS, "primitive.text")?;
        let limits = text_limits(
            &params,
            "primitive.text",
            ctx.column.column_type.character_max_length,
        )?;
        let pattern = params.get_str("pattern");
        let charset = params.get_str("charset");

        if pattern.is_some() && charset.is_some() {
            return Err(GenerationError::InvalidPlan(
                "primitive.text cannot combine pattern with charset".to_string(),
            ));
        }

        if params.get_i64("max_repeat").is_some() && pattern.is_none() {
            return Err(GenerationError::InvalidPlan(
                "primitive.text max_repeat requires pattern".to_string(),
            ));
        }

        let (min_len, max_len) = resolve_text_bounds("primitive.text", &limits)?;

        let value = if let Some(pattern) = pattern {
            let max_repeat = parse_max_repeat(&params, "primitive.text")?;
            let regex = RandRegex::compile(pattern, max_repeat).map_err(|err| {
                GenerationError::InvalidPlan(format!(
                    "invalid regex pattern for primitive.text: {}",
                    err
                ))
            })?;
            rng.sample(regex)
        } else {
            let charset = charset.unwrap_or(DEFAULT_CHARSET);
            if charset.is_empty() {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text charset must not be empty".to_string(),
                ));
            }
            let chars: Vec<char> = charset.chars().collect();
            if chars.is_empty() {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text charset must include valid characters".to_string(),
                ));
            }
            let len = if min_len == max_len {
                min_len
            } else {
                rng.random_range(min_len..=max_len)
            };
            let mut value = String::with_capacity(len);
            for _ in 0..len {
                let idx = rng.random_range(0..chars.len());
                value.push(chars[idx]);
            }
            value
        };

        validate_text_constraints("primitive.text", &value, &limits, pattern, charset)?;

        Ok(GeneratedValue::Text(value))
    }
}

struct TextPatternGenerator;

impl Generator for TextPatternGenerator {
    fn id(&self) -> &'static str {
        "primitive.text.pattern"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, TEXT_PATTERN_PARAMS, "primitive.text.pattern")?;
        let pattern = params.get_str("pattern").ok_or_else(|| {
            GenerationError::InvalidPlan(
                "primitive.text.pattern requires params.pattern".to_string(),
            )
        })?;
        let max_repeat = parse_max_repeat(&params, "primitive.text.pattern")?;
        let regex = RandRegex::compile(pattern, max_repeat).map_err(|err| {
            GenerationError::InvalidPlan(format!(
                "invalid regex pattern for primitive.text.pattern: {}",
                err
            ))
        })?;
        let value: String = rng.sample(regex);
        let limits = text_limits(
            &params,
            "primitive.text.pattern",
            ctx.column.column_type.character_max_length,
        )?;
        validate_text_constraints(
            "primitive.text.pattern",
            &value,
            &limits,
            Some(pattern),
            None,
        )?;
        Ok(GeneratedValue::Text(value))
    }
}

struct TextLoremGenerator;

impl Generator for TextLoremGenerator {
    fn id(&self) -> &'static str {
        "primitive.text.lorem"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, TEXT_LOREM_PARAMS, "primitive.text.lorem")?;
        let limits = text_limits(
            &params,
            "primitive.text.lorem",
            ctx.column.column_type.character_max_length,
        )?;

        let words = if let Some(words) = params.get_i64("words") {
            if words < 0 {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text.lorem words must be >= 0".to_string(),
                ));
            }
            let words = u32::try_from(words).map_err(|_| {
                GenerationError::InvalidPlan("primitive.text.lorem words must fit u32".to_string())
            })?;
            if words == 0 && !limits.allow_empty {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text.lorem words must be > 0 when allow_empty is false".to_string(),
                ));
            }
            words
        } else {
            let min_words = params.get_i64("min_words").unwrap_or(3);
            let max_words = params.get_i64("max_words").unwrap_or(8);
            if min_words < 0 || max_words < 0 {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text.lorem min_words/max_words must be >= 0".to_string(),
                ));
            }
            let min_words = u32::try_from(min_words).map_err(|_| {
                GenerationError::InvalidPlan(
                    "primitive.text.lorem min_words must fit u32".to_string(),
                )
            })?;
            let max_words = u32::try_from(max_words).map_err(|_| {
                GenerationError::InvalidPlan(
                    "primitive.text.lorem max_words must fit u32".to_string(),
                )
            })?;
            if min_words > max_words {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text.lorem min_words must be <= max_words".to_string(),
                ));
            }
            if min_words == 0 && !limits.allow_empty {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text.lorem min_words must be > 0 when allow_empty is false"
                        .to_string(),
                ));
            }
            rng.random_range(min_words..=max_words)
        } as usize;

        let mut value = String::new();
        for idx in 0..words {
            if idx > 0 {
                value.push(' ');
            }
            let word = LOREM_WORDS[rng.random_range(0..LOREM_WORDS.len())];
            value.push_str(word);
        }
        validate_text_constraints("primitive.text.lorem", &value, &limits, None, None)?;
        Ok(GeneratedValue::Text(value))
    }
}

struct UuidGenerator {
    id: &'static str,
}

impl Generator for UuidGenerator {
    fn id(&self) -> &'static str {
        self.id
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        validate_params(params, &[], self.id)?;
        let mut bytes = [0_u8; 16];
        rng.fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(GeneratedValue::Uuid(
            uuid::Uuid::from_bytes(bytes).to_string(),
        ))
    }
}

struct DateRangeGenerator {
    id: &'static str,
}

impl Generator for DateRangeGenerator {
    fn id(&self) -> &'static str {
        self.id
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, DATE_RANGE_PARAMS, self.id)?;
        let default_min = ctx.base_date;
        let default_max = ctx.base_date + chrono::Duration::days(365);
        let min = params
            .get_str("min")
            .and_then(parse_date_value)
            .unwrap_or(default_min);
        let max = params
            .get_str("max")
            .and_then(parse_date_value)
            .unwrap_or(default_max);
        if min > max {
            return Err(GenerationError::InvalidPlan(format!(
                "{} min must be <= max",
                self.id
            )));
        }
        let span = (max - min).num_days().max(0);
        let offset = rng.random_range(0..=span) as i64;
        Ok(GeneratedValue::Date(min + chrono::Duration::days(offset)))
    }
}

struct TimeRangeGenerator {
    id: &'static str,
}

impl Generator for TimeRangeGenerator {
    fn id(&self) -> &'static str {
        self.id
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, TIME_RANGE_PARAMS, self.id)?;
        let min = params
            .get_str("min")
            .and_then(parse_time_value)
            .unwrap_or_else(|| safe_time(0, 0, 0));
        let max = params
            .get_str("max")
            .and_then(parse_time_value)
            .unwrap_or_else(|| safe_time(23, 59, 59));

        if min > max {
            return Err(GenerationError::InvalidPlan(format!(
                "{} min must be <= max",
                self.id
            )));
        }

        let min_seconds = min.num_seconds_from_midnight() as i64;
        let max_seconds = max.num_seconds_from_midnight() as i64;
        let seconds = rng.random_range(min_seconds..=max_seconds) as u32;
        let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0)
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        Ok(GeneratedValue::Time(time))
    }
}

struct TimestampRangeGenerator {
    id: &'static str,
}

impl Generator for TimestampRangeGenerator {
    fn id(&self) -> &'static str {
        self.id
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, TIMESTAMP_RANGE_PARAMS, self.id)?;
        let default_min = ctx
            .base_date
            .and_hms_opt(0, 0, 0)
            .unwrap_or_else(|| NaiveDateTime::new(ctx.base_date, NaiveTime::default()));
        let default_max = ctx
            .base_date
            .and_hms_opt(23, 59, 59)
            .unwrap_or_else(|| NaiveDateTime::new(ctx.base_date, safe_time(23, 59, 59)))
            + chrono::Duration::days(365);
        let min = params
            .get_str("min")
            .and_then(parse_timestamp_value)
            .unwrap_or(default_min);
        let max = params
            .get_str("max")
            .and_then(parse_timestamp_value)
            .unwrap_or(default_max);
        if min > max {
            return Err(GenerationError::InvalidPlan(format!(
                "{} min must be <= max",
                self.id
            )));
        }
        let span = (max - min).num_seconds().max(0);
        let offset = rng.random_range(0..=span) as i64;
        Ok(GeneratedValue::Timestamp(
            min + chrono::Duration::seconds(offset),
        ))
    }
}

struct EnumGenerator;

impl Generator for EnumGenerator {
    fn id(&self) -> &'static str {
        "primitive.enum"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        validate_params(params, &[], "primitive.enum")?;
        let values = ctx.enum_values.ok_or_else(|| {
            GenerationError::InvalidPlan("enum values missing for primitive.enum".to_string())
        })?;
        if values.is_empty() {
            return Ok(GeneratedValue::Text("unknown".to_string()));
        }
        let idx = rng.random_range(0..values.len());
        Ok(GeneratedValue::Text(values[idx].clone()))
    }
}

fn resolve_text_bounds(
    ctx: &'static str,
    limits: &TextLimits,
) -> Result<(usize, usize), GenerationError> {
    let min_len = limits
        .min_len
        .unwrap_or(if limits.allow_empty { 0 } else { 1 });
    let max_len = limits
        .max_len
        .or(limits.schema_max)
        .unwrap_or(DEFAULT_TEXT_MAX);
    if min_len > max_len {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: min_len must be <= max_len"
        )));
    }
    Ok((min_len, max_len))
}

fn parse_max_repeat(
    params: &crate::params::ParamMap<'_>,
    ctx: &'static str,
) -> Result<u32, GenerationError> {
    if let Some(value) = params.get_i64("max_repeat") {
        if value <= 0 {
            return Err(GenerationError::InvalidPlan(format!(
                "{ctx}: max_repeat must be > 0"
            )));
        }
        u32::try_from(value)
            .map_err(|_| GenerationError::InvalidPlan(format!("{ctx}: max_repeat must fit u32")))
    } else {
        Ok(DEFAULT_MAX_REPEAT)
    }
}

fn safe_time(hours: u32, minutes: u32, seconds: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hours, minutes, seconds)
        .or_else(|| {
            NaiveTime::from_num_seconds_from_midnight_opt(hours * 3600 + minutes * 60 + seconds, 0)
        })
        .unwrap_or_default()
}

const LOREM_WORDS: &[&str] = &[
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
    "sed",
    "do",
    "eiusmod",
    "tempor",
    "incididunt",
    "ut",
    "labore",
    "et",
    "dolore",
    "magna",
    "aliqua",
];
