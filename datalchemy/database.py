import subprocess
from typing import Dict, List, Union

from sqlalchemy import create_engine, exc
from sqlalchemy.orm import sessionmaker


class DatabaseConnectionManager:
    """
    Esta classe gerencia conexões com múltiplos bancos de dados, suporta diferentes dialetos, e facilita operações.

    Attributes:
        connections (dict):
            Dicionário contendo as conexões ativas.
            A chave é o nome da conexão e o valor é outro dicionário com o engine SQLAlchemy e a factory de sessões.

    Examples:
        >>> from SDSDG_Lib import DatabaseConnectionManager

        ### Configurações de conexões
        >>> configs = [
                {
                    "name": "main_db",
                    "dialect": "mysql",
                    "username": "root",
                    "password": "password",
                    "host": "localhost",
                    "port": 3306,
                    "database": "meu_banco"
                },
                {
                    "name": "analytics_db",
                    "dialect": "postgresql",
                    "username": "admin",
                    "password": "admin123",
                    "host": "analytics.server.com",
                    "port": 5432,
                    "database": "analytics"
                }
            ]

        ### Cria o gerenciador de conexões
            >>> db_manager = DatabaseConnectionManager(configs)

        ### Obtém uma sessão para uma das conexões
            >>> session = db_manager.get_session("main_db")
    """

    def __init__(self, configs: List[Dict[str, Union[str, int]]]):
        """
        Inicializa o gerenciador de conexões com base nas configurações fornecidas.

        Args:
            configs (list): Uma lista de configurações para conexão com bancos de dados.
        """
        self.configs = configs
        self.connections = {}

        for config in configs:
            try:
                self.add_connection(config)
            except Exception as e:
                raise ValueError(
                    f"Erro ao adicionar conexão {config.get('name', '<sem_nome>')}: {e}"
                )

    def add_connection(self, config: Dict[str, Union[str, int]]):
        """
        Adiciona uma nova conexão ao gerenciador e gera os modelos correspondentes.

        Args:
            config (dict): Configurações para a conexão com o banco de dados.

        Raises:
            ValueError: Se o dicionário de configuração estiver incompleto ou contiver valores inválidos.
        """

        try:
            config_v, missing_keys = self.validate_requirements_keys(config)
            if config_v:
                connection_string = self.build_connection_url(config)
                engine = create_engine(connection_string)
                Session = sessionmaker(bind=engine)
                self.connections[config['name']] = {
                    'engine': engine,
                    'session': Session,
                }

            else:
                raise ValueError(
                    f"Configuração inválida, valores ausentes ou vazios: {', '.join(missing_keys)}"
                )
        except subprocess.SubprocessError as e:
            if 'Access denied' in str(e):
                raise PermissionError(
                    f'Acesso negado. '
                    'Verifique suas credenciais de autenticação.'
                )

    def get_session(self, name: str):
        """
        Retorna uma sessão para o banco de dados especificado.

        Args:
            name (str): Nome da conexão configurada.

        Returns:
            sqlalchemy.orm.session.Session: Uma sessão para o banco de dados especificado.

        Raises:
            ValueError: Se a conexão especificada não for encontrada.
        """
        if name not in self.connections:
            raise ValueError(f"Conexão '{name}' não encontrada.")
        try:
            return self.connections[name]['session']()
        except Exception as e:
            raise RuntimeError(
                f"Erro ao criar sessão para conexão '{name}': {e}"
            )

    def get_engine(self, name: str):
        """
        Retorna a engine para o banco de dados especificado.

        Args:
            name (str): Nome da conexão configurada.

        Returns:
            sqlalchemy.engine.Engine: A engine para o banco de dados especificado.

        Raises:
            ValueError: Se a conexão especificada não for encontrada.
        """
        if name not in self.connections:
            raise ValueError(f"Conexão '{name}' não encontrada.")
        try:
            return self.connections[name]['engine']
        except Exception as e:
            raise RuntimeError(
                f"Erro ao criar engine para conexão '{name}': {e}"
            )

    def close_all_connections(self):
        """
        Fecha todas as conexões gerenciadas.
        """
        for name, conn in self.connections.items():
            try:
                conn['engine'].dispose()
            except Exception as e:
                raise ValueError(f"Erro ao fechar conexão '{name}': {e}")
        self.connections.clear()

    def get_config_by_name(self, name):
        """
        Busca uma configuração pelo nome.

        Args:
            name (str): Nome da configuração a ser buscada.

        Returns:
            dict: Configuração correspondente ao nome fornecido.

        Raises:
            ValueError: Se nenhuma configuração for encontrada com o nome fornecido.
        """
        for config in self.configs:
            if config.get('name') == name:
                return config
        raise ValueError(f"Configuração com o nome '{name}' não encontrada.")

    @staticmethod
    def build_connection_url(config: Dict[str, Union[str, int]]) -> str:
        """
        Constrói uma URL de conexão a partir das configurações fornecidas.

        Args:
            config (dict): Configurações para a conexão com o banco de dados.

        Returns:
            str: URL de conexão no formato esperado por SQLAlchemy.

        Raises:
            ValueError: Se o dicionário de configuração estiver incompleto.
        """

        if config.get('dialect') == 'sqlite':
            url = f"{config['dialect']}:///{config['database']}"
        else:
            url = (
                f"{config['dialect']}://{config['username']}:{config['password']}@"
                f"{config['host']}:{config['port']}/{config['database']}"
            )

        return url

    @staticmethod
    def validate_requirements_keys(config):
        """
        Valida as chaves e valores de configuração antes de estabelecer uma conexão.

        Args:
            config (dict): Configuração para a conexão com o banco de dados.

        Returns:
            tuple: (config, None) se válido; (None, missing_keys) se inválido.

        Raises:
            ValueError: Se houver erros de validação nos valores fornecidos.
        """
        required_keys = ['name', 'dialect', 'database']
        is_sqlite = config.get('dialect') == 'sqlite'

        # Chaves adicionais obrigatórias para bancos que não são SQLite
        if not is_sqlite:
            required_keys.extend(['username', 'password', 'host', 'port'])

        # Verifica chaves ausentes
        missing_keys = [key for key in required_keys if not config.get(key)]
        if missing_keys:
            return None, missing_keys

        # Validações adicionais para valores
        if not isinstance(config['name'], str) or not config['name'].strip():
            raise ValueError("O campo 'name' deve ser uma string não vazia.")

        if config['dialect'] not in ['sqlite', 'mysql+pymysql', 'postgresql']:
            raise ValueError(
                f"Dialeto '{config['dialect']}' não suportado. Use 'sqlite', 'mysql' ou 'postgresql'."
            )

        if not is_sqlite:
            if (
                not isinstance(config['host'], str)
                or not config['host'].strip()
            ):
                raise ValueError(
                    "O campo 'host' deve ser uma string não vazia."
                )

            if not isinstance(config['port'], int) or not (
                1 <= config['port'] <= 65535
            ):
                raise ValueError(
                    "O campo 'port' deve ser um número inteiro entre 1 e 65535."
                )

            if (
                not isinstance(config['username'], str)
                or not config['username'].strip()
            ):
                raise ValueError(
                    "O campo 'username' deve ser uma string não vazia."
                )

            if not isinstance(config['password'], str):
                raise ValueError("O campo 'password' deve ser uma string.")

        return config, None
