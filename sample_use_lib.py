import os

from dotenv import load_dotenv

from datalchemy import DatabaseConnectionManager, DataHandler, Generators

load_dotenv()

configs = [
    {
        'name': 'main_db',
        'dialect': 'mysql+pymysql',
        'username': 'brunom',
        'password': 'toor',
        'host': 'localhost',
        'port': 3306,
        'database': 'meu_banco',
    }
]

db_m = DatabaseConnectionManager(configs)
generator = Generators(db_m, 'bruno-gomes-qa/datalchemy-model')

res = generator.generate_data(
    'main_db',
    'Gere 10 produtos para 3 departamentos distintos',
)
print(res)
