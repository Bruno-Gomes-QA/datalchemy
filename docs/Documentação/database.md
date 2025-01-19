::: database

## Bancos de Dados Suportados
Atualmente, a classe suporta os seguintes bancos de dados:
- SQLite
- MySQL
- PostgreSQL

Adicionalmente, aqui estão as configurações e instruções para Oracle e MariaDB, mesmo que não estejam totalmente suportados.

---

## Configuração para cada Banco de Dados

### 1. **SQLite**
- **Driver Recomendado**: Nativo no Python (não requer instalação).
- **Configuração de Exemplo**:

```python
{
    'name': 'sqlite_db',
    'dialect': 'sqlite',
    'database': ':memory:'  # Para um banco em memória
}
```

### 2. **MySQL**
- **Driver Recomendado**: `pymysql` (instale com `pip install pymysql`).
- **Configuração de Exemplo**:
```python
{
    'name': 'mysql_db',
    'dialect': 'mysql+pymysql',
    'username': 'root',
    'password': 'password',
    'host': 'localhost',
    'port': 3306,
    'database': 'meu_banco'
}
```

### 3. **PostgreSQL**
- **Driver Recomendado**: `psycopg2-binary` (instale com `pip install psycopg2-binary`).
- **Configuração de Exemplo**:
```python
{
    'name': 'postgres_db',
    'dialect': 'postgresql',
    'username': 'admin',
    'password': 'admin123',
    'host': 'localhost',
    'port': 5432,
    'database': 'meu_banco'
}
```

### 4. **Oracle**
- **Driver Recomendado**: `cx_Oracle` (instale com `pip install cx_Oracle`).  
  **Nota**: Requer instalação das [Oracle Instant Client Libraries](https://www.oracle.com/database/technologies/instant-client.html).
- **Configuração de Exemplo**:
```python
{
    'name': 'oracle_db',
    'dialect': 'oracle',
    'username': 'admin',
    'password': 'oracle123',
    'host': 'localhost',
    'port': 1521,
    'database': 'XE'  # Nome do serviço ou SID
}
```

### 5. **MariaDB**
- **Driver Recomendado**: `mariadb` (instale com `pip install mariadb`).
- **Configuração de Exemplo**:
```python
{
    'name': 'mariadb_db',
    'dialect': 'mariadb',
    'username': 'root',
    'password': 'password',
    'host': 'localhost',
    'port': 3306,
    'database': 'meu_banco'
}
```

---

## Detalhes Importantes
1. **Escolha do Driver**: Certifique-se de instalar o driver correto para cada banco antes de usar o `DatabaseConnectionManager`.
2. **Erros Comuns**: Confira as permissões do usuário e certifique-se de que o servidor do banco de dados esteja acessível.
3. **Formato de Conexão**: A URL de conexão é construída automaticamente pela classe, mas pode ser ajustada, se necessário.

---

Para dúvidas ou mais exemplos, consulte a documentação oficial dos drivers ou entre em contato com o desenvolvedor da classe.
