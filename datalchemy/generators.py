import json
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
        qtd_lines: int = 100,
    ):
        """
        Gera dados semânticos usando o modelo OpenAI com base em um prompt.

        Args:
            db_name (str): Nome do banco de dados associado à geração de dados.
            prompt (str): Mensagem enviada ao modelo para geração de dados.
            model (str): Modelo OpenAI a ser utilizado.
            max_tokens (int): Número máximo de tokens permitidos na resposta, recomendamos não alterar essa quantidade, pois a função realiza o cálculo automático e administra os tokens para melhor performance na resposta.
            temp (float): Grau de criatividade da resposta (default: 0.3).
            qtd_lines (int): Quantidade máxima de linhas de dados a serem geradas (default: 100).

        Returns:
            str: Resposta em JSON com os dados gerados.

        Raises:
            ValueError: Se o banco de dados não for encontrado.
            RuntimeError: Se ocorrer um erro na comunicação com a API da OpenAI.
        """
        contents = self.read_prompts()

        if db_name not in self.manager.connections:
            raise ValueError(
                f"O banco de dados '{db_name}' não foi encontrado no gerenciador."
            )
        engine = self.manager.get_engine(db_name)
        database_structure = self.get_metadata(engine)

        try:
            tables_in_prompt = self.fetch_model_response(
                prompt,
                contents.get('get_tables_in_user_prompt'),
                database_structure,
                model,
                max_tokens,
                temp,
            )
            tables_in_prompt_ = self.get_parental_tables(
                tables_in_prompt, database_structure
            )
            database_structure_in_prompt = self.filter_tables(
                database_structure, tables_in_prompt_
            )
            result = self.fetch_model_response(
                prompt,
                contents.get('data_generation_rules'),
                database_structure_in_prompt,
                model,
                max_tokens,
                temp,
            )
            return result
        except Exception as e:
            raise RuntimeError(f'Erro ao gerar dados: {str(e)}')

    def fetch_model_response(
        self,
        prompt: str,
        content: str,
        database_structure: dict,
        model: str,
        max_tokens: int,
        temp: float,
    ):
        """
        Obtém a resposta do modelo OpenAI com base no prompt fornecido e no conteúdo.

        Args:
            prompt (str): Mensagem enviada pelo usuário.
            content (str): Mensagem de sistema com instruções para o modelo.
            database_structure (dict): Estrutura do banco de dados a ser incluída no contexto.
            model (str): Modelo OpenAI a ser utilizado.
            max_tokens (int): Número máximo de tokens permitidos na resposta.
            temp (float): Grau de criatividade da resposta.

        Returns:
            str: Resposta do modelo no formato esperado.

        Raises:
            RuntimeError: Se ocorrer um erro ao comunicar com a API da OpenAI.
        """
        try:
            database_structure_tokens = self.count_tokens(
                str(database_structure), model
            )
            content_tokens = self.count_tokens(content, model)
            prompt_tokens = self.count_tokens(prompt, model)
            res_tokens = (
                max_tokens
                - (database_structure_tokens + content_tokens + prompt_tokens)
                - 40  # Overhead
            )
            if res_tokens < 1000:
                raise ValueError(
                    f'Quantidade de tokens restantes menor que o mínimo de 1000: tokens restantes = {res_tokens}'
                )
            response = self.openai_client.chat.completions.create(
                model=model,
                messages=[
                    {
                        'role': 'system',
                        'content': f'<ESTRUTURA_DO_BANCO> {database_structure}',
                    },  # Estrutura do banco de dados solicitado
                    {
                        'role': 'system',  # Prompt para o modelo seguir as regras e entregar a melhor resposta no formato adequado
                        'content': content,
                    },
                    {'role': 'user', 'content': prompt},  # Prompt do usuário
                ],
                max_tokens=res_tokens,
                temperature=temp,
            )

        except Exception as e:
            raise RuntimeError(f'Erro ao gerar dados: {str(e)}')

        result = response.choices[0].message.content
        gen_key = f'gen{len(self.history) + 1}'
        self.history[gen_key] = {'prompt': prompt, 'result': result}

        return result

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
    def get_metadata(engine):
        """
        Retorna a estrutura do banco de dados a partir dos metadados.

        Args:
            engine: Instância do SQLAlchemy Engine conectada ao banco de dados.

        Returns:
            dict: Dicionário contendo informações sobre tabelas, colunas e chaves estrangeiras.
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
    def get_parental_tables(llm_response, db_structure):
        """
        Valida se todas as tabelas pai necessárias para as tabelas filhas na resposta da LLM estão presentes.
        Se uma tabela pai estiver ausente, ela será adicionada na primeira posição.

        Args:
            llm_response (str): Resposta JSON da LLM contendo as tabelas geradas.
            db_structure (dict): Estrutura do banco de dados como dicionário.

        Returns:
            list: Lista atualizada de tabelas com tabelas pai adicionadas.

        Raises:
            ValueError: Se a resposta da LLM não contiver tabelas ou retornar um erro.
        """
        llm_response_json = json.loads(llm_response)
        tables = llm_response_json.get('tables', [])
        if tables == []:
            raise ValueError(llm_response_json.get('error'))
        updated_tables = list(tables)

        for i, table in enumerate(tables):
            # Verifique se a tabela está na estrutura do banco
            if table in db_structure:
                # Verifique as chaves estrangeiras da tabela
                table_ = db_structure.get(table)
                foreign_keys = table_.get('foreign_keys')
                for fk in foreign_keys:
                    # Extraia a tabela pai referenciada
                    parent_table = fk.get('referenced_table')
                    if parent_table and parent_table not in updated_tables:
                        # Adicione a tabela pai no início da lista
                        updated_tables.insert(0, parent_table)
        return updated_tables

    @staticmethod
    def count_tokens(msg: str, model: str = 'gpt-3.5-turbo-16k'):
        """
        Conta o número de tokens de uma mensagem usando o modelo especificado.

        Args:
            msg (str): Mensagem para calcular os tokens.
            model (str): Modelo OpenAI utilizado para calcular os tokens.

        Returns:
            int: Número de tokens na mensagem.

        Raises:
            RuntimeError: Se ocorrer um erro ao contar os tokens.
        """
        try:
            encoding = tiktoken.encoding_for_model(model)   # Inicia o tiktoken
            return len(encoding.encode(msg))
        except Exception as e:
            raise RuntimeError(f'Erro inesperado contar tokens: {str(e)}')

    @staticmethod
    def read_prompts():
        """
        Retorna os prompts pré-definidos usados na geração de dados e identificação de tabelas.

        Returns:
            dict: Dicionário contendo os prompts utilizados pelo sistema.
        """
        return {
            'get_tables_in_user_prompt': 'Com base na <ESTRUTURA_DO_BANCO> de dados fornecida e no contexto do prompt do usuário, identifique as tabelas relevantes para a geração de dados. Qualquer tabela fora da estrutura do banco de dados deve ser considerada como inválida e deve retornar um erro.\nCertifique-se de:\n- Considerar as tabelas mencionadas no prompt e que estejam relacionadas via constraints (e.g., FK, NOT NULL).\n- A ordem das tabelas é importante, tabela pai primeiro e tabelas filhas em seguida. Deixe a resposta na ordem de hierarquia das tabelas, mesmo que a tabela pai não tenha sido mencionada no prompt, sempre ordernar\n- Se nenhuma tabela fizer sentido com base no prompt e na <ESTRUTURA_DO_BANCO>, retorne um JSON no formato:\n\n{\n   "error": "Breve explicação do motivo."\n}\n\n- Não inclua explicações ou comentários adicionais, apenas o JSON solicitado. Retorne apenas um JSON contendo os nomes das tabelas no seguinte formato:\n\n{\n   "tables": [\n      "nome_da_tabela1",\n      "nome_da_tabela2"\n   ]\n}\nSe for preciso criar outras tabelas fora da <ESTRUTURA_DO_BANCO>, será considerado inválido',
            'data_generation_rules': 'Respostas que não preencham todas as tabelas da <ESTRUTURA_DO_BANCO> serão consideradas inválidas. Você deve responder apenas no formato JSON. Não inclua explicações, comentários ou outro conteúdo. Você é um assistente especializado em geração de dados sintéticos. Sua tarefa é gerar resultados no formato JSON seguindo estas regras:\n\n Gere dados para todas as tabelas da <ESTRUTURA_DE_DADOS>.\nNunca responda as perguntas do usuário, retorne apenas os dados solicitados no formato JSON e nada mais.\nSe o usuário solicitar mais do que 50 linhas de dados, gerar no máximo 50, distribuindo as linhas pelas tabelas incluídas na <ESTRUTURA_DO_BANCO>. Se a resposta conter a estrutura do banco de dados será considerada inválida. Se a resposta conter tabelas que não fazem parte da estrutura do banco de dados será considerada inválida. Qualquer resposta fora do formato JSON será considerada inválida. \nTodas as tabelas na <ESTRUTURA_DO_BANCO> devem ser populadas, com dados, considere a quantidade e cardinalidade.\n Não incluir os ID ou chaves primarias se elas forem auto_increment.\nFormato: Responda apenas em JSON. Não inclua explicações ou comentários.\nEstrutura: Os dados devem seguir a estrutura fornecida (tabelas, colunas e relações) e respeitar constraints (e.g., NOT NULL, UNIQUE, FK).\nRelações: Mantenha consistência nas FK e nas relações entre tabelas.\nQuantidade de Dados: Gere 10 registros por tabela, salvo especificação no prompt. Respeite a coerência dos dados. Ex.: Produtos devem pertencer a departamentos válidos.\nFormato do JSON:\nOrdem: Primeiro tabelas de FK referenciadas, depois dependentes.\nExemplo:\n{\n    "tabela": {\n        "atributos": ["coluna1", "coluna2"],\n        "valores": [\n            [v1, v2],\n            [v3, v4]\n        ]\n    }\n}\nInconsistências: Retorne {"error": "motivo do erro"} para solicitações inválidas ou com conflitos.\nExemplos do Usuário: Baseie-se em exemplos fornecidos e gere dados consistentes.\nSegurança: Anonimize dados sensíveis (e.g., CPFs, e-mails) e siga regras como GDPR/LGPD.\nPlausibilidade: Gere dados realistas (e.g., sem preços negativos).\nIdioma: Gere em pt-BR, salvo solicitação contrária.\nSe as IDs são auto-increment então não devem ser geradas na resposta. Todas as tabelas na estrutura do banco devem conter dados válidos e consistentes. Nenhuma tabela pode ficar sem dados. Respostas que não preencham todas as tabelas serão consideradas inválidas.',
        }

    @staticmethod
    def filter_tables(database_structure: dict, tables_to_keep: list):
        """
        Filtra o dicionário para manter apenas as tabelas especificadas.

        Args:
            database_structure (dict): O dicionário original contendo a estrutura completa.
            tables_to_keep (list): Uma lista das tabelas que devem ser mantidas.

        Returns:
            dict: Um novo dicionário contendo apenas as tabelas filtradas.
        """
        return {
            key: value
            for key, value in database_structure.items()
            if key in tables_to_keep
        }
