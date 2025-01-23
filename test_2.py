from sdv.metadata.multi_table import MultiTableMetadata
from sdv.multi_table import HMASynthesizer
import pandas as pd

# Dados fictícios para a tabela Clientes
clientes_data = pd.DataFrame({
    "id_cliente": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    "nome": ["Ana", "Carlos", "Beatriz", "Daniel", "Eduarda", "Fernanda", "Gustavo", "Helena", "Igor", "Juliana"],
    "idade": [25, 34, 29, 40, 22, 31, 27, 35, 28, 30],
    "cidade": ["São Paulo", "Rio de Janeiro", "Curitiba", "Belo Horizonte", "Porto Alegre",
               "Fortaleza", "Brasília", "Recife", "Manaus", "Salvador"],
})


# Dados fictícios para a tabela Pedidos
pedidos_data = pd.DataFrame({
    "id_pedido": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    "id_cliente": [1, 1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 9, 10, 10, 10],
    "valor": [150.50, 200.00, 300.00, 120.00, 450.00, 180.75, 220.30, 90.60, 75.25, 310.50, 140.90, 230.10, 320.40, 250.00, 400.00],
    "data_pedido": [
        "2023-01-10", "2023-01-11", "2023-01-15", "2023-02-01", "2023-02-05", 
        "2023-02-10", "2023-02-15", "2023-02-20", "2023-02-25", "2023-03-01",
        "2023-03-05", "2023-03-10", "2023-03-15", "2023-03-20", "2023-03-25",
    ],
})


# Criar metadata
metadata = MultiTableMetadata()

# Adicionar as tabelas
metadata.add_table("clientes")
metadata.add_table("pedidos")

# Adicionar colunas à tabela Clientes
metadata.add_column("clientes", "id_cliente", sdtype="id")
metadata.add_column("clientes", "nome", sdtype="text")
metadata.add_column("clientes", "idade", sdtype="numerical")
metadata.add_column("clientes", "cidade", sdtype="categorical")

# Configurar a chave primária da tabela Clientes
metadata.set_primary_key("clientes", "id_cliente")

# Adicionar colunas à tabela Pedidos
metadata.add_column("pedidos", "id_pedido", sdtype="id")
metadata.add_column("pedidos", "id_cliente", sdtype="id")
metadata.add_column("pedidos", "valor", sdtype="numerical")
metadata.add_column("pedidos", "data_pedido", sdtype="datetime", datetime_format="%Y-%m-%d")

# Configurar a chave primária e o relacionamento
metadata.set_primary_key("pedidos", "id_pedido")
metadata.add_relationship(
    parent_table_name="clientes",
    child_table_name="pedidos",
    parent_primary_key="id_cliente",
    child_foreign_key="id_cliente"
)

# Inicializar o sintetizador
synthesizer = HMASynthesizer(metadata)

# Treinar o sintetizador com os dados
synthesizer.fit({
    "clientes": clientes_data,
    "pedidos": pedidos_data
})

# Gerar dados sintéticos
synthetic_data = synthesizer.sample()

# Exibir os resultados
print("Clientes Sintéticos:")
print(synthetic_data["clientes"])
print("\nPedidos Sintéticos:")
print(synthetic_data["pedidos"])
