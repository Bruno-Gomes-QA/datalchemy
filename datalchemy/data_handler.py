import json

from sqlalchemy import MetaData, Table


class DataHandler:
    def __init__(self, engine):
        """
        Inicializa o orquestrador de dados automáticos.

        Args:
            engine: Instância do SQLAlchemy engine conectada ao banco.
        """
        self.engine = engine
        self.metadata = MetaData(bind=self.engine)
        self.metadata.reflect()  # Reflete as tabelas existentes no banco

    def insert(self, json_data):
        """
        Insere dados no banco de dados com base no JSON fornecido.

        Args:
            json_data (dict | str): Estrutura JSON contendo as tabelas, colunas e valores a serem inseridos.

        Raises:
            ValueError: Caso alguma tabela no JSON não seja encontrada no banco.
        """
        # Converte JSON string para dicionário, se necessário
        if isinstance(json_data, str):
            json_data = json.loads(json_data)

        with self.engine.connect() as connection:
            transaction = connection.begin()
            try:
                for table_name, content in json_data.items():
                    # Busca a tabela nos metadados
                    table = self.metadata.tables.get(table_name)
                    if table is None:
                        raise ValueError(
                            f"Tabela '{table_name}' não encontrada no banco de dados."
                        )

                    # Extrai os atributos (colunas) e valores
                    attributes = content['atributos']
                    values = content['valores']

                    # Insere os dados na tabela
                    for value_set in values:
                        row_data = dict(
                            zip(attributes, value_set)
                        )  # Mapeia colunas para valores
                        connection.execute(table.insert(), row_data)

                # Confirma a transação
                transaction.commit()
                print('Dados inseridos com sucesso!')

            except Exception as e:
                transaction.rollback()
                print(f'Erro ao inserir dados: {e}')
                raise
