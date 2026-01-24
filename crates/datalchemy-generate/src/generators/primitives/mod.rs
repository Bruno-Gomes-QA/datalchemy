use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use rand::Rng;
use rand_regex::Regex as RandRegex;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_generator(Box::new(BoolGenerator));
    registry.register_generator(Box::new(IntRangeGenerator));
    registry.register_generator(Box::new(IntSequenceHintGenerator));
    registry.register_generator(Box::new(FloatRangeGenerator));
    registry.register_generator(Box::new(DecimalNumericGenerator));
    registry.register_generator(Box::new(TextPatternGenerator));
    registry.register_generator(Box::new(TextLoremGenerator));
    registry.register_generator(Box::new(UuidV4Generator));
    registry.register_generator(Box::new(DateRangeGenerator));
    registry.register_generator(Box::new(TimeRangeGenerator));
    registry.register_generator(Box::new(TimestampRangeGenerator));
    registry.register_generator(Box::new(EnumGenerator));
}

struct BoolGenerator;

impl Generator for BoolGenerator {
    fn id(&self) -> &'static str {
        "primitive.bool"
    }

    fn generate(
        &self,
        _ctx: &GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        Ok(GeneratedValue::Bool(rng.gen_bool(0.5)))
    }
}

struct IntRangeGenerator;

impl Generator for IntRangeGenerator {
    fn id(&self) -> &'static str {
        "primitive.int.range"
    }

    fn generate(
        &self,
        _ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let min = get_i64(params, "min").unwrap_or(0);
        let max = get_i64(params, "max").unwrap_or(10000);
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "primitive.int.range min must be <= max".to_string(),
            ));
        }
        Ok(GeneratedValue::Int(rng.gen_range(min..=max)))
    }
}

struct IntSequenceHintGenerator;

impl Generator for IntSequenceHintGenerator {
    fn id(&self) -> &'static str {
        "primitive.int.sequence_hint"
    }

    fn generate(
        &self,
        ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let start = get_i64(params, "start").unwrap_or(1);
        let step = get_i64(params, "step").unwrap_or(1);
        if step == 0 {
            return Err(GenerationError::InvalidPlan(
                "primitive.int.sequence_hint step must be non-zero".to_string(),
            ));
        }
        let value = start.saturating_add((ctx.row_index as i64).saturating_mul(step));
        let value = if let Some(max) = get_i64(params, "max") {
            value.min(max)
        } else {
            value
        };
        Ok(GeneratedValue::Int(value))
    }
}

struct FloatRangeGenerator;

impl Generator for FloatRangeGenerator {
    fn id(&self) -> &'static str {
        "primitive.float.range"
    }

    fn generate(
        &self,
        _ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let min = get_f64(params, "min").unwrap_or(0.0);
        let max = get_f64(params, "max").unwrap_or(10000.0);
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "primitive.float.range min must be <= max".to_string(),
            ));
        }
        Ok(GeneratedValue::Float(rng.gen_range(min..=max)))
    }
}

struct DecimalNumericGenerator;

impl Generator for DecimalNumericGenerator {
    fn id(&self) -> &'static str {
        "primitive.decimal.numeric"
    }

    fn generate(
        &self,
        ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let min = get_f64(params, "min").unwrap_or(0.0);
        let max = get_f64(params, "max").unwrap_or(10000.0);
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "primitive.decimal.numeric min must be <= max".to_string(),
            ));
        }
        let scale = get_u32(params, "scale")
            .map(|scale| scale as i32)
            .or(ctx.column.column_type.numeric_scale)
            .unwrap_or(2)
            .max(0);
        let value = rng.gen_range(min..=max);
        let factor = 10_f64.powi(scale);
        let rounded = (value * factor).round() / factor;
        Ok(GeneratedValue::Float(rounded))
    }
}

struct TextPatternGenerator;

impl Generator for TextPatternGenerator {
    fn id(&self) -> &'static str {
        "primitive.text.pattern"
    }

    fn generate(
        &self,
        ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let pattern = params
            .and_then(|params| params.get("pattern"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                GenerationError::InvalidPlan(
                    "primitive.text.pattern requires params.pattern".to_string(),
                )
            })?;
        let max_repeat = get_u32(params, "max_repeat").unwrap_or(32);
        let regex = RandRegex::compile(pattern, max_repeat).map_err(|err| {
            GenerationError::InvalidPlan(format!(
                "invalid regex pattern for primitive.text.pattern: {}",
                err
            ))
        })?;
        let mut value: String = rng.sample(regex);
        if let Some(max_len) = ctx.column.column_type.character_max_length {
            value.truncate(max_len as usize);
        }
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
        ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let words = if let Some(words) = get_u32(params, "words") {
            words
        } else {
            let min = get_u32(params, "min_words").unwrap_or(3);
            let max = get_u32(params, "max_words").unwrap_or(8);
            if min > max {
                return Err(GenerationError::InvalidPlan(
                    "primitive.text.lorem min_words must be <= max_words".to_string(),
                ));
            }
            rng.gen_range(min..=max)
        } as usize;

        let mut value = String::new();
        for idx in 0..words {
            if idx > 0 {
                value.push(' ');
            }
            let word = LOREM_WORDS[rng.gen_range(0..LOREM_WORDS.len())];
            value.push_str(word);
        }
        if let Some(max_len) = ctx.column.column_type.character_max_length {
            value.truncate(max_len as usize);
        }
        Ok(GeneratedValue::Text(value))
    }
}

