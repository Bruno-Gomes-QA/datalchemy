CREATE TABLE IF NOT EXISTS multas (
    id_multa INTEGER PRIMARY KEY AUTOINCREMENT,
    id_contrato INTEGER NOT NULL,
    descricao TEXT,
    valor REAL NOT NULL,
    FOREIGN KEY (id_contrato) REFERENCES contratos(id_contrato)
);
