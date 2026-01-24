use crate::generators::GeneratorRegistry;

pub mod crm;
pub mod finance;
pub mod logistics;

pub fn register(registry: &mut GeneratorRegistry) {
    crm::register(registry);
    finance::register(registry);
    logistics::register(registry);
}
