import json

def convert_json_to_jsonl(input_file, output_file):
    prefix = "Below is an instruction that describes a task. Write a response that appropriately completes the request.\n\n### Instruction:\nComplete the task based on the database structure and user request.\n\n### Input:\n"
    response_prefix = "\n\n### Response:\n"

    try:
        with open(input_file, "r", encoding="utf-8") as f:
            data = json.load(f)
        
        if not isinstance(data, list):
            raise ValueError("O arquivo JSON deve conter uma lista de objetos.")

        with open(output_file, "w", encoding="utf-8") as f:
            for entry in data:
                corrected_entry = {
                    "text": prefix + entry["input"] + response_prefix + entry["output"]
                }
                f.write(json.dumps(corrected_entry, ensure_ascii=False) + "\n")

        print(f"Conversão concluída! Arquivo salvo como {output_file}")

    except json.JSONDecodeError:
        print("Erro ao processar o arquivo JSON. Verifique se ele está bem formatado.")
    except FileNotFoundError:
        print("Arquivo não encontrado. Verifique o caminho do arquivo de entrada.")
    except ValueError as e:
        print(f"Erro: {e}")


input_path = "datalchemy_full_scenarios.json" 
output_path = "datalchemy_auto_train.jsonl"
convert_json_to_jsonl(input_path, output_path)
