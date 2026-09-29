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
| `Program { items }` | Campo `functions` substituído por `items: Vec<TopLevelItem<Ty>>` |
| `Decl.init` como `Option` | Permite declarar uma variável sem initializer explícito |
| `Program::main_function()` | Atualizado para buscar `main` entre os `items` |

> Correção posterior: o campo foi escrito inicialmente como `itens` e renomeado
> para `items` no commit `ed89dcd`, junto com o restante do código do projeto,
> que é em inglês.

---

## Commit: Isolar o parser da entrega 1

**Data:** 2026-09-28
**Commit:** `24a933f` — "disabled temporarily some lib modules"
**Arquivo:** `src/lib.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `pub mod interpreter;` comentado | Módulo da entrega 2; quebrado pelo commit do AST |
| `pub mod semantic;` comentado | Type checker, entrega 2; quebrado pelo commit do AST |
| `pub mod codegen;` comentado | Geração de código, entrega 3 |
| `pub mod stdlib;` comentado | Importa `Value`, `RuntimeError` e `NativeFn` de `crate::interpreter`, então não fica ligado sozinho |

### Por que

O commit `903be84` deixou `semantic`, `interpreter` e `codegen` sem compilar. Como
o `stdlib` importa tipos do interpreter e o interpreter importa o `NativeRegistry`
do stdlib, os dois formam um par inseparável. Rust compila o crate inteiro de uma
vez, então esses quatro módulos impediam qualquer verificação da entrega 1.

Desligados eles, a lib passa a conter só `environment`, `ir` e `parser`, e
`cargo build --lib` falha apenas nas duas linhas do **próprio parser** que o commit
do AST deixou para trás: `src/parser/program.rs` e `src/parser/statements.rs`, já
atribuídas a T2 e T3 no plano de tasks.

### Verificação

- `cargo build --lib` — restam 2 erros, ambos em arquivos de T2 e T3.
- `cargo test --lib` — roda os testes de `src/` e ignora `tests/`, que só volta a
  compilar na entrega 2.


---

## Commit: T3 — Declaração com struct e acesso a campo

**Data:** 2026-09-28
**Arquivos:** `src/parser/statements.rs`, `src/parser/expressions.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `decl_statement` usa `value_type_name` | Aceita `struct Nome p;` como declaração |
| Initializer opcional | `T x;` gera `Decl.init = None`; `T x = e;` gera `Some` — para todos os tipos |
| `postfix` em `expressions.rs` | Laço de sufixos `[ expr ]` e `.campo`; `.campo` gera `Expr::Field { base, field }` |
| `lvalue` reusa `postfix` | `p.campo = 42;` e `p.v[i] = 1;` viram `Statement::Assign` |
| Testes em `#[cfg(test)]` | Declarações com e sem initializer, leitura/escrita de campo, encadeamento e rejeições |

### Por que

Decisão 7 revisada: o parser não ramifica por tipo; quem transforma `None` no
valor zero é o checker. Decisão 10: acesso a campo com `.`, válido em leitura e
como destino de atribuição. Leitura (`primary`) e escrita (`lvalue`) usam o
mesmo `postfix`, então as duas aceitam exatamente os mesmos sufixos.
`struct Pessoa p = q;` é aceito pelo parser; a recusa da decisão 18 é do checker.

### Verificação

- `cargo test --lib` — 16 testes passando (10 da T3 + 6 da T1), com
  `program.rs` corrigido **localmente e sem commit**, já que ele é da T2 e ainda
  impede a lib de compilar.
- `rustfmt --check` limpo nos dois arquivos.
