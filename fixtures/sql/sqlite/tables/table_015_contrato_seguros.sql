CREATE TABLE IF NOT EXISTS contrato_seguros (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    id_contrato INTEGER NOT NULL,
    id_seguro INTEGER NOT NULL,
    FOREIGN KEY (id_contrato) REFERENCES contratos(id_contrato),
    FOREIGN KEY (id_seguro) REFERENCES seguros(id_seguro)
);
