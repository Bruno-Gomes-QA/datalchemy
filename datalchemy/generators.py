import json
import os
import subprocess

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer
from sqlalchemy import inspect

from .database import DatabaseConnectionManager


class Generators:
    def __init__(
        self, manager: DatabaseConnectionManager, model_path: str
    ):
        """
        Inicializa os geradores de dados e define o gerenciador de conexões.

        Args:
            manager (DatabaseConnectionManager): Gerenciador de conexões com bancos de dados.
            model_path (str): Caminho ou nome do modelo Hugging Face a ser utilizado.
        """
        self.manager = manager
        self.models_dir = 'Datalchemy_Models'
        self.history = {}  # Armazena o histórico de prompts e respostas
        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

        self.tokenizer = AutoTokenizer.from_pretrained(model_path)
        self.model = AutoModelForCausalLM.from_pretrained(
            model_path,
            device_map="auto",
            torch_dtype="auto"
        ).eval()

    def generate_data(
        self,
        db_name: str,
        prompt: str,
    ):
        """
        Gera dados semânticos usando um modelo local da Hugging Face.

        Args:
            db_name (str): Nome do banco de dados associado à geração de dados.
            prompt (str): Mensagem enviada ao modelo para geração de dados.

        Returns:
            str: Resposta em JSON com os dados gerados.
        """
        if db_name not in self.manager.connections:
            raise ValueError(
                f"O banco de dados '{db_name}' não foi encontrado no gerenciador."
            )
        engine = self.manager.get_engine(db_name)
        database_structure = self.get_metadata(engine)

        try:
            full_prompt = self.format_prompt(prompt, database_structure)
            messages = [{"role": "user", "content": full_prompt}]
            input_ids = self.tokenizer.apply_chat_template(
                conversation=messages, tokenize=True, add_generation_prompt=True, return_tensors='pt'
            )
            output_ids = self.model.generate(input_ids.to(self.device))
            result = self.tokenizer.decode(output_ids[0][input_ids.shape[1]:], skip_special_tokens=True)
            return result
        except Exception as e:
            raise RuntimeError(f'Erro ao gerar dados: {str(e)}')

    def format_prompt(self, prompt: str, database_structure: dict):
        """
        Formata o prompt para incluir a estrutura do banco de dados.
        """
        return f"""
        <DATABASE_STRUCTURE> {json.dumps(database_structure, indent=2)}
        <USER_REQUEST> {prompt}
        """.strip()

    @staticmethod
    def get_metadata(engine):
        """
        Retorna a estrutura do banco de dados a partir dos metadados.
        """
        inspector = inspect(engine)
        metadata = {}

        for table_name in inspector.get_table_names():
            table_info = {
                'columns': [],
                'foreign_keys': [],
            }

            # Adiciona informações das colunas
            for column in inspector.get_columns(table_name):
                table_info['columns'].append(
                    {
                        'name': column['name'],
                        'type': str(column['type']),
                        'nullable': column['nullable'],
                    }
                )

            # Adiciona informações das chaves estrangeiras
            for fk in inspector.get_foreign_keys(table_name):
                table_info['foreign_keys'].append(
                    {
                        'column': fk['constrained_columns'],
                        'referenced_table': fk['referred_table'],
                        'referenced_column': fk['referred_columns'],
                    }
                )

            metadata[table_name] = table_info

        return metadata
