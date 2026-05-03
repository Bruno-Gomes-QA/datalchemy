CREATE TABLE IF NOT EXISTS historico_km (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    id_veiculo INTEGER NOT NULL,
    km INTEGER NOT NULL,
    data_registro DATE DEFAULT CURRENT_DATE,
    FOREIGN KEY (id_veiculo) REFERENCES veiculos(id_veiculo)
);
