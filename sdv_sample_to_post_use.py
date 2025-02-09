from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import PeftModel  # Biblioteca para LoRA

# Modelo base e caminho dos adaptadores
base_model_name = "deepseek-ai/DeepSeek-R1-Distill-Llama-8B"
adapter_model_path = "bruno-gomes-qa/datalchemy-model"

# Carregar o modelo base e o tokenizer
tokenizer = AutoTokenizer.from_pretrained(base_model_name)
base_model = AutoModelForCausalLM.from_pretrained(base_model_name)

# Carregar os adaptadores LoRA
model = PeftModel.from_pretrained(base_model, adapter_model_path)

# Usar o modelo para inferência
prompt = "Olá, como você está?"
inputs = tokenizer(prompt, return_tensors="pt").to("cpu")
outputs = model.generate(**inputs)
response = tokenizer.decode(outputs[0], skip_special_tokens=True)

print(response)