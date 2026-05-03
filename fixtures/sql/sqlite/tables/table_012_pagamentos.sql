CREATE TABLE IF NOT EXISTS pagamentos (
    id_pagamento INTEGER PRIMARY KEY AUTOINCREMENT,
    id_contrato INTEGER NOT NULL,
    valor REAL NOT NULL,
    data_pagamento DATE,
    metodo TEXT,
    status TEXT,
    FOREIGN KEY (id_contrato) REFERENCES contratos(id_contrato)
);
