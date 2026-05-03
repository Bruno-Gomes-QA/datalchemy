use datalchemy_generate::generators::GeneratorRegistry;

fn main() {
    let registry = GeneratorRegistry::new();
    for id in registry.generator_ids() {
        println!("{id}");
    }
}
