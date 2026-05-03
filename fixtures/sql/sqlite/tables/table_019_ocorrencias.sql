CREATE TABLE IF NOT EXISTS ocorrencias (
    id_ocorrencia INTEGER PRIMARY KEY AUTOINCREMENT,
    id_contrato INTEGER NOT NULL,
    descricao TEXT,
    data_registro DATE DEFAULT CURRENT_DATE,
    FOREIGN KEY (id_contrato) REFERENCES contratos(id_contrato)
);
