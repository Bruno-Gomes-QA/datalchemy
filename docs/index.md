![Datalchemy](assets/DATALCHEMY_.png){width="300"}

# **Datalchemy**


### **Simplificando a Geração de Dados Sintéticos Orientados por Semântica**

---

## 📌 **O que é?**

Datalchemy é uma biblioteca poderosa e intuitiva para facilitar a geração de dados sintéticos com base na estrutura do banco de dados do usuário. Ideal para desenvolvedores, cientistas de dados e equipes de QA que precisam criar dados consistentes, seguros e prontos para uso em testes ou protótipos.

---

## ✨ **Principais Funcionalidades**

### 📊 **Integração com Múltiplos Bancos de Dados**
Gerencie conexões com bancos SQL como MySQL, PostgreSQL, SQLite, entre outros, em poucos passos.

### 🛠️ **Geração de Modelos Automática**
Use o sqlacodegen para traduzir a estrutura do banco em modelos Python prontos para uso com SQLAlchemy.

### 🤖 **Assistente Semântico Alimentado por LLMs**
Solicite dados sintéticos diretamente por prompts em linguagem natural, garantindo consistência com as relações e constraints do banco.

### ⚙️ **Configuração e Uso Simplificados**
Configure múltiplas conexões e gere dados rapidamente por meio de uma interface intuitiva.

### 🔒 **Dados Seguros e Anonimizados**
Gera dados que seguem as melhores práticas de segurança e anonimização, atendendo a normas como LGPD e GDPR.

---

## 🚀 **Como Datalchemy Pode Te Ajudar?**

- **Testes Automatizados:** Gere cenários realistas com dados consistentes para validar aplicações sem acessar dados reais.
- **Desenvolvimento de Prototótipos:** Popule rapidamente bancos de dados de desenvolvimento ou sandbox.
- **Treinamento de Modelos de IA:** Crie dados sintéticos com características específicas para treinar seus modelos.
- **Análise de Dados:** Simule cenários completos sem interferir no ambiente de produção.

---

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
prompt = "Gere 10 registros para a tabela clientes, respeitando constraints."
response = generator.generate_data("main_db", prompt)
print(response)
```

### **Geração de Modelos**

Gere os modelos SQLAlchemy do banco de dados automaticamente:

```python
models_code = generator.generate_models("main_db", save_to_file=True)
print(models_code)
```

---

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

---

## 🔮 **Funcionalidades Futuras**

- **Geração em Larga Escala:** Suporte para grandes volumes de dados, otimizando o uso de tokens e recursos.
- **Validação Avançada:** Regras configuráveis para validar os dados antes de inseri-los no banco.
- **Suporte Expandido:** Integração com bancos de dados NoSQL.

---

## 📢 **Dicas para Maximizar o Uso**

- Use prompts claros e objetivos para obter dados relevantes e consistentes.
- Combine os dados gerados com ferramentas de visualização para entender melhor os cenários simulados.
- Explore a geração de modelos para documentar seu banco e facilitar futuras integrações.

---

✨ **Datalchemy: Simplifique sua jornada com dados sintéticos!**
