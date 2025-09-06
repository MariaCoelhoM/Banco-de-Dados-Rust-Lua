# KV em Rust + VM Lua (extensões)

## Rodar no VS Code (terminal)

```bash
# 1) Entre na pasta do projeto
cd rust-lua-kv

# 2) Compile e rode
cargo run
```

Se quiser editar/expandir extensões, abra `extensions.lua`. O binário recarrega esse arquivo a cada execução.

## Comandos suportados
- `ADD <chave> <valor>`
- `GET <chave>`
- `EXIT` para sair

## Exemplos
```
ADD cpf_zezinho 12345678909
GET cpf_zezinho
ADD data_nascimento_zezinho 2000-01-23
GET data_nascimento_zezinho
```
