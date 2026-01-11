drop schema if exists crm cascade;
create schema crm;

create extension if not exists pgcrypto;

create type crm.status_lead as enum ('novo', 'qualificado', 'perdido');
create type crm.status_oportunidade as enum ('aberta', 'ganha', 'perdida');
create type crm.status_atividade as enum ('pendente', 'concluida', 'cancelada');
create type crm.status_tarefa as enum ('aberta', 'em_andamento', 'concluida', 'cancelada');
create type crm.status_fatura as enum ('aberta', 'paga', 'cancelada');
create type crm.status_pagamento as enum ('pendente', 'confirmado', 'estornado');
