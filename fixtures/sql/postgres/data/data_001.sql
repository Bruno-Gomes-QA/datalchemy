insert into crm.usuarios (id, nome, email, telefone)
values
  ('11111111-1111-1111-1111-111111111111', 'Ana Lima', 'ana.lima@crm.local', '11999990001'),
  ('22222222-2222-2222-2222-222222222222', 'Bruno Souza', 'bruno.souza@crm.local', '11999990002');

insert into crm.empresas (id, razao_social, nome_fantasia, cnpj, email, telefone, site)
values
  ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'Empresa Alpha LTDA', 'Alpha', '12345678000100', 'contato@alpha.local', '1130000001', 'https://alpha.local'),
  ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'Empresa Beta SA', 'Beta', '22345678000100', 'contato@beta.local', '1130000002', 'https://beta.local');

insert into crm.contatos (id, empresa_id, nome, sobrenome, email, telefone, cargo, data_nascimento)
values
  ('cccccccc-cccc-cccc-cccc-cccccccccccc', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'Carlos', 'Silva', 'carlos.silva@alpha.local', '11990000001', 'Compras', '1988-05-20'),
  ('dddddddd-dddd-dddd-dddd-dddddddddddd', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'Daniela', 'Oliveira', 'daniela.oliveira@beta.local', '11990000002', 'Financeiro', '1990-08-15');

insert into crm.fontes_lead (id, nome, descricao)
values
  ('eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee', 'Formulario Site', 'Leads vindos do site'),
  ('ffffffff-ffff-ffff-ffff-ffffffffffff', 'Indicacao', 'Leads por indicacao');

