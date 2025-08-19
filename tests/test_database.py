from unittest.mock import patch

import pytest

from datalchemy import DatabaseConfig, DatabaseConnectionManager


@pytest.fixture
def db_configs():
    """Fixture com configurações para os testes."""
    return [
        {'name': 'test_db_1', 'dialect': 'sqlite', 'database': ':memory:'},
        {
            'name': 'test_db_2',
            'dialect': 'mysql+pymysql',
            'username': 'root',
            'password': 'toor',
            'host': 'localhost',
            'port': 3306,
            'database': 'test_db',
        },
        {
            'name': 'test_db_3',
            'dialect': 'postgresql',
            'username': 'admin',
            'password': 'admin123',
            'host': 'localhost',
            'port': 5432,
            'database': 'postgres_db',
        },
    ]


def test_initialization_creates_connections(db_configs):
    """Testa se as conexões são inicializadas corretamente."""
    manager = DatabaseConnectionManager(db_configs)

    assert len(manager.connections) == 3
    assert 'test_db_1' in manager.connections
    assert 'test_db_2' in manager.connections
    assert 'test_db_3' in manager.connections


def test_add_connection(db_configs):
    """Testa a adição de uma nova conexão."""
    manager = DatabaseConnectionManager([])
    config = DatabaseConfig(**db_configs[1])

    manager.add_connection(config)

    assert len(manager.connections) == 1
    assert 'test_db_2' in manager.connections


def test_add_connection_missing_keys():
    """Testa se uma exceção é levantada ao adicionar conexão com configuração incompleta."""
    incomplete_config = {
        'name': 'invalid_db',
        'dialect': 'sqlite',
        # Missing 'database'
    }

    with pytest.raises(ValueError) as exc_info:
        DatabaseConnectionManager([incomplete_config])

    assert 'database' in str(exc_info.value)


def test_add_connection_password_type_error():
    """Testa se uma exceção é levantada ao adicionar conexão com tipo inválido."""
    type_error_config = {
        'name': 'test_db_3',
        'dialect': 'postgresql',
        'username': 'admin',
        'password': 123,  # Invalid type (int instead of str)
        'host': 'localhost',
        'port': 5432,
        'database': 'postgres_db',
    }

    with pytest.raises(ValueError) as exc_info:
        DatabaseConnectionManager([type_error_config])

    assert 'password' in str(exc_info.value).lower()


def test_get_session(db_configs):
    """Testa a recuperação de uma sessão válida."""
    manager = DatabaseConnectionManager(db_configs)

    session = manager.get_session('test_db_1')
    assert session is not None


def test_get_session_invalid_name(db_configs):
    """Testa se uma exceção é levantada ao buscar uma sessão de uma conexão inexistente."""
    manager = DatabaseConnectionManager(db_configs)

    with pytest.raises(
        ValueError, match="Conexão 'invalid_db' não encontrada."
    ):
        manager.get_session('invalid_db')


def test_close_all_connections(db_configs):
    """Testa se todas as conexões são fechadas corretamente."""
    manager = DatabaseConnectionManager(db_configs)

    assert len(manager.connections) == 3
    manager.close_all_connections()
    assert len(manager.connections) == 0


def test_build_connection_url():
    """Testa se a URL de conexão é construída corretamente."""
    manager = DatabaseConnectionManager([])

    mysql_config = DatabaseConfig(
        name='test_mysql',
        dialect='mysql+pymysql',
        username='user',
        password='pass',
        host='localhost',
        port=3306,
        database='testdb',
    )
    mysql_url = 'mysql+pymysql://user:pass@localhost:3306/testdb'
    assert str(manager._build_connection_url(mysql_config)) == mysql_url

    sqlite_config = DatabaseConfig(
        name='test_sqlite',
        dialect='sqlite',
        database=':memory:',
    )
    sqlite_url = 'sqlite:///:memory:'
    assert str(manager._build_connection_url(sqlite_config)) == sqlite_url

    postgres_config = DatabaseConfig(
        name='test_postgres',
        dialect='postgresql',
        username='admin',
        password='admin123',
        host='localhost',
        port=5432,
        database='postgres_db',
    )
    postgres_url = 'postgresql://admin:admin123@localhost:5432/postgres_db'
    assert str(manager._build_connection_url(postgres_config)) == postgres_url


def test_duplicate_connection_name_error(db_configs):
    """Testa erro ao adicionar conexão com nome duplicado."""
    manager = DatabaseConnectionManager(db_configs[:1])
    duplicate_config = DatabaseConfig(**db_configs[0])

    with pytest.raises(ValueError) as exc_info:
        manager.add_connection(duplicate_config)

    assert 'já existe' in str(exc_info.value).lower()


def test_context_manager_closes_connections(db_configs):
    """Testa se o gerenciador de contexto fecha conexões automaticamente."""
    with DatabaseConnectionManager(db_configs) as manager:
        assert len(manager.connections) == 3

    assert len(manager.connections) == 0


def test_pool_size_validation():
    """Testa validação de pool_size inválido."""
    invalid_config = {
        'name': 'test_db',
        'dialect': 'sqlite',
        'database': ':memory:',
        'pool_size': 0,
    }

    with pytest.raises(ValueError) as exc_info:
        DatabaseConnectionManager([invalid_config])

    assert 'pool_size' in str(exc_info.value)


def test_missing_driver_error():
    """Testa erro quando driver necessário não está instalado."""
    config = {
        'name': 'pg_db',
        'dialect': 'postgresql+psycopg2',
        'username': 'user',
        'password': 'pass',
        'host': 'localhost',
        'database': 'test',
    }

    with patch('datalchemy.database.util.find_spec', return_value=None):
        with pytest.raises(ImportError) as exc_info:
            DatabaseConnectionManager([config])

    assert 'psycopg2' in str(exc_info.value)
    assert 'pip install datalchemy[postgresql]' in str(exc_info.value)


def test_session_context_manager(db_configs):
    """Testa uso da sessão dentro de um bloco with."""
    manager = DatabaseConnectionManager(db_configs)

    with manager.get_session('test_db_1') as session:
        result = session.execute('SELECT 1')
        assert result.scalar() == 1


def test_port_validation():
    """Testa validação de porta inválida."""
    invalid_config = {
        'name': 'invalid_port',
        'dialect': 'mysql',
        'username': 'root',
        'password': 'pass',
        'host': 'localhost',
        'port': 70000,
        'database': 'test',
    }

    with pytest.raises(ValueError) as exc_info:
        DatabaseConnectionManager([invalid_config])

    assert 'port' in str(exc_info.value)


def test_sqlite_minimal_config():
    """Testa configuração mínima válida para SQLite."""
    config = {
        'name': 'minimal_sqlite',
        'dialect': 'sqlite',
        'database': '/tmp/test.db',
    }

    manager = DatabaseConnectionManager([config])
    assert 'minimal_sqlite' in manager.connections


def test_invalid_dialect_handling():
    """Testa tratamento de dialeto não suportado."""
    config = {
        'name': 'unknown_db',
        'dialect': 'firebird',
        'database': 'test.fdb',
    }

    with pytest.raises(ImportError):
        DatabaseConnectionManager([config])
