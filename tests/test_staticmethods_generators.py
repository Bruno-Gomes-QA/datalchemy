import unittest
from unittest.mock import MagicMock

from sqlalchemy.engine.base import Engine

from datalchemy import Generators


class TestGeneratorsStaticMethods(unittest.TestCase):
    def setUp(self):
        self.engine_mock = MagicMock(spec=Engine)

    def test_get_metadata(self):
        # Mockando o comportamento do inspector
        inspector_mock = MagicMock()
        inspector_mock.get_table_names.return_value = ['table1', 'table2']
        inspector_mock.get_columns.side_effect = [
            [{'name': 'id', 'type': 'INTEGER', 'nullable': False}],
            [{'name': 'name', 'type': 'VARCHAR', 'nullable': True}],
        ]
        inspector_mock.get_foreign_keys.side_effect = [
            [
                {
                    'constrained_columns': ['id'],
                    'referred_table': 'table2',
                    'referred_columns': ['id'],
                }
            ],
            [],
        ]

        with unittest.mock.patch(
            'datalchemy.generators.inspect', return_value=inspector_mock
        ):
            metadata = Generators.get_metadata(self.engine_mock)

        expected_metadata = {
            'table1': {
                'columns': [
                    {'name': 'id', 'type': 'INTEGER', 'nullable': False}
                ],
                'foreign_keys': [
                    {
                        'column': ['id'],
                        'referenced_table': 'table2',
                        'referenced_column': ['id'],
                    }
                ],
            },
            'table2': {
                'columns': [
                    {'name': 'name', 'type': 'VARCHAR', 'nullable': True}
                ],
                'foreign_keys': [],
            },
        }
        self.assertEqual(metadata, expected_metadata)

    def test_get_parental_tables(self):
        llm_response = '{"tables": ["table2"]}'
        db_structure = {
            'table1': {'foreign_keys': []},
            'table2': {
                'foreign_keys': [
                    {
                        'column': ['id'],
                        'referenced_table': 'table1',
                        'referenced_column': ['id'],
                    }
                ]
            },
        }
        updated_tables = Generators.get_parental_tables(
            llm_response, db_structure
        )
        self.assertEqual(updated_tables, ['table1', 'table2'])

    def test_count_tokens(self):
        mock_encoding = MagicMock()
        mock_encoding.encode.return_value = [1, 2, 3, 4, 5]

        with unittest.mock.patch(
            'datalchemy.generators.tiktoken.encoding_for_model',
            return_value=mock_encoding,
        ):
            token_count = Generators.count_tokens(
                'Test message', model='gpt-3.5-turbo-16k'
            )

        self.assertEqual(token_count, 5)

    def test_filter_tables(self):
        db_structure = {
            'table1': {'columns': [], 'foreign_keys': []},
            'table2': {'columns': [], 'foreign_keys': []},
        }
        tables_to_keep = ['table1']
        filtered_structure = Generators.filter_tables(
            db_structure, tables_to_keep
        )
        expected_structure = {'table1': {'columns': [], 'foreign_keys': []}}
        self.assertEqual(filtered_structure, expected_structure)

    def test_read_prompts(self):
        prompts = Generators.read_prompts()
        self.assertIn('get_tables_in_user_prompt', prompts)
        self.assertIn('data_generation_rules', prompts)
