CREATE TABLE IF NOT EXISTS veiculos (
    id_veiculo INTEGER PRIMARY KEY AUTOINCREMENT,
    id_modelo INTEGER NOT NULL,
    placa TEXT UNIQUE NOT NULL,
    ano INTEGER,
    cor TEXT,
    km_atual INTEGER DEFAULT 0,
    status TEXT CHECK(status IN ('disponivel','alugado','manutencao')) DEFAULT 'disponivel',
    FOREIGN KEY (id_modelo) REFERENCES modelos(id_modelo)
);
