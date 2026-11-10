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

O inicializador é opcional para todos os tipos, e o parser não ramifica por
tipo: o que fazer com `None` fica para as etapas seguintes. Acesso a campo usa
`.` e vale tanto na leitura quanto como destino de atribuição. Leitura (`primary`) e escrita (`lvalue`) usam o
mesmo `postfix`, então as duas aceitam exatamente os mesmos sufixos.
`struct Pessoa p = q;` é aceito pelo parser; recusar a cópia de um struct inteiro para outro é papel do checker.

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
| Checker remonta `items` em ordem | Funções são checadas; structs passam sem mudança e mantêm a posição relativa às funções |
| `Decl` sem inicializador | Erro explícito "not supported yet" no checker e no runtime (liberado no commit da declaração sem inicializador) |
| `Expr::Field` | Erro explícito "not supported yet" no checker e no runtime (fica para a tarefa de acesso a campo) |
| Testes migrados | `prog.functions` → `prog.functions()`; `init` → `Some(init)` |

### Por que

O AST de struct (`items`, `init: Option`) quebrou os módulos da entrega 2, e o
merge do upstream (ponteiros) trouxe mais código com a API antiga. Esta é a
base de que as outras tarefas dependem. Os casos ainda não implementados dão
**erro explícito**, e não `todo!()`: o CLI informa a falha em vez de entrar em
pânico, e os testes de cada dono mostram o erro mudar para o comportamento certo.

**`TopLevelItem` continua sendo a representação do programa.** O que deixou de
existir foi o campo antigo `functions`, que o checker, o interpretador, os testes
e o código de ponteiros do upstream ainda usavam. Em vez de repetir em cada um
deles o `match` que separa funções de structs, `Program::functions()` oferece
essa visão a partir de `items`: quem só precisa das funções (registrar
assinaturas, achar a `main`, montar o ambiente) não precisa saber que structs
existem. Quem precisa dos dois tipos de item, como o checker ao remontar o
programa, continua fazendo `match` em `TopLevelItem`.

O codegen (entrega 3) não foi alterado: ele tem um curinga `_ => todo!()`, então
compila, mas entra em pânico se receber `Field`, literal de struct ou declaração
sem inicializador. O CLI não chama o codegen.

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
| `Expr::StructLit { name, fields }` | Literal `Pessoa { idade: 30, nome: "Ana" }`; `fields: Vec<(String, ExprD)>` |
| Braços com erro explícito | Checker e runtime respondem "struct literal is not supported yet" até a tarefa do literal de struct |

### Por que

Os campos ficam numa **lista ordenada**, não num mapa. A lista guarda a ordem em
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

Pré-requisito do próximo commit (`struct` opcional ao usar o tipo): quando um
identificador puder ser o nome de um tipo struct, `if x;` seria lido como "declarar `x` do tipo `if`". Antes,
as palavras-chave só eram reconhecidas pelo `tag` de cada comando, e nada
impedia usá-las como nome de variável (`int struct = 1;` passava). Ficam
reservadas todas as palavras que começam um comando ou são operadores (`if`,
`else`, `while`, `and`, `or`, `struct`), e não só `if`, para que nenhuma delas
possa ser lida como nome de tipo ou de variável. Reservar só a palavra **exata** mantém
nomes como `structure` válidos. O custo é que programas que usavam essas
palavras como nome deixam de compilar; nenhum teste ou fixture fazia isso.

### Verificação

- `cargo test` (parser: 66) e `shelltest tests/cli/` verdes.

---

## Commit: Parser — `struct` opcional ao usar o tipo

**Data:** 2026-11-02
**Arquivos:** `src/parser/functions.rs`, `src/parser/statements.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `value_type_name` | `alt((struct Nome, Nome, type_name))`: identificador solto vira `Type::Struct(Nome)` |
| Testes | `Pessoa p;` ≡ `struct Pessoa p;`; `integer p;`; `int x;`, `int* p;`, `str[] s;` continuam tipos embutidos; `x = 1;`, `p.idade = 1;` e `print(x);` não viram declaração |

### Por que

`Pessoa p;` passa a valer o mesmo que `struct Pessoa p;`. O professor pediu
menos verbosidade que o C, e o enunciado já escreve `Point p`. O parser agora só
reconhece a **forma** "tipo seguido de nome"; quem confirma que `Pessoa` é um
struct definido é o checker (tarefa do registro de structs).

A **ordem das alternativas** é a decisão técnica: o `alt` do nom é uma escolha
ordenada (PEG) e não volta atrás depois de um sucesso. Com `type_name` antes,
`integer p;` casaria `int` e deixaria `eger p;`, e a declaração inteira
falharia. O identificador vem antes porque consome a palavra inteira e rejeita
as reservadas (`int`, `str`, ...), que então caem em `type_name` (`int*`, `int[]`).
Structs continuam fora de parâmetros, tipos de retorno e campos: esses lugares
usam `type_name`, que não aceita identificador.

### Verificação

- `cargo test` (lib: 22) e `shelltest tests/cli/` verdes.

---

## Commit: Parser — `;` opcional depois da definição de struct

**Data:** 2026-11-02
**Arquivo:** `src/parser/structs.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `struct_decl` | O `;` depois do `}` passa a ser `opt(...)` |
| Teste | `struct Point { int x; }` e `struct Point { int x; };` geram o mesmo nó |

### Por que

No C, o `;` existe porque depois do `}` pode vir uma lista de variáveis
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

O `program` é um `many0`: ele para no primeiro item
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

## Commit: Checker — declaração sem inicializador

**Data:** 2026-11-02
**Arquivos:** `src/semantic/type_checker.rs`, `tests/type_checker.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `Statement::Decl` com `init: None` | Aceito para qualquer tipo, exceto `void`; a variável entra no ambiente com o tipo declarado |
| `init: Some(e)` | Igual a antes: `e` é checado **antes** de declarar o nome (`int x = x;` continua sendo erro) |
| Testes | Todos os tipos sem inicializador; `x` continua `int` depois (`x = "a"` é erro); `void v;` e redeclaração continuam erro; o `None` chega ao programa checado |

### Por que

Uma variável declarada sem inicializador (`int x;`) fica **undefined** até a
primeira atribuição, e lê-la antes disso é erro de runtime. Ficar sem valor é um
**estado de execução**, não um tipo diferente. Na
regra de tipagem, `T x;` estende o contexto exatamente como `T x = e;`
(Γ ⊢ … ⇒ Γ, x:T). O checker não tenta provar que `x` foi atribuído antes de
ser lido (*definite assignment*, como Java e C#): essa análise de fluxo
rejeitaria programas corretos, como atribuir dentro de um `while` e ler depois.
A verificação fica no runtime (próximo commit).

**Limite conhecido:** o checker ainda aceita qualquer nome como tipo struct, até
um que não foi definido (`Fantasma f;`). Validar o nome é da tarefa do registro
de structs.

### Verificação

- `cargo test` (type_checker: 44) e `shelltest tests/cli/` verdes.

---

## Commit: Interpretador — valor `undefined` e erro na leitura

**Data:** 2026-11-02
**Arquivos:** `src/interpreter/{value,eval_expr,exec_stmt}.rs`, `tests/interpreter.rs`, `tests/cli/run.test`, `tests/fixtures/interpreter_undefined.minic`, `docs/01-language.md`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `Value::Undefined` | Conteúdo de `T x;` até a primeira atribuição |
| `read_defined(valor, "o quê")` | Portão único de leitura: `Undefined` vira o erro "variable 'x' was read before being assigned" |
| Pontos de leitura | `Expr::Ident`, `*p` (`eval_deref`) e a base de `a[i] = v` (`assign_index`) passam pelo portão |
| `&x` | **Não** lê o valor: `int x; int* p = &x; *p = 5;` define `x` |
| Testes | 9 de runtime (leitura, propagação, todos os tipos, ponteiro, índice, argumento, caminho não executado, laço) + caso de CLI |
| `docs/01-language.md` | Remove "não há variáveis não inicializadas"; documenta undefined, palavras reservadas, `struct` opcional no uso e `;` opcional na definição |

### Por que

**Valor explícito, e não "nome ausente".** Depois do merge do upstream, o
`Environment` separa nome → endereço (`scopes`) de endereço → valor (`store`).
Se `T x;` só reservasse o nome, sem valor no `store`, `write_store` (que exige a
chave) recusaria `*p = 5` com `p = &x`, e `*p` daria "endereço inválido" em vez
de "não atribuído". Com `Undefined` no `store`, o endereço existe desde a
declaração, e só a **leitura** é proibida.

**A mensagem é diferente da de variável não declarada.** O interpretador já
usava "undefined variable 'x'" para um nome que **não existe**. Por isso a leitura
de uma variável declarada mas ainda sem valor diz "variable 'x' was read before
being assigned": a palavra *undefined* sozinha confundiria os dois casos.

**A checagem é na leitura, e não em cada operação.** Assim, `int y = x;` já falha
na declaração de `y`, e um `Undefined` nunca é copiado para outra variável. Se a
checagem ficasse nas operações (soma, comparação, `print`...), o valor poderia
passar de variável em variável até alguém usá-lo, e o erro apareceria longe da
causa.

**Um portão, vários pontos de leitura.** O plano inicial era checar só em
`Expr::Ident`, mas os ponteiros criaram outra porta de leitura (`*p` lê do
`store` sem passar pelo nome). Por isso a regra virou uma função: cada caminho
que lê um valor guardado passa por ela. Quem implementar a leitura de `Field`
chama o mesmo `read_defined`.

**Combinado para campos de struct:** campo não atribuído = campo
**ausente** no mapa do valor struct. O runtime não precisa conhecer a definição
do struct (o `Decl` só tem o nome do tipo), e o checker já garante que o campo
existe, então "ausente" só pode significar "não atribuído". Na leitura de
campo, a ausência vira `read_defined(Value::Undefined, "field 'p.idade'")`.

**Estado atual de uma variável struct:** sem a tarefa do registro de structs,
`Pessoa p;` guarda `Value::Undefined`, como qualquer outra declaração sem
inicializador. Essa tarefa troca isso por um valor struct **vazio** (mapa sem
campos) quando o tipo é struct, para que `p.idade = 3` tenha onde gravar.

### Verificação

- `cargo test` (interpreter: 40) e `shelltest tests/cli/`: 14/14.

---

## Commit: AST — `Type::Struct` vira `Type::UserDefined`

**Data:** 2026-11-02
**Arquivos:** `src/ir/ast.rs`, `src/parser/functions.rs`, `src/parser/{statements,structs}.rs` (testes), `docs/01-language.md`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `Type::Struct(String)` → `Type::UserDefined(String)` | O tipo de `Pessoa p;` passa a dizer "um tipo criado pelo programa, chamado `Pessoa`", e não "um struct chamado `Pessoa`" |
| `struct Pessoa p;` e `Pessoa p;` | Os dois geram `Type::UserDefined("Pessoa")`; a palavra `struct` no uso continua opcional |
| `docs/01-language.md` | O struct é apresentado como um tipo definido pelo usuário |

### Por que

**Sugestão do professor**: representar o tipo como *user-defined type* (tipo
definido pelo usuário), o mesmo termo do enunciado ("user-defined record types").

1. **O AST não deve afirmar o que o parser não sabe.** Desde que o `struct`
   ficou opcional no uso, o parser aceita qualquer identificador como tipo.
   Em `integer p;`, o AST dizia `Type::Struct("integer")`, afirmando que é um
   struct sem ter como saber. `UserDefined` diz só o que o parser sabe: é um
   nome. Se ele existe, e **que tipo de definição** é, quem decide é o checker,
   consultando a tabela de tipos (nome → definição) da tarefa do registro de
   structs.
2. **O MiniC vai evoluir além do struct.** Com `Struct(String)`, cada novo tipo
   do usuário (enum, apelido de tipo...) exigiria uma variante nova em `Type`,
   e o parser teria que decidir qual delas usar só olhando um nome, o que ele
   não consegue. Com `UserDefined(String)`, um tipo novo vira só mais um tipo
   de entrada na tabela do checker; `Type`, o parser e todo `match` sobre
   `Type` ficam como estão.

**Custo:** quem lê `Type::UserDefined` não sabe, só pelo AST, que é um struct;
precisa olhar a tabela de definições. Também se perde uma informação: com
`struct Pessoa p;` explícito, o programador disse que é um struct, e hoje isso
não chega ao checker. Se um dia houver enums, `struct Direcao d;` (com `Direcao`
sendo enum) não será recusado por esse motivo, a menos que o AST volte a guardar
se a palavra `struct` foi escrita.

**Por que `UserDefined` e não `Named`:** "named type" (tipo nomeado) é o termo
mais comum na literatura de compiladores, mas não é uma expressão do dia a
dia da equipe. `UserDefined` é o termo do professor e do enunciado e descreve bem o que o
tipo é: criado pelo programa, e não embutido na linguagem.

**Momento:** a mudança entra antes da tarefa do registro de structs, para que a
tabela de definições e as mensagens de erro já nasçam com o nome novo.

### Verificação

- `cargo test` e `shelltest tests/cli/` (14/14) verdes; nenhum comportamento muda.

---

## Commit: Checker e Interpretador — Registro de structs, tipagem nominal e proibições de struct inteiro

**Data:** 2026-11-09  
**Arquivos:** `src/semantic/type_checker.rs`, `src/semantic/mod.rs`, `src/interpreter/value.rs`, `src/interpreter/exec_stmt.rs`, `tests/type_checker.rs`, `tests/interpreter.rs`

### Mudanças

| Mudança | Descrição |
|---------|-----------|
| `structs: HashMap<String, StructDecl>` | Catálogo de tipos de struct preenchido e verificado em ordem de declaração no programa |
| `lookup_struct` | Função utilitária para consultar a definição de um struct pelo nome ou emitir erro de tipo |
| Validações de struct | Rejeita struct com nome duplicado, colisão de identificador com função, campos com nomes duplicados, campos de tipo `void` e campos aninhados |
| Funções sem struct em assinatura | Proíbe funções de terem parâmetros ou tipo de retorno com tipo struct |
| Validação em declaração local | Tipo declarado pelo usuário deve existir no catálogo; nome de variável local não pode colidir com struct |
| Tipagem nominal | Dois `Type::UserDefined` só são compatíveis se tiverem exatamente o mesmo nome (TAPL §19.3); structs não são compatíveis com `Type::Any` |
| Proibições de struct inteiro | Bloqueia cópia em declaração (`Point b = a;`), atribuição completa (`a = b;`), comparações (`==`, `!=`), literais de array (`[p]`) e passagem de struct para funções (`print(p)`) |
| `Value::Struct(HashMap<String, Value>)` | Nova variante de valor no interpretador para instâncias de structs |
| Inicialização no interpretador | Declaração sem inicializador de tipo `UserDefined` instancia struct com mapa de campos vazio |
| Testes automatizados | 15 novos testes no type checker e 1 novo teste no interpretador cobrindo todas as novas regras |

### Por que

**Declaração antes do uso (ordem textual única):** O compilador do MiniC processa as definições em um único passo sequencial. O catálogo de structs é populado na ordem em que aparecem no código. O uso de um tipo antes de sua declaração (ou a declaração de structs recursivos/aninhados) é rejeitado com erro de tipo não definido (`undefined type`).

**Espaço de nomes compartilhado:** Structs, funções e variáveis locais compartilham o mesmo espaço conceitual de identificadores. Evita-se assim ambiguidade entre invocar uma função ou instanciar/usar um tipo de mesmo nome no mesmo escopo.

**Tipagem nominal (TAPL §19.3):** Adota-se estritamente tipagem nominal para `Type::UserDefined`: dois structs são tipos distintos mesmo que possuam campos idênticos em nome e tipo. Além disso, structs não casam com `Type::Any`, impedindo que funções polimórficas built-in (como `print`) aceitem structs diretamente.

**Proibições de operações em struct inteiro:** O modelo do MiniC não implementa cópia profunda (deep copy) nem comparação campo a campo automática de registros. Para evitar comportamentos indefinidos ou cópias parciais de estado em memória:
- Atribuição de struct inteiro (`a = b;` ou `Point b = a;`) é proibida;
- Comparações (`a == b`, `a != b`) são proibidas;
- Literais ou arrays de structs são proibidos;
- Passagem de structs inteiros como argumentos ou retornos de funções é rejeitada no checker.
A manipulação deve ocorrer exclusivamente campo a campo (ou futuramente via ponteiros).

**Inicialização com mapa vazio no runtime:** Conforme alinhado no commit do valor `Undefined`, um campo de struct não inicializado é representado pela sua ausência no mapa interno do `Value::Struct`. Ao executar uma declaração de struct sem inicializador (`Point p;`), o interpretador cria um `Value::Struct(HashMap::new())`. Quando o acesso a campo for avaliado, a ausência de uma chave resultará na chamada a `read_defined(Value::Undefined, "field 'p.campo'")`, garantindo detecção precisa de leituras não inicializadas sem sobrecarregar o interpretador com schemas estáticos de tipos.

### Verificação

- `cargo check --tests` e `cargo check --lib` passando com 0 erros.
- Cobertura completa de cenários válidos e inválidos em `tests/type_checker.rs` e `tests/interpreter.rs`.