struct UuidV4Generator;

impl Generator for UuidV4Generator {
    fn id(&self) -> &'static str {
        "primitive.uuid.v4"
    }

    fn generate(
        &self,
        _ctx: &GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let mut bytes = [0_u8; 16];
        rng.fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(GeneratedValue::Uuid(
            uuid::Uuid::from_bytes(bytes).to_string(),
        ))
    }
}

struct DateRangeGenerator;

impl Generator for DateRangeGenerator {
    fn id(&self) -> &'static str {
        "primitive.date.range"
    }

    fn generate(
        &self,
        ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let default_min = ctx.base_date;
        let default_max = ctx.base_date + chrono::Duration::days(365);
        let (min, max) = parse_date_range(params, default_min, default_max)?;
        let span = (max - min).num_days().max(0);
        let offset = rng.gen_range(0..=span) as i64;
        Ok(GeneratedValue::Date(min + chrono::Duration::days(offset)))
    }
}

struct TimeRangeGenerator;

impl Generator for TimeRangeGenerator {
    fn id(&self) -> &'static str {
        "primitive.time.range"
    }

    fn generate(
        &self,
        _ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let min = params
            .and_then(|params| params.get("min"))
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveTime::parse_from_str(value, "%H:%M:%S").ok())
            .unwrap_or_else(|| safe_time(0, 0, 0));
        let max = params
            .and_then(|params| params.get("max"))
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveTime::parse_from_str(value, "%H:%M:%S").ok())
            .unwrap_or_else(|| safe_time(23, 59, 59));

        if min > max {
            return Err(GenerationError::InvalidPlan(
                "primitive.time.range min must be <= max".to_string(),
            ));
        }

        let min_seconds = min.num_seconds_from_midnight() as i64;
        let max_seconds = max.num_seconds_from_midnight() as i64;
        let seconds = rng.gen_range(min_seconds..=max_seconds) as u32;
        let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0)
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        Ok(GeneratedValue::Time(time))
    }
}

struct TimestampRangeGenerator;

impl Generator for TimestampRangeGenerator {
    fn id(&self) -> &'static str {
        "primitive.timestamp.range"
    }

    fn generate(
        &self,
        ctx: &GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let default_min = ctx
            .base_date
            .and_hms_opt(0, 0, 0)
            .unwrap_or_else(|| NaiveDateTime::new(ctx.base_date, NaiveTime::default()));
        let default_max = ctx
            .base_date
            .and_hms_opt(23, 59, 59)
            .unwrap_or_else(|| NaiveDateTime::new(ctx.base_date, safe_time(23, 59, 59)))
            + chrono::Duration::days(365);
        let (min, max) = parse_timestamp_range(params, default_min, default_max)?;
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "primitive.timestamp.range min must be <= max".to_string(),
            ));
        }
        let span = (max - min).num_seconds().max(0);
        let offset = rng.gen_range(0..=span) as i64;
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
        ctx: &GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ctx.enum_values.ok_or_else(|| {
            GenerationError::InvalidPlan("enum values missing for primitive.enum".to_string())
        })?;
        if values.is_empty() {
            return Ok(GeneratedValue::Text("unknown".to_string()));
        }
        let idx = rng.gen_range(0..values.len());
        Ok(GeneratedValue::Text(values[idx].clone()))
    }
}

fn get_i64(params: Option<&Value>, key: &str) -> Option<i64> {
    params
        .and_then(|params| params.get(key))
        .and_then(|v| v.as_i64())
}

fn get_u32(params: Option<&Value>, key: &str) -> Option<u32> {
    params
        .and_then(|params| params.get(key))
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
}

fn get_f64(params: Option<&Value>, key: &str) -> Option<f64> {
    params
        .and_then(|params| params.get(key))
        .and_then(|v| v.as_f64())
}

fn parse_date_range(
    params: Option<&Value>,
    default_min: NaiveDate,
    default_max: NaiveDate,
) -> Result<(NaiveDate, NaiveDate), GenerationError> {
    let min = params
        .and_then(|params| params.get("min"))
        .and_then(|value| value.as_str())
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .unwrap_or(default_min);
    let max = params
        .and_then(|params| params.get("max"))
        .and_then(|value| value.as_str())
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .unwrap_or(default_max);
    if min > max {
        return Err(GenerationError::InvalidPlan(
            "primitive.date.range min must be <= max".to_string(),
        ));
    }
    Ok((min, max))
}

fn parse_timestamp_range(
    params: Option<&Value>,
    default_min: NaiveDateTime,
    default_max: NaiveDateTime,
) -> Result<(NaiveDateTime, NaiveDateTime), GenerationError> {
    let min = params
        .and_then(|params| params.get("min"))
        .and_then(|value| value.as_str())
        .and_then(|value| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").ok())
        .unwrap_or(default_min);
    let max = params
        .and_then(|params| params.get("max"))
        .and_then(|value| value.as_str())
        .and_then(|value| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").ok())
        .unwrap_or(default_max);
    Ok((min, max))
}

fn safe_time(hours: u32, minutes: u32, seconds: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hours, minutes, seconds)
        .or_else(|| {
            NaiveTime::from_num_seconds_from_midnight_opt(hours * 3600 + minutes * 60 + seconds, 0)
        })
        .unwrap_or_else(NaiveTime::default)
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
