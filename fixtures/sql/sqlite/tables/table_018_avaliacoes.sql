CREATE TABLE IF NOT EXISTS avaliacoes (
    id_avaliacao INTEGER PRIMARY KEY AUTOINCREMENT,
    id_contrato INTEGER NOT NULL,
    nota INTEGER CHECK(nota BETWEEN 1 AND 5),
    comentario TEXT,
    FOREIGN KEY (id_contrato) REFERENCES contratos(id_contrato)
);
