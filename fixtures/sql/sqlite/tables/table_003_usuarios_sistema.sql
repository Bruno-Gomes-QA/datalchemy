CREATE TABLE IF NOT EXISTS usuarios_sistema (
    id_usuario INTEGER PRIMARY KEY AUTOINCREMENT,
    nome TEXT NOT NULL,
    email TEXT UNIQUE,
    senha_hash TEXT NOT NULL,
    perfil TEXT CHECK(perfil IN ('admin','atendente')),
    ativo INTEGER DEFAULT 1
);
