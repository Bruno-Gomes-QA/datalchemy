import os
import subprocess

import tiktoken
from openai import OpenAI
from sqlalchemy import inspect

from .database import DatabaseConnectionManager


class Generators:
    def __init__(
        self, manager: DatabaseConnectionManager, OPENAI_API_KEY: str
    ):
        """
        Inicializa os geradores de dados e define o gerenciador de conexões.

        Args:
            manager (DatabaseConnectionManager): Gerenciador de conexões com bancos de dados.
            OPENAI_API_KEY (str): Chave de autenticação da API da OpenAI.
        """
        self.manager = manager
        self.models_dir = 'Datalchemy_Models'
        self.history = {}  # Armazena o histórico de prompts e respostas
        self.openai_client = OpenAI(api_key=OPENAI_API_KEY)

    def generate_data(
        self,
        db_name: str,
        prompt: str,
        model: str = 'gpt-3.5-turbo-16k',
        max_tokens: int = 16385,
        temp: float = 0.3,
    ):
        """
        Gera dados semânticos usando o modelo OpenAI com base em um prompt.

        Args:
            db_name (str): Nome do banco de dados associado à geração de dados.
            prompt (str): Mensagem enviada ao modelo para geração de dados.
            model (str): Modelo OpenAI a ser utilizado (default: 'gpt-3.5-turbo').
            max_tokens (int): Número máximo de tokens permitidos na resposta (default: 4096).
            temp (float): Grau de criatividade da resposta (default: 0.3).

        Returns:
            str: Resposta gerada pelo modelo OpenAI.

        Raises:
            ValueError: Se o banco de dados não for encontrado.
            RuntimeError: Se ocorrer um erro na comunicação com a API da OpenAI.
        """
        content = """
Você é um assistente especializado em geração de dados sintéticos. Sua tarefa é gerar resultados no formato JSON seguindo estas regras:

Formato: Responda apenas em JSON. Não inclua explicações ou comentários.
Estrutura: Os dados devem seguir a estrutura fornecida (tabelas, colunas e relações) e respeitar constraints (e.g., NOT NULL, UNIQUE, FK).
Relações: Mantenha consistência nas FK e nas relações entre tabelas.
Quantidade de Dados: Gere 10 registros por tabela, salvo especificação no prompt. Respeite a coerência dos dados. Ex.: Produtos devem pertencer a departamentos válidos.
Formato do JSON:
Ordem: Primeiro tabelas de FK referenciadas, depois dependentes.
Exemplo:
{
    "tabela": {
        "atributos": ["coluna1", "coluna2"],
        "valores": [
            [v1, v2],
            [v3, v4]
        ]
    }
}
Inconsistências: Retorne {} para solicitações inválidas ou com conflitos.
Exemplos do Usuário: Baseie-se em exemplos fornecidos e gere dados consistentes.
Segurança: Anonimize dados sensíveis (e.g., CPFs, e-mails) e siga regras como GDPR/LGPD.
Plausibilidade: Gere dados realistas (e.g., sem preços negativos).
Idioma: Gere em pt-BR, salvo solicitação contrária.
Se as IDs são auto-increment então não devem ser geradas na resposta.
"""
        if db_name not in self.manager.connections:
            raise ValueError(
                f"O banco de dados '{db_name}' não foi encontrado no gerenciador."
            )
        engine = self.manager.get_engine(db_name)
        database_structure = self.get_metadata(engine)

        database_structure_tokens = self.count_tokens(
            database_structure, model
        )
        content_tokens = self.count_tokens(content, model)
        prompt_tokens = self.count_tokens(prompt, model)
        res_tokens = (
            max_tokens
            - (database_structure_tokens + content_tokens + prompt_tokens)
            - 40 # Overhead
        )   

        if res_tokens < 1000:
            raise ValueError(
                f'Quantidade de tokens restantes menor que o mínimo de 1000: tokens restantes = {res_tokens}'
            )
        try:
            # Envia o prompt para o modelo
            response = self.openai_client.chat.completions.create(
                model=model,
                messages=[
                    {
                        'role': 'system',  # Prompt para o modelo seguir as regras e entregar a melhor resposta no formato adequado
                        'content': content,
                    },
                    {
                        'role': 'system',
                        'content': database_structure,
                    },  # Estrutura do banco de dados solicitado
                    {'role': 'user', 'content': prompt},  # Prompt do usuário
                ],
                max_tokens=res_tokens,
                temperature=temp,
            )

            # Extrai o resultado da resposta
            result = response.choices[0].message.content

            # Salva no histórico
            gen_key = f'gen{len(self.history) + 1}'
            self.history[gen_key] = {'prompt': prompt, 'result': result}

            return result

        except Exception as e:
            raise RuntimeError(f'Erro ao gerar dados: {str(e)}')

    def generate_models(self, db_name: str, save_to_file: bool = False):
        """
        Gera os modelos SQLAlchemy do banco de dados especificado.

        Args:
            db_name (str): Nome do banco de dados a ser utilizado.
            save_to_file (bool): Indica se o código gerado deve ser salvo em arquivo. Default é False.

        Returns:
            str: Código gerado pelo sqlacodegen.

        Raises:
            ValueError: Se o banco de dados não for encontrado.
            RuntimeError: Se ocorrer um erro ao gerar os modelos.
        """
        if db_name not in self.manager.connections:
            raise ValueError(
                f"O banco de dados '{db_name}' não foi encontrado no gerenciador."
            )

        db_config = self.manager.get_config_by_name(db_name)
        db_url = self.manager.build_connection_url(db_config)

        try:
            # Verifica se o sqlacodegen está instalado
            result = subprocess.run(
                ['sqlacodegen', '--help'], capture_output=True, text=True
            )
            if result.returncode != 0:
                raise EnvironmentError('sqlacodegen não está instalado.')

            # Gera os modelos do banco
            result = subprocess.run(
                ['sqlacodegen', db_url],
                capture_output=True,
                text=True,
                check=True,
            )
            code = result.stdout  # Código gerado em memória

            # Salva em arquivo, se solicitado
            if save_to_file:
                os.makedirs(
                    self.models_dir, exist_ok=True
                )  # Garante que a pasta models exista
                output_path = (
                    f'{self.models_dir}/{db_name}.py'  # Nome do arquivo
                )
                with open(output_path, 'w') as f:
                    f.write(code)
                print(f'Modelos salvos em: {output_path}')

            return code

        except subprocess.CalledProcessError as e:
            raise RuntimeError(
                f"Erro ao gerar models para '{db_name}': {e.stderr}"
            )
        except Exception as e:
            raise RuntimeError(f'Erro inesperado ao gerar models: {str(e)}')

    @staticmethod
    def get_metadata(self, engine):
        """
        Retorna a estrutura do banco de dados a partir dos metadados.

        :param engine: Instância do SQLAlchemy Engine conectada ao banco de dados.
        :return: Dicionário contendo informações sobre tabelas, colunas e chaves estrangeiras.
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

    @staticmethod
    def count_tokens(msg: str, model: str = 'gpt-3.5-turbo-16k'):
        try:
            encoding = tiktoken.encoding_for_model(model)   # Inicia o tiktoken
            return len(encoding.encode(msg))
        except Exception as e:
            raise RuntimeError(f'Erro inesperado contar tokens: {str(e)}')
