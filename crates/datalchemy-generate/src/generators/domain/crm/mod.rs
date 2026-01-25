use rand::Rng;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_generator(Box::new(LeadStageGenerator));
    registry.register_generator(Box::new(ActivityTypeGenerator));
    registry.register_generator(Box::new(DealValueGenerator));
    registry.register_generator(Box::new(PipelineNameGenerator));
}

struct LeadStageGenerator;

impl Generator for LeadStageGenerator {
    fn id(&self) -> &'static str {
        "domain.crm.lead_stage"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ["novo", "qualificado", "perdido"];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

struct ActivityTypeGenerator;

impl Generator for ActivityTypeGenerator {
    fn id(&self) -> &'static str {
        "domain.crm.activity_type"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ["tarefa", "reuniao", "anotacao"];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

struct DealValueGenerator;

impl Generator for DealValueGenerator {
    fn id(&self) -> &'static str {
        "domain.crm.deal_value"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let min = get_f64(params, "min").unwrap_or(1000.0);
        let max = get_f64(params, "max").unwrap_or(75000.0);
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "domain.crm.deal_value min must be <= max".to_string(),
            ));
        }
        let value = rng.gen_range(min..=max);
        Ok(GeneratedValue::Float(round_currency(value)))
    }
}

struct PipelineNameGenerator;

impl Generator for PipelineNameGenerator {
    fn id(&self) -> &'static str {
        "domain.crm.pipeline_name"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ["Prospeccao", "Qualificacao", "Fechamento"];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

fn pick(values: &[&str], rng: &mut dyn rand::RngCore) -> String {
    let idx = rng.gen_range(0..values.len());
    values[idx].to_string()
}

fn round_currency(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn get_f64(params: Option<&Value>, key: &str) -> Option<f64> {
    params
        .and_then(|params| params.get(key))
        .and_then(|value| value.as_f64())
}
