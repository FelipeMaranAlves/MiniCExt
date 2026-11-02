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

---

## Commit: Entrega 2 — religar checker, interpretador, stdlib e codegen

**Data:** 2026-11-02
**Arquivos:** `src/lib.rs`, `src/ir/ast.rs`, `src/semantic/type_checker.rs`, `src/interpreter/{mod,exec_stmt,eval_expr}.rs`, `tests/{type_checker,interpreter,program}.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `src/lib.rs` | `semantic`, `interpreter`, `stdlib` e `codegen` voltam a compilar |
| `Program::functions()` | Itera só as funções de `items`; `main_function()` passa a usá-lo |
| Checker remonta `items` em ordem | Funções são checadas; structs passam sem mudança (D1) |
| `Decl` sem inicializador | Erro explícito "not supported yet" no checker e no runtime (liberado na tarefa do undefined) |
| `Expr::Field` | Erro explícito "not supported yet" no checker e no runtime (tarefa da Pessoa C) |
| Testes migrados | `prog.functions` → `prog.functions()`; `init` → `Some(init)` |

### Por que

O AST de struct (`items`, `init: Option`) quebrou os módulos da entrega 2, e o
merge do upstream (ponteiros) trouxe mais código com a API antiga. Esta é a
base de que as outras tarefas dependem. Os casos ainda não implementados dão
**erro explícito**, e não `todo!()`: o CLI informa a falha em vez de entrar em
pânico, e os testes de cada dono mostram o erro mudar para o comportamento certo.

### Verificação

- `cargo test`: todos os alvos verdes (lib, parser, program, type_checker, interpreter, stdlib, tac_gen).
- `shelltest tests/cli/`: 12/12.

---

## Commit: AST — nó do literal de struct

**Data:** 2026-11-02
**Arquivos:** `src/ir/ast.rs`, `src/semantic/type_checker.rs`, `src/interpreter/eval_expr.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `Expr::StructLit { name, fields }` | Literal `Pessoa { idade: 30, nome: "Ana" }` (D23); `fields: Vec<(String, ExprD)>` |
| Braços com erro explícito | Checker e runtime respondem "struct literal is not supported yet" até a tarefa da Pessoa D |

### Por que

Combinado A+D do doc 16: **lista ordenada**, não mapa. A lista guarda a ordem em
que o programador escreveu (que vira a ordem de avaliação) e deixa o checker
detectar campo repetido. Num `HashMap`, `{ a: 1, a: 2 }` viraria um campo só, em
silêncio. Com este commit, `src/ir/ast.rs` fica fechado para a entrega 2: as
outras tarefas só adicionam braços de `match`, sem mexer no AST.

### Verificação

- `cargo test` e `shelltest tests/cli/` verdes (nada muda de comportamento).

---

## Commit: Parser — palavras-chave reservadas

**Data:** 2026-11-02
**Arquivos:** `src/parser/identifiers.rs`, `tests/parser.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `RESERVED` | Passa a incluir `struct`, `if`, `else`, `while`, `and`, `or` |
| Testes | Palavras-chave rejeitadas como identificador; prefixos (`structure`, `iffy`, `android`) continuam válidos; `int if = 1;` é erro de sintaxe |

### Por que

Pré-requisito da D21 (próximo commit): quando um identificador puder ser o nome
de um tipo struct, `if x;` seria lido como "declarar `x` do tipo `if`". Antes,
as palavras-chave só eram reconhecidas pelo `tag` de cada comando, e nada
impedia usá-las como nome de variável (`int struct = 1;` passava). Reservar só
a palavra **exata** mantém nomes como `structure` válidos.

### Verificação

- `cargo test` (parser: 66) e `shelltest tests/cli/` verdes.

---

## Commit: Parser — `struct` opcional ao usar o tipo (D21)

**Data:** 2026-11-02
**Arquivos:** `src/parser/functions.rs`, `src/parser/statements.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `value_type_name` | `alt((struct Nome, Nome, type_name))`: identificador solto vira `Type::Struct(Nome)` |
| Testes | `Pessoa p;` ≡ `struct Pessoa p;`; `integer p;`; `int x;`, `int* p;`, `str[] s;` continuam tipos embutidos; `x = 1;`, `p.idade = 1;` e `print(x);` não viram declaração |

### Por que

D21: o professor pediu menos verbosidade que o C, e o enunciado já escreve
`Point p`. O parser agora só reconhece a **forma** "tipo seguido de nome"; quem
confirma que `Pessoa` é um struct definido é o checker (Pessoa B).

A **ordem das alternativas** é a decisão técnica: o `alt` do nom é uma escolha
ordenada (PEG) e não volta atrás depois de um sucesso. Com `type_name` antes,
`integer p;` casaria `int` e deixaria `eger p;`, e a declaração inteira
falharia. O identificador vem antes porque consome a palavra inteira e rejeita
as reservadas (`int`, `str`, ...), que então caem em `type_name` (`int*`, `int[]`).
A D15 continua valendo: parâmetros, retorno e campos usam `type_name`, que não
aceita identificador.

### Verificação

- `cargo test` (lib: 22) e `shelltest tests/cli/` verdes.

---

## Commit: Parser — `;` opcional depois da definição de struct (D22)

**Data:** 2026-11-02
**Arquivo:** `src/parser/structs.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `struct_decl` | O `;` depois do `}` passa a ser `opt(...)` |
| Teste | `struct Point { int x; }` e `struct Point { int x; };` geram o mesmo nó |

### Por que

D22: no C, o `;` existe porque depois do `}` pode vir uma lista de variáveis
(`struct P {...} a, b;`). O MiniC não tem essa forma, então o `;` não separa
nada. Aceitar os dois jeitos não cria ambiguidade, porque no topo só existem
definições de struct e funções, e nenhuma delas começa com `;`.

### Verificação

- `cargo test` (lib: 23) e `shelltest tests/cli/` verdes.

---

## Commit: CLI — erro quando sobra entrada depois do programa

**Data:** 2026-11-02
**Arquivos:** `src/main.rs`, `tests/cli/check.test`, `tests/fixtures/cli_trailing_garbage.minic`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `main.rs` | Usa `all_consuming(terminated(program, multispace0))`: o arquivo inteiro precisa ser consumido (espaço no fim é tolerado) |
| `check.test` | `invalid_syntax.minic` passa a dar **Parse error** (antes dava "Type error: program must have a main function") |
| Fixture nova | `main` válido seguido de `struct P { int x;` sem fechar |

### Por que

Questão técnica 1 do doc 11. O `program` é um `many0`: ele para no primeiro item
que não reconhece e devolve o resto, que o CLI descartava. Antes desta correção,
a fixture nova **imprimia `1` e saía com código 0**: o struct malformado sumia em
silêncio. Um erro de sintaxe tem que ser reportado como erro de sintaxe, e não
reaparecer mais tarde como outro erro (ou não reaparecer).

**Escopo: correção da linguagem como um todo, e não parte da extensão de
struct.** O bug já existia no MiniC base e continua na `main` do repositório do
professor: o parser de programa original (`many0(fun_decl)`) também parava no
primeiro item que não reconhecia, e o CLI descartava o resto do mesmo jeito. A
correção entra aqui porque o problema apareceu ao testar definições de struct
(um `struct P { int x;` sem fechar no fim do arquivo sumia sem erro) e porque
estava na lista de pendências do parser. Pode ser proposta também ao upstream.

### Verificação

- `cargo test` verde; `shelltest tests/cli/`: 13/13.

---

## Commit: Checker — declaração sem inicializador (D20)

**Data:** 2026-11-02
**Arquivos:** `src/semantic/type_checker.rs`, `tests/type_checker.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `Statement::Decl` com `init: None` | Aceito para qualquer tipo, exceto `void`; a variável entra no ambiente com o tipo declarado |
| `init: Some(e)` | Igual a antes: `e` é checado **antes** de declarar o nome (`int x = x;` continua sendo erro) |
| Testes | Todos os tipos sem inicializador; `x` continua `int` depois (`x = "a"` é erro); `void v;` e redeclaração continuam erro; o `None` chega ao programa checado |

### Por que

D20: ficar sem valor é um **estado de execução**, não um tipo diferente. Na
regra de tipagem, `T x;` estende o contexto exatamente como `T x = e;`
(Γ ⊢ … ⇒ Γ, x:T). O checker não tenta provar que `x` foi atribuído antes de
ser lido (*definite assignment*, como Java e C#): essa análise de fluxo
rejeitaria programas corretos, como atribuir dentro de um `while` e ler depois.
A verificação fica no runtime (próximo commit). Se o struct `Pessoa` em
`Pessoa p;` existe é responsabilidade da Pessoa B.

### Verificação

- `cargo test` (type_checker: 44) e `shelltest tests/cli/` verdes.
