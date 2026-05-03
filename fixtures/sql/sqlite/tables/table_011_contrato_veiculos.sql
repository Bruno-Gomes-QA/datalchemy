CREATE TABLE IF NOT EXISTS contrato_veiculos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    id_contrato INTEGER NOT NULL,
    id_veiculo INTEGER NOT NULL,
    valor_diaria REAL NOT NULL,
    FOREIGN KEY (id_contrato) REFERENCES contratos(id_contrato),
    FOREIGN KEY (id_veiculo) REFERENCES veiculos(id_veiculo)
);
