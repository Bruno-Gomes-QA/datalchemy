CREATE TABLE IF NOT EXISTS clientes (
    id_cliente INTEGER PRIMARY KEY AUTOINCREMENT,
    nome TEXT NOT NULL,
    cpf TEXT UNIQUE,
    email TEXT,
    telefone TEXT,
    data_nascimento DATE,
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP
);
