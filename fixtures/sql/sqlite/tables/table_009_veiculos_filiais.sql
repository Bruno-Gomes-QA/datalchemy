CREATE TABLE IF NOT EXISTS veiculos_filiais (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    id_veiculo INTEGER NOT NULL,
    id_filial INTEGER NOT NULL,
    data_alocacao DATE,
    FOREIGN KEY (id_veiculo) REFERENCES veiculos(id_veiculo),
    FOREIGN KEY (id_filial) REFERENCES filiais(id_filial)
);
