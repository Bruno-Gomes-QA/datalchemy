use rand::RngCore;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::faker_rs::FakeRsAdapter;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};
use crate::params::{
    ParamKind, ParamSpec, text_limits, validate_params, validate_text_constraints,
};

pub fn register(registry: &mut GeneratorRegistry) {
    for &id in FakeRsAdapter::list_ids() {
        registry.register_generator(Box::new(FakerAdapterGenerator { id }));
    }
}

const FAKER_TEXT_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min_len", ParamKind::Int, false),
    ParamSpec::new("max_len", ParamKind::Int, false),
    ParamSpec::new("pattern", ParamKind::String, false),
    ParamSpec::new("charset", ParamKind::String, false),
    ParamSpec::new("allow_empty", ParamKind::Bool, false),
];

struct FakerAdapterGenerator {
    id: &'static str,
}

impl Generator for FakerAdapterGenerator {
    fn id(&self) -> &'static str {
        self.id
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let params = validate_params(params, FAKER_TEXT_PARAMS, self.id)?;
        let limits = text_limits(
            &params,
            self.id,
            ctx.column.column_type.character_max_length,
        )?;
        let pattern = params.get_str("pattern");
        let charset = params.get_str("charset");
        if let Some(charset) = charset
            && charset.is_empty()
        {
            return Err(GenerationError::InvalidPlan(format!(
                "{}: charset must not be empty",
                self.id
            )));
        }

        let value = FakeRsAdapter::generate_value(self.id, ctx.generator_locale, None, rng)?;
        if let GeneratedValue::Text(text) = &value {
            validate_text_constraints(self.id, text, &limits, pattern, charset)?;
        }
        Ok(value)
    }
}
