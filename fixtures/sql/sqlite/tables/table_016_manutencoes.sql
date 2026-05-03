CREATE TABLE IF NOT EXISTS manutencoes (
    id_manutencao INTEGER PRIMARY KEY AUTOINCREMENT,
    id_veiculo INTEGER NOT NULL,
    data_inicio DATE,
    data_fim DATE,
    tipo TEXT,
    custo REAL,
    FOREIGN KEY (id_veiculo) REFERENCES veiculos(id_veiculo)
);
