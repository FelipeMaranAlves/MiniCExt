# Sincronização com o upstream: ponteiros (PR #3 do professor)

O repositório do professor (`rbonifacio/MiniC`) recebeu o PR #3, de outro grupo, que adiciona **ponteiros** ao MiniC.
O nosso fork estava sem ele. Este merge traz essas mudanças para a nossa `main`.

## O que veio do upstream

| O quê | Exemplo | Onde |
|---|---|---|
| Tipo ponteiro (só `int`, `float`, `bool`, `str`) | `int* p;` | `Type::Pointer` em `ir/ast.rs` |
| Endereço de variável | `p = &x;` | `Expr::AddrOf` |
| Desreferência | `y = *p;` | `Expr::Deref` |
| Escrever através do ponteiro | `*p = *p + 1;` | `lvalue` em `parser/statements.rs` |
| Regras de tipo | `&` só em variável; `*` só em ponteiro | `semantic/type_checker.rs` |
| Execução | o ponteiro guarda um endereço (`Value::Ptr(usize)`) | `interpreter/` |
| TAC | instruções `x = *y` e `*x = y` | `codegen/`, `ir/tac.rs` |

**Mudança grande no `Environment`** (`environment/env.rs`): antes era só `nome → valor`. Agora são duas tabelas:
`nome → endereço` (`scopes`) e `endereço → valor` (`store`). Os métodos `names()` e `remove_new()` **deixaram de existir**.
O type checker e o interpretador do struct vão ser escritos **em cima desse modelo novo**.

## Impacto no nosso struct e como resolvemos

Houve 2 conflitos, ambos resolvidos mantendo as duas features:

1. **`ir/ast.rs`**: nós adicionamos `Expr::Field` (`p.campo`) e eles `Expr::AddrOf`/`Expr::Deref` no mesmo lugar.
   → Ficaram as três variantes.
2. **`parser/statements.rs` (`lvalue`)**: os dois grupos reescreveram o lado esquerdo da atribuição.
   → O começo agora aceita `x` ou `*x` (deles), seguido de `[i]` e `.campo` em qualquer ordem (nosso `postfix`).
   Funcionam `p.x = 1;`, `*p = 1;`, `a[i].x = 1;` etc.

Testes do upstream que tivemos de **adaptar ao nosso AST** (as mudanças do struct já tinham quebrado esses testes antes do merge):

- `tests/program.rs`: `prog.functions` não existe mais (agora é `prog.items`, com funções e structs). Criamos o helper `functions(&prog)`.
- `tests/parser.rs`: `init` de `Decl` agora é `Option` (para `struct P p;` sem `=`). Usamos `init.as_ref().unwrap()`.

Nada do que já fizemos no struct precisou mudar de comportamento.

## Estado dos testes depois do merge

- `cargo test --lib`: 19 ok
- `tests/parser.rs`: 64 ok (inclui os de ponteiro)
- `tests/program.rs`: 10 ok (inclui os de ponteiro)

> Atenção: `cargo test` direto ainda falha, mas **já falhava antes do merge**. O `src/main.rs` e os testes
> `interpreter`, `type_checker`, `tac_gen` e `stdlib` usam os módulos que desligamos em `src/lib.rs`.
> Para rodar só a parte do parser, troque o `src/main.rs` por `fn main(){}` **localmente** e rode
> `cargo test --lib --test parser --test program`.

## Próximo passo (type checker e interpretador do struct)

Quando religarmos `semantic`, `interpreter`, `codegen` e `stdlib` em `src/lib.rs`, o código desses módulos (o nosso e o de ponteiros do upstream) precisa:

- percorrer `program.items` em vez de `program.functions`;
- tratar `Decl.init` como `Option` (sem `=`, usar o valor zero do tipo);
- ter casos para `Expr::Field` e `Type::Struct` nos `match`;
- usar o `Environment` novo (`scopes` + `store`).
