CREATE TABLE IF NOT EXISTS enderecos_clientes (
    id_endereco INTEGER PRIMARY KEY AUTOINCREMENT,
    id_cliente INTEGER NOT NULL,
    rua TEXT,
    cidade TEXT,
    estado TEXT,
    cep TEXT,
    FOREIGN KEY (id_cliente) REFERENCES clientes(id_cliente)
);
