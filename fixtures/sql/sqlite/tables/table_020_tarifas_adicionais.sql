CREATE TABLE IF NOT EXISTS tarifas_adicionais (
    id_tarifa INTEGER PRIMARY KEY AUTOINCREMENT,
    descricao TEXT NOT NULL,
    valor REAL NOT NULL
);
