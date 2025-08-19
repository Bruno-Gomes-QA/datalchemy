# Datalchemy

![Datalchemy](assets/DATALCHEMY_.png){width="300", class="center"}

## Simplificando a Geração de Dados Sintéticos Orientados por Semântica

---

## Introdução

Datalchemy é uma biblioteca para a geração de dados sintéticos com base na estrutura do banco de dados do usuário. É ideal para desenvolvedores, cientistas de dados e equipes de QA que precisam criar dados consistentes e seguros para uso em testes ou protótipos.

---

## Principais Funcionalidades

- **Integração com Múltiplos Bancos de Dados:** Gerenciamento de conexões com bancos SQL como MySQL, PostgreSQL, SQLite, entre outros.

- **Geração de Modelos Automática:** Utiliza o sqlacodegen para traduzir a estrutura do banco em modelos Python para uso com SQLAlchemy.

- **Assistente Semântico com LLMs:** Geração de dados sintéticos através de prompts em linguagem natural, garantindo consistência com as relações e constraints do banco.

- **Configuração e Uso Simplificados:** Interface intuitiva para configurar conexões e gerar dados.

- **Dados Seguros e Anonimizados:** Geração de dados que seguem as melhores práticas de segurança e anonimização, atendendo a normas como LGPD e GDPR.

---

## Casos de Uso

- **Testes Automatizados:** Gere cenários realistas com dados consistentes para validar aplicações sem acessar dados reais.
- **Desenvolvimento de Prototipos:** Popule rapidamente bancos de dados de desenvolvimento ou sandbox.
- **Treinamento de Modelos de IA:** Crie dados sintéticos com características específicas para treinar modelos.
- **Análise de Dados:** Simule cenários completos sem interferir no ambiente de produção.

---

## Como Começar

### Instalação

```bash
pip install datalchemy
```

### Configuração

Defina as configurações de conexão com seus bancos de dados:

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

Conecte-se a um LLM para gerar dados sintéticos com base em prompts:

```python
from datalchemy import Generators

generator = Generators(manager, OPENAI_API_KEY="sua_chave_aqui")
prompt = "Gere 10 registros para a tabela clientes, respeitando constraints."
response = generator.generate_data("main_db", prompt)
print(response)
```

### Geração de Modelos

Gere os modelos SQLAlchemy do banco de dados:

```python
models_code = generator.generate_models("main_db", save_to_file=True)
print(models_code)
```

---

## Roadmap

- **Geração em Larga Escala:** Suporte para grandes volumes de dados, com otimização do uso de tokens e recursos.
- **Validação Avançada:** Regras configuráveis para validar os dados antes da inserção no banco.
- **Suporte Expandido:** Integração com bancos de dados NoSQL.

---

## Dicas para Maximizar o Uso

- Utilize prompts claros e objetivos para obter dados mais relevantes.
- Combine os dados gerados com ferramentas de visualização para melhor compreensão dos cenários simulados.
- Explore a geração de modelos para documentar seu banco de dados e facilitar futuras integrações.