insert into crm.leads (id, fonte_lead_id, contato_id, empresa_id, responsavel_id, status, score, data_qualificacao)
values
  ('10101010-1010-1010-1010-101010101010', 'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee', 'cccccccc-cccc-cccc-cccc-cccccccccccc', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', '11111111-1111-1111-1111-111111111111', 'qualificado', 80, now()),
  ('20202020-2020-2020-2020-202020202020', 'ffffffff-ffff-ffff-ffff-ffffffffffff', 'dddddddd-dddd-dddd-dddd-dddddddddddd', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', '22222222-2222-2222-2222-222222222222', 'novo', 30, null);

insert into crm.funis (id, nome, descricao)
values
  ('30303030-3030-3030-3030-303030303030', 'Funil Principal', 'Processo padrao');

insert into crm.etapas_funil (id, funil_id, nome, ordem, probabilidade)
values
  ('40404040-4040-4040-4040-404040404040', '30303030-3030-3030-3030-303030303030', 'Prospeccao', 1, 10.00),
  ('50505050-5050-5050-5050-505050505050', '30303030-3030-3030-3030-303030303030', 'Proposta', 2, 50.00),
  ('60606060-6060-6060-6060-606060606060', '30303030-3030-3030-3030-303030303030', 'Fechamento', 3, 90.00);

insert into crm.oportunidades (id, empresa_id, contato_id, etapa_id, responsavel_id, titulo, valor_estimado, status, data_abertura)
values
  ('70707070-7070-7070-7070-707070707070', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'cccccccc-cccc-cccc-cccc-cccccccccccc', '50505050-5050-5050-5050-505050505050', '11111111-1111-1111-1111-111111111111', 'Contrato Alpha 2024', 15000.00, 'aberta', current_date),
  ('80808080-8080-8080-8080-808080808080', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'dddddddd-dddd-dddd-dddd-dddddddddddd', '40404040-4040-4040-4040-404040404040', '22222222-2222-2222-2222-222222222222', 'Projeto Beta', 8000.00, 'aberta', current_date);

insert into crm.produtos (id, sku, nome, descricao, preco_base)
values
  ('90909090-9090-9090-9090-909090909090', 'PROD-001', 'CRM Basico', 'Licenca mensal', 299.90),
  ('a0a0a0a0-a0a0-a0a0-a0a0-a0a0a0a0a0a0', 'PROD-002', 'CRM Premium', 'Licenca anual', 2999.00);

insert into crm.listas_precos (id, nome, moeda, data_inicio, data_fim)
values
  ('b0b0b0b0-b0b0-b0b0-b0b0-b0b0b0b0b0b0', 'Tabela Padrao', 'BRL', '2024-01-01', null);

insert into crm.itens_lista_precos (id, lista_precos_id, produto_id, preco)
values
  ('c0c0c0c0-c0c0-c0c0-c0c0-c0c0c0c0c0c0', 'b0b0b0b0-b0b0-b0b0-b0b0-b0b0b0b0b0b0', '90909090-9090-9090-9090-909090909090', 279.90),
  ('d0d0d0d0-d0d0-d0d0-d0d0-d0d0d0d0d0d0', 'b0b0b0b0-b0b0-b0b0-b0b0-b0b0b0b0b0b0', 'a0a0a0a0-a0a0-a0a0-a0a0-a0a0a0a0a0a0', 2899.00);

insert into crm.cotacoes (id, oportunidade_id, responsavel_id, codigo, data_emissao, validade)
values
  ('e0e0e0e0-e0e0-e0e0-e0e0-e0e0e0e0e0e0', '70707070-7070-7070-7070-707070707070', '11111111-1111-1111-1111-111111111111', 'COT-0001', current_date, current_date + interval '15 days');

insert into crm.itens_cotacao (id, cotacao_id, produto_id, quantidade, preco_unitario, desconto_percentual)
values
  ('f0f0f0f0-f0f0-f0f0-f0f0-f0f0f0f0f0f0', 'e0e0e0e0-e0e0-e0e0-e0e0-e0e0e0e0e0e0', 'a0a0a0a0-a0a0-a0a0-a0a0-a0a0a0a0a0a0', 1, 2899.00, 5.00);

insert into crm.faturas (id, cotacao_id, responsavel_id, numero, data_emissao, data_vencimento, status, valor_total)
values
  ('11112222-3333-4444-5555-666677778888', 'e0e0e0e0-e0e0-e0e0-e0e0-e0e0e0e0e0e0', '11111111-1111-1111-1111-111111111111', 'FAT-0001', current_date, current_date + interval '10 days', 'aberta', 2754.05);

insert into crm.itens_fatura (id, fatura_id, produto_id, quantidade, preco_unitario)
values
  ('9999aaaa-bbbb-cccc-dddd-eeeeffff0000', '11112222-3333-4444-5555-666677778888', 'a0a0a0a0-a0a0-a0a0-a0a0-a0a0a0a0a0a0', 1, 2754.05);

insert into crm.pagamentos (id, fatura_id, valor, data_pagamento, status, metodo)
values
  ('12121212-1212-1212-1212-121212121212', '11112222-3333-4444-5555-666677778888', 2754.05, current_date, 'pendente', 'boleto');

insert into crm.atividades (id, usuario_id, contato_id, oportunidade_id, tipo, status, assunto, descricao, data_inicio, data_fim)
values
  ('13131313-1313-1313-1313-131313131313', '11111111-1111-1111-1111-111111111111', 'cccccccc-cccc-cccc-cccc-cccccccccccc', '70707070-7070-7070-7070-707070707070', 'tarefa', 'pendente', 'Preparar proposta', 'Revisar pontos comerciais', '2024-03-01 09:00:00+00', null),
  ('14141414-1414-1414-1414-141414141414', '22222222-2222-2222-2222-222222222222', 'dddddddd-dddd-dddd-dddd-dddddddddddd', '80808080-8080-8080-8080-808080808080', 'reuniao', 'concluida', 'Reuniao inicial', 'Apresentacao do produto', '2024-03-02 14:00:00+00', '2024-03-02 15:00:00+00'),
  ('15151515-1515-1515-1515-151515151515', '11111111-1111-1111-1111-111111111111', 'cccccccc-cccc-cccc-cccc-cccccccccccc', '70707070-7070-7070-7070-707070707070', 'anotacao', 'concluida', 'Registro', 'Observacoes do cliente', '2024-03-03 10:00:00+00', '2024-03-03 10:05:00+00');

insert into crm.tarefas (id, atividade_id, status, prioridade, data_limite)
values
  ('16161616-1616-1616-1616-161616161616', '13131313-1313-1313-1313-131313131313', 'em_andamento', 3, '2099-12-31');

insert into crm.reunioes (id, atividade_id, local, link_reuniao, duracao_minutos)
values
  ('17171717-1717-1717-1717-171717171717', '14141414-1414-1414-1414-141414141414', 'Sala 1', 'https://meet.local/reuniao-1', 60);

insert into crm.anotacoes (id, atividade_id, conteudo)
values
  ('18181818-1818-1818-1818-181818181818', '15151515-1515-1515-1515-151515151515', 'Cliente solicitou proposta com desconto.');
