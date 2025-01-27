# **Datalchemy** <img src="https://datalchemy.readthedocs.io/en/latest/assets/DATALCHEMY_.png" width="100">

[![CI](https://github.com/Bruno-Gomes-QA/datalchemy/actions/workflows/pipeline.yaml/badge.svg)](https://github.com/Bruno-Gomes-QA/datalchemy/actions/workflows/pipeline.yaml)
[![Documentation Status](https://readthedocs.org/projects/datalchemy/badge/?version=latest)](https://datalchemy.readthedocs.io/en/latest/?badge=latest)
[![codecov](https://codecov.io/gh/Bruno-Gomes-QA/datalchemy/graph/badge.svg?token=sYf3a0mhbR)](https://codecov.io/gh/Bruno-Gomes-QA/datalchemy)

📌 **Datalchemy** é uma biblioteca intuitiva projetada para facilitar a geração de dados sintéticos com base na estrutura do banco de dados do usuário. Atualmente, a ferramenta é ideal para **prototipagem de aplicações** e para **estudantes** que desejam realizar testes com dados consistentes e realistas, mas em menor escala.

## ✨ **Principais Funcionalidades**

### 📊 **Facilidade de Conexão com Bancos de Dados**
Gerencie conexões com bancos SQL como MySQL, PostgreSQL, SQLite, entre outros, em poucos passos.

### 🛠️ **Exploração e Modelagem de Banco**
Use o sqlacodegen para traduzir automaticamente a estrutura do banco em modelos SQLAlchemy.

### 🤖 **Assistente Baseado em LLMs**
Solicite dados sintéticos diretamente por prompts, garantindo que as relações e constraints do banco sejam respeitadas.

### ⚙️ **Uso Simples e Intuitivo**
Uma interface que facilita o uso, desde a configuração de conexões até a geração de dados.

### 🔒 **Dados Seguros e Consistentes**
Os dados gerados seguem boas práticas de segurança e coerência, respeitando constraints definidas no banco de dados.

## 🚀 **Como Datalchemy Pode Te Ajudar?**

- **Prototipagem de Aplicações:** Popule rapidamente bancos de dados de desenvolvimento com dados iniciais consistentes.
- **Ensino e Aprendizado:** Ofereça uma maneira simples de estudantes explorarem conceitos de bancos de dados e programação.
- **Testes Automatizados:** Crie cenários simples e confiáveis para validar funcionalidades.

## 🛠️ **Como Começar?**

### **Instalação**
```bash
pip install datalchemy
```

### **Configuração**
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

### **Geração de Dados**
Conecte-se à LLM para gerar dados sintéticos com base em prompts:

```python
from datalchemy import Generators

generator = Generators(manager, OPENAI_API_KEY="sua_chave_aqui")
prompt = "Gere 10 registros para a tabela clientes."
response = generator.generate_data("main_db", prompt)
print(response)
```

### **Geração de Modelos**
Gere os modelos SQLAlchemy do banco de dados automaticamente:

```python
models_code = generator.generate_models("main_db", save_to_file=True)
print(models_code)
```

## 📚 **Exemplos e Casos de Uso**

### **Prototipagem Simples**
Gere poucos dados para tabelas relacionadas:

```python
prompt = "Gere 5 registros para cada tabela do banco de dados."
print(generator.generate_data("main_db", prompt))
```

### **Exploração de Estrutura**
Exporte os modelos SQLAlchemy para entender e documentar a estrutura do banco:

```python
generator.generate_models("main_db", save_to_file=True)
```

## 🔮 **Funcionalidades Futuras**
- **Geração em Larga Escala:** Suporte para geração de grandes volumes de dados, otimizando o uso de tokens e recursos.
- **Validação Avançada:** Regras configuráveis para validar os dados antes de inseri-los no banco.
- **Suporte Expandido:** Integração com bancos de dados NoSQL.

## 📢 **Dicas para Maximizar o Uso**
- Use prompts claros e objetivos para obter dados relevantes e consistentes.
- Combine os dados gerados com ferramentas de visualização para entender melhor os cenários simulados.
- Explore a geração de modelos para documentar seu banco e facilitar futuras integrações.
