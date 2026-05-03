CREATE TABLE IF NOT EXISTS contratos (
    id_contrato INTEGER PRIMARY KEY AUTOINCREMENT,
    id_cliente INTEGER NOT NULL,
    id_usuario INTEGER NOT NULL,
    data_inicio DATE,
    data_fim_prevista DATE,
    data_fim_real DATE,
    status TEXT CHECK(status IN ('aberto','fechado','cancelado')) DEFAULT 'aberto',
    FOREIGN KEY (id_cliente) REFERENCES clientes(id_cliente),
    FOREIGN KEY (id_usuario) REFERENCES usuarios_sistema(id_usuario)
);
