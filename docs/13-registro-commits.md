# Registro de Commits

## Commit: AST — Suporte a struct

**Data:** 2026-09-27  
**Arquivo:** `src/ir/ast.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `Type::Struct(String)` | Nova variante de tipo para representar structs declarados pelo usuário |
| `Expr::Field { base, field }` | Novo nó de expressão para acesso a campos (`p.campo`) |
| `StructField { name, ty }` | Estrutura para representar um campo individual de um struct |
| `StructDecl { name, fields }` | Estrutura para representar uma declaração completa de struct |
| `TopLevelItem::Function \| ::Struct` | Enum para distinguir funções e structs no nível superior |
| `Program { itens }` | Campo `functions` substituído por `itens: Vec<TopLevelItem<Ty>>` |
| `Decl.init` como `Option` | Permite declaração de variáveis struct sem initializer |
| `Program::main_function()` | Atualizado para buscar `main` entre `itens` |
