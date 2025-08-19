# Datalchemy

[![CI](https://github.com/Bruno-Gomes-QA/datalchemy/actions/workflows/pipeline.yaml/badge.svg)](https://github.com/Bruno-Gomes-QA/datalchemy/actions/workflows/pipeline.yaml)
[![Documentation Status](https://readthedocs.org/projects/datalchemy/badge/?version=latest)](https://datalchemy.readthedocs.io/en/latest/?badge=latest)
[![codecov](https://codecov.io/gh/Bruno-Gomes-QA/datalchemy/graph/badge.svg?token=sYf3a0mhbR)](https://codecov.io/gh/Bruno-Gomes-QA/datalchemy)
[![PyPI - Version](https://img.shields.io/pypi/v/datalchemy.svg?logo=pypi&label=PyPI&logoColor=gold)](https://pypi.org/project/datalchemy/)

**Datalchemy** é uma biblioteca para a geração de dados sintéticos baseada na estrutura de bancos de dados relacionais. É uma ferramenta voltada para a prototipagem de aplicações e para fins educacionais, permitindo a criação de dados consistentes para testes em pequena escala.

## Principais Funcionalidades

- **Conexão com Bancos de Dados:** Gerenciamento de conexões com bancos de dados SQL como MySQL, PostgreSQL, SQLite, entre outros.

- **Modelagem de Banco de Dados:** Geração automática de modelos SQLAlchemy a partir da estrutura do banco de dados existente.

- **Assistente de Geração de Dados:** Utiliza LLMs para gerar dados sintéticos através de prompts em linguagem natural, respeitando as relações e constraints do banco.

- **Interface Simplificada:** Oferece uma interface intuitiva para configuração e geração de dados.

- **Consistência e Segurança:** Garante a geração de dados consistentes e anônimos, em conformidade com as constraints do banco.

## Casos de Uso Principais

- **Prototipagem de Aplicações:** Popule rapidamente bancos de dados de desenvolvimento com dados consistentes.
- **Ensino e Aprendizado:** Facilita a exploração de conceitos de banco de dados e programação em ambientes de aprendizado.
- **Testes de Funcionalidade:** Permite a criação de cenários de teste simples e confiáveis para a validação de funcionalidades.

## Como Começar

### Instalação
```bash
pip install datalchemy
```

### Configuração
Defina as configurações de conexão com seu banco de dados:

```python
from datalchemy import DatabaseConnectionManager

configs = [
    {
        'name': 'main_db',
        'dialect': 'mysql+pymysql',
        'username': 'seu_usuario',
        'password': 'sua_senha',
        'host': 'localhost',
        'port': 3306,
        'database': 'meu_banco',
    }
]

manager = DatabaseConnectionManager(configs)
```

### Geração de Dados
Utilize um LLM para gerar dados sintéticos com base em prompts:

```python
from datalchemy import Generators

generator = Generators(manager, OPENAI_API_KEY="sua_chave_aqui")
prompt = "Gere 10 produtos para 3 departamentos diferentes, relacionados ao setor de tecnologia."
response = generator.generate_data("main_db", prompt)
print(response)
```

### Inserindo os Dados
Após a geração, os dados podem ser inseridos no banco de dados:

```python
from datalchemy import DataHandler
handler = DataHandler(manager.get_engine())
handler.insert(response)
```

### Geração de Modelos
Gere os modelos SQLAlchemy do banco de dados:

```python
models_code = generator.generate_models("main_db", save_to_file=True)
print(models_code)
```

## Roadmap

- **Geração em Larga Escala:** Suporte para geração de grandes volumes de dados.
- **Validação Avançada:** Implementação de regras configuráveis para validação dos dados.
- **Suporte a NoSQL:** Expansão da compatibilidade para bancos de dados NoSQL.
- **LLMs Locais:** Integração com modelos de linguagem open source e personalizáveis.
