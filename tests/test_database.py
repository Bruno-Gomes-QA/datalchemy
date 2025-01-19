from unittest.mock import patch

import pytest

from datalchemy import DatabaseConnectionManager


@pytest.fixture
def db_configs():
    """Fixture com configurações para os testes."""
    return [
        {
            'name': 'test_db_1',
            'dialect': 'sqlite',
            'database': ':memory:',
        },
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
    config = db_configs[1]

    manager.add_connection(config)

    assert len(manager.connections) == 1
    assert 'test_db_2' in manager.connections


def test_add_connection_missing_keys():
    """Testa se uma exceção é levantada ao adicionar conexão com configuração incompleta."""
    manager = DatabaseConnectionManager([])

    incomplete_config = {
        'name': 'invalid_db',
        'dialect': 'sqlite',
    }

    with pytest.raises(
        ValueError,
        match='Configuração inválida, valores ausentes ou vazios: database',
    ):
        manager.add_connection(incomplete_config)


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
    mysql_config = {
        'dialect': 'mysql+pymysql',
        'username': 'user',
        'password': 'pass',
        'host': 'localhost',
        'port': 3306,
        'database': 'testdb',
    }
    mysql_url = 'mysql+pymysql://user:pass@localhost:3306/testdb'
    assert (
        DatabaseConnectionManager.build_connection_url(mysql_config)
        == mysql_url
    )

    sqlite_config = {
        'dialect': 'sqlite',
        'database': ':memory:',
    }
    sqlite_url = 'sqlite:///:memory:'
    assert (
        DatabaseConnectionManager.build_connection_url(sqlite_config)
        == sqlite_url
    )

    postgres_config = {
        'dialect': 'postgresql',
        'username': 'admin',
        'password': 'admin123',
        'host': 'localhost',
        'port': 5432,
        'database': 'postgres_db',
    }
    postgres_url = 'postgresql://admin:admin123@localhost:5432/postgres_db'
    assert (
        DatabaseConnectionManager.build_connection_url(postgres_config)
        == postgres_url
    )
