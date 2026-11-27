## Registro de decisões

Este registro guarda as decisões de design, o motivo, as alternativas consideradas e as consequências. Uma opção só será marcada como **decidida** depois de confirmada pelo autor da feature.

### Decisão 1 — Como representar declarações de `struct` no programa

**Status:** decidida — opção B, lista única de itens de topo.

#### Situação atual

O parser de topo em [src/parser/program.rs](../src/parser/program.rs) aplica `many0(fun_decl)`: tenta ler declarações de função repetidamente e guarda os resultados. O AST `Program<Ty>` em [src/ir/ast.rs](../src/ir/ast.rs) possui somente `functions: Vec<FunDecl<Ty>>`.

Portanto, se a sintaxe permitir definir um `struct` no nível superior, como `struct Pessoa { ... }`, o parser atual não tem uma regra para reconhecê-lo e o `Program` atual não tem um campo onde guardar essa definição. A gramática de topo e a representação do AST terão de ser consideradas juntas.

#### Pergunta a decidir

Como o programa deve representar definições de `struct` junto com funções?

#### Decisão e justificativa

O autor escolheu a opção B porque quer preservar a ordem do fonte e prefere uma representação que possa acomodar outras categorias de declaração no futuro. A opção A é entendida como duas coleções — uma de funções e uma de structs —, enquanto B é uma única sequência que pode conter funções ou structs.

Esta decisão determina a forma de representar os itens no programa; não determina as regras de visibilidade. B preserva a ordem textual, mas a semântica de visibilidade e de “declarar antes de usar” ainda terá de ser especificada separadamente.

#### Opção A — Guardar structs e funções em coleções separadas

Representação conceitual:

```text
Program
├── structs: Vec<StructDecl>
└── functions: Vec<FunDecl>
```

O parser poderia reconhecer os dois tipos de declaração e colocar cada um na coleção correspondente.

**Prós**

- É uma extensão direta e fácil de explicar do `Program` atual.
- As funções continuam disponíveis na coleção que o type checker e o interpreter já percorrem.
- É simples obter “todas as definições de struct” ou “todas as funções”.

**Contras / perguntas em aberto**

- A ordem original entre structs e funções não fica naturalmente preservada. Por exemplo, não se sabe se uma definição apareceu antes ou depois de uma função.
- Se a linguagem futuramente exigir que um struct seja declarado antes de seu uso, ou permitir declarações incompletas, essa informação de ordem pode ser importante.
- As etapas seguintes terão de decidir se structs são registrados antes de verificar funções, independentemente da ordem no arquivo.

**Exemplo do trade-off da ordem**

Considere:

```c
struct Pessoa { ... }
void main() { ... }
struct Endereco { ... }
```

Com coleções separadas, o AST pode guardar `Pessoa` e `Endereco` em `structs`, e `main` em `functions`, mas não informa naturalmente que `Endereco` aparecia depois de `main`. Isso só é uma limitação observável se a ordem do fonte tiver significado para a linguagem ou precisar ser preservada para ferramentas/diagnósticos. Se o checker primeiro registrar todas as definições de tipo e depois verificar funções, talvez a ordem não importe.

#### Opção B — Uma lista única de itens de topo

Representação conceitual:

```text
Program
└── items: Vec<TopLevelItem>
	├── Struct(StructDecl)
	└── Function(FunDecl)
```

Cada item mantém sua posição original no programa.

**Prós**

- Preserva a ordem do código-fonte.
- Torna explícito que um programa pode conter diferentes categorias de declaração.
- Pode acomodar outras declarações de topo no futuro sem adicionar uma coleção separada para cada categoria.

**Contras**

- Introduz um novo tipo de enumeração para os itens de topo.
- O código que hoje percorre diretamente `program.functions` teria de filtrar ou tratar os tipos de item.
- A ordem preservada não determina, por si só, a regra semântica: ainda seria preciso decidir se referências a structs definidos depois são válidas.

**Exemplo da ordem preservada**

Para o exemplo acima, `items` poderia guardar, na mesma sequência, `Struct(Pessoa)`, `Function(main)` e `Struct(Endereco)`. Isso registra a ordem textual; ainda caberia ao type checker decidir se `main` pode usar `Endereco` antes da sua definição.

**Relação com C**

Em C, a posição das declarações afeta a visibilidade dos nomes: em geral, um identificador só pode ser usado depois de sua declaração visível. Tags de `struct` também podem ter declaração incompleta e definição posterior. Preservar a sequência do fonte em B mantém a informação necessária para implementar regras desse tipo. Porém, B sozinha não implementa essas regras: o parser guarda a ordem e a análise semântica decide o que é visível em cada ponto. O MiniC pode adotar uma regra mais simples ou permitir referências para frente; isso continua sendo uma decisão da linguagem.

#### Opção C — Manter as definições de struct fora do AST do programa

Nesta opção, `Program` continuaria guardando funções, e as definições de struct seriam mantidas em outra estrutura, por exemplo um catálogo de tipos entregue ao type checker. Isso é diferente da opção A: em A, os structs continuam fazendo parte do AST `Program`, em um campo separado; em C, a própria AST do programa não os contém.

Por exemplo, o resultado do parsing poderia conceitualmente ser um par: `(ProgramComFuncoes, CatalogoDeStructs)`. Outra possibilidade seria construir esse catálogo durante uma etapa de análise.

**Prós**

- Pode separar o catálogo de tipos das funções executáveis.
- Pode ser conveniente se as definições de tipo forem tratadas como metadados usados pelo type checker.

**Contras / perguntas em aberto**

- É menos direto para alguém que espera que a AST represente tudo o que foi escrito no programa.
- Há que definir onde fica a ordem do código-fonte e como os erros apontam para uma declaração.
- Pode duplicar ou dispersar informação entre a AST e o registro de tipos.
- Se o catálogo não carregar localização/origem suficiente, pode ficar mais difícil produzir bons erros para campos ou tipos inválidos.

**Quando poderia fazer sentido**

- Se definições de struct forem tratadas apenas como informação de tipo necessária durante a verificação, e não como itens que outras etapas precisam percorrer como parte do programa.
- Se houver uma razão clara para manter a AST executável focada nas funções.

O custo é que o programa deixa de ser uma representação única e completa do fonte. É preciso manter sincronizados o programa e o catálogo de tipos.

**Por que não é o foco atual**

O objetivo desta feature é representar a declaração de `struct` no resultado do parsing/AST. Um catálogo auxiliar ainda pode ser construído depois pelo type checker, mas não precisa substituir a declaração na AST. Por isso, C não parece atender à preferência atual de manter no programa a representação do que foi escrito.

#### Opção D — Permitir definição de struct somente em contexto local/embutido

Em vez de uma declaração reutilizável no nível superior, a linguagem poderia permitir definir o tipo dentro de uma função/bloco, ou junto da declaração de uma variável. Uma sintaxe ilustrativa seria:

```c
void main() {
	struct Ponto { int x; int y; } p;
}
```

Essa é apenas uma ilustração: a gramática exata ainda teria de ser escolhida. O ponto definidor da opção é que a declaração pertence a um escopo local, e não à lista de itens de topo.

**Prós**

- Evita mudar `Program` para guardar definições locais; não evita mudanças no parser de statements nem no AST de statements.
- Pode limitar o primeiro incremento da feature.
- Pode permitir que tipos sejam locais a um bloco/função, caso essa seja uma regra desejada.

**Contras**

- Reutilizar o mesmo tipo em várias funções fica difícil ou exige repetir definições, a menos que também exista uma forma de declaração compartilhada.
- A declaração do tipo fica acoplada a um escopo ou lugar de uso; isso muda a visibilidade e o tempo de vida do nome do tipo.
- É uma escolha de linguagem bastante diferente de uma definição reutilizável de struct no nível do programa.
- Mesmo sem tocar `Program`, o parser e o AST ainda precisariam representar a declaração local e o escopo do tipo.

**Quando poderia fazer sentido**

- Se a intenção da linguagem for deliberadamente ensinar tipos locais/escopo local.
- Se o primeiro objetivo for experimentar structs que só são usados em uma função, aceitando a limitação de reuso.

**Por que não é o foco atual**

O objetivo é ter structs reutilizáveis e representados junto das declarações do programa, não introduzir tipos locais ou definições acopladas a uma variável. D foi considerada como alternativa para evitar mudar a lista de itens de topo, mas não evita trabalho no parser/AST: apenas desloca a declaração para o parser de statements e acrescenta questões de escopo. Sem um requisito para tipos locais, esse custo não traz benefício claro para a feature pretendida.

#### Comparação direta entre A e B

Sim: A é uma coleção de funções mais uma coleção de structs; B é uma única lista de itens que podem ser funções ou structs. O trade-off central é **simplicidade imediata (A)** versus **representação uniforme e ordem explícita (B)**, com uma ressalva: preservar a ordem só é uma vantagem semântica se a linguagem ou ferramentas precisarem dela.

- **A é mais simples de encaixar no código atual:** mantém `functions` como coleção direta e acrescenta uma coleção `structs`. Consumidores que processam funções podem continuar percorrendo a lista atual. O preço é perder a ordem intercalada entre categorias.
- **B preserva a ordem do fonte:** todos os itens ficam em uma sequência. Isso pode ajudar se houver regra “declarar antes de usar”, mensagens/diagnósticos com ordem, ou novas categorias de item no futuro. O preço é que consumidores atuais não podem mais assumir que todo item de topo é função; terão de selecionar/tratar funções.
- **B não é automaticamente necessária para permitir tipos declarados depois:** o type checker poderia, em princípio, primeiro coletar todos os structs e depois verificar funções. Isso funciona com A ou B. A decisão sobre permitir referências para frente é uma regra semântica separada da forma do AST.
- **B não torna futuras extensões gratuitas:** adiciona uma estrutura de item genérica que terá de ser tratada nas etapas seguintes. Ela apenas oferece um lugar uniforme para novas categorias.

#### Recomendação registrada antes da decisão

Se o objetivo for permitir definições nomeadas e reutilizáveis no nível superior, as opções A e B são as mais diretas. Como recomendação de modelagem, **a opção B (lista única de itens de topo) é a mais flexível se preservar a ordem do fonte e permitir futuras categorias de declaração forem importantes**. **A opção A é uma alternativa mais simples se o programa tratar tipos e funções como coleções separadas e a ordem entre eles não tiver significado**.

Essa recomendação era condicional. A opção B foi escolhida pelo autor pelos motivos registrados acima. Uma pergunta ainda relevante para uma decisão posterior é: o MiniC permitirá usar um tipo declarado mais abaixo no arquivo? A resposta define a regra semântica de visibilidade, não altera a decisão de representar os itens em uma lista ordenada.

#### Consequências comuns às opções A e B

- O parser de topo deixará de tratar cada item como necessariamente uma função.
- Será necessário um parser de declaração de struct e um nó correspondente no AST.
- O type checker terá de conhecer as definições de struct para resolver nomes de tipos e campos.
- O suporte a valores e acesso de campos no interpreter virá depois; reconhecer a declaração no parser não implementa, por si só, a semântica completa de structs.

#### Narrativa para apresentação

> O programa originalmente era uma lista de funções. Ao introduzir `struct` como uma declaração de topo, percebemos que a gramática e o AST precisavam representar mais de uma categoria de item. Consideramos duas coleções separadas e uma lista unificada que preserva a ordem, além de manter structs fora do AST ou declará-los localmente. Escolhemos a lista unificada porque preserva a sequência do fonte e oferece um lugar comum para outras categorias futuras. A ordem registrada, por si só, não define as regras de visibilidade; essas serão especificadas pela linguagem e aplicadas na análise semântica.

### Decisão 2 — Como escrever um tipo `struct` em declarações

**Status:** decidida — usar `struct Nome` ao declarar uma variável desse tipo.

#### Decisão e justificativa

Exemplo escolhido:

```c
struct Pessoa { int idade; };
struct Pessoa p;
```

O autor escolheu a opção 1 por ser mais simples de reconhecer sintaticamente e mais próxima da forma padrão de C. A palavra-chave explícita `struct` indica ao parser que o nome seguinte é uma tag de struct, em vez de exigir que ele trate qualquer identificador como possível tipo definido pelo usuário.


Essa escolha não elimina a necessidade de representar o tipo nomeado `Pessoa` no AST/sistema de tipos nem de validar a definição e os campos no type checker. Ela torna mais clara a distinção sintática entre tipo struct e identificador comum.

#### Opção 1 — Manter `struct` antes do nome ao usar o tipo — escolhida

Exemplo:

```c
struct Pessoa { int idade; };
struct Pessoa p;
```

Essa forma explicita que `Pessoa` é um struct. É próxima do C, mas exige que o parser de tipos reconheça a combinação `struct` + identificador.

**Precisão importante sobre implementação:** essa forma facilita o parser a reconhecer sintaticamente que se trata de um tipo estruturado, pois há uma palavra-chave explícita. Porém, ela **não elimina** a necessidade de representar o tipo nomeado no AST/tipo da linguagem (por exemplo, guardar que o tipo é `Pessoa`) nem de o type checker verificar se essa definição existe e quais campos possui. A dificuldade que ela evita é principalmente decidir, durante o parsing, se uma palavra identificadora isolada como `Pessoa` já é um nome de tipo.

#### Opção 2 — Usar o nome do struct diretamente como tipo — não escolhida

Exemplo:

```c
struct Pessoa { int idade; };
Pessoa p;
```

Essa forma é mais curta no uso. O parser precisa distinguir nomes de tipos definidos pelo usuário de nomes comuns de identificadores; essa distinção pode exigir uma tabela de nomes conhecidos ou deixar a resolução para uma etapa posterior.

**Precisão importante sobre implementação:** não é obrigatório que o parser consulte uma tabela de tipos enquanto lê. Uma alternativa é ele aceitar sintaticamente um identificador como candidato a tipo e deixar para o type checker confirmar se `Pessoa` foi declarado. Ainda assim, isso exige representar o nome do tipo no resultado do parsing e validar sua existência depois. A diferença é onde e quando se decide que um nome é um tipo válido.

**Relação com C e C++:** em C padrão, `struct Pessoa p;` declara `p` com a tag `Pessoa`; a forma curta `Pessoa p;` normalmente exige um alias criado com `typedef`. C++ permite usar o nome da classe/struct diretamente como tipo, então a opção 2 lembra mais C++. A escolha aqui favorece a forma explícita de C padrão.

#### O que a documentação atual sugere sobre seguir C

Não encontrei uma declaração de objetivo dizendo que MiniC deve manter compatibilidade sintática completa com C. As evidências apontam para **inspiração em C com simplificações para ensino**:

- O [README](../README.md) chama MiniC de linguagem “C-like” e diz que o pipeline é intencionalmente pequeno para ser lido e entendido.
- [docs/01-language.md](01-language.md) descreve uma linguagem própria, com tipos como `str`, operadores `and`/`or`, arrays com sintaxe `T[]` e programa definido como lista de funções.
- Um registro histórico em [openspec/changes/archive/2026-02-18-not-to-bang-syntax/proposal.md](../openspec/changes/archive/2026-02-18-not-to-bang-syntax/proposal.md) justifica `!` por familiaridade com linguagens C-like, indicando que familiaridade com C é um critério de design, mas não uma exigência de compatibilidade.

Assim, a sintaxe `struct Pessoa` ao usar o tipo é a opção que mais se aproxima do C sem adicionar `typedef`. Isso apoiou a escolha da opção 1. Não é evidência de que o autor original tenha exigido compatibilidade com C; a documentação disponível indica inspiração em C com simplificações didáticas.

#### Opção 3 — Não fixar ainda a grafia; primeiro definir a gramática de declaração — alternativa considerada, não escolhida

Podemos decidir primeiro a forma completa da declaração de struct (incluindo campos e terminador) e adiar a grafia usada em declarações de variáveis. Isso reduz decisões simultâneas, mas deixa incompleta a sintaxe dos exemplos de uso do tipo.

### Decisão 3 — Terminador da definição de `struct`

**Status:** decidida — uma definição completa de struct termina com `;`.

Exemplo adotado para a discussão da gramática:

```c
struct Pessoa {
	int idade;
};
```

O autor considera o ponto e vírgula a convenção esperada e não vê benefício em removê-lo. Isso mantém a forma próxima de C e deixa explícito o fim da declaração de tipo. O `;` depois da chave fecha a declaração inteira; cada campo também termina com `;`.

### Decisão 4 — Somente definições completas de `struct`

**Status:** decidida — o escopo inicial aceita definições completas com campos e não aceita declaração incompleta isolada, como `struct Pessoa;`.

#### Justificativa e comparação com funções

No MiniC atual, `fun_decl` em [src/parser/functions.rs](../src/parser/functions.rs) lê tipo de retorno, nome e parâmetros, e depois exige um corpo chamando `statement`. Portanto, o AST de função guarda uma função com corpo; não há uma forma própria para protótipo sem corpo, como `int calcula(int x);`.

Isso dá uma analogia razoável para manter o primeiro escopo de structs pequeno: assim como uma função precisa vir com corpo, a declaração de struct virá com a lista completa de campos. Não é uma equivalência perfeita — C permite protótipos de função e declarações incompletas de struct por motivos diferentes —, mas o MiniC não adotou protótipos de função até agora. Não há necessidade didática ou de implementação já identificada para acrescentar structs incompletos nesta feature.

**Observação técnica sobre o parser atual:** `program()` usa `many0(fun_decl)`, que para quando uma função não pode ser lida. Os testes de programa usam `all_consuming(program)` em [tests/program.rs](../tests/program.rs), então texto não consumido causa falha nesses testes. Já o CLI chama `program()` sem verificar o restante da entrada em [src/main.rs](../src/main.rs); por isso, uma entrada com sintaxe não reconhecida pode ser parcialmente ignorada e resultar depois em outro erro, em vez de um diagnóstico sintático direto. Isso é uma limitação existente, não suporte a protótipos. A decisão de structs completos define a gramática pretendida; a robustez do diagnóstico do parser pode ser tratada separadamente.

#### Consequências

- A regra de struct começa com uma única forma completa: nome e campos entre chaves.
- Não precisamos nesta etapa representar uma declaração incompleta sem campos, como `struct Pessoa;`.
- Esta decisão **não** determina se uma variável pode usar um struct cuja definição completa aparece mais adiante. Essa regra de visibilidade/ordem é tratada na decisão 11. Structs em campos e em parâmetros estão fora do escopo inicial por decisões separadas.
- Se, no futuro, surgirem casos de uso para referências entre structs ou declarações antecipadas, isso poderá ser discutido como uma extensão separada.

### Decisão 5 — Tipos padrão permitidos nos campos

**Status:** decidida — os campos podem usar os tipos armazenáveis já existentes no MiniC, exceto `void`.

Isso inclui os tipos escalares (`int`, `float`, `bool`, `str`) e os arrays que a linguagem já suporta. A regra acompanha as variáveis: um campo guarda um valor, então `void`, que representa ausência de valor, não é permitido.

**Justificativa:** não há motivo suficiente, didático ou de linguagem, para inventar uma restrição diferente da que já vale para variáveis MiniC. Restringir campos a um subconjunto dos tipos existentes só acrescentaria uma regra especial sem benefício claro. `void` é excluído porque já significa ausência de valor e não pode ser armazenado em uma variável.

Esta decisão trata dos tipos já existentes. O uso de outro tipo `struct` como campo é uma decisão separada, registrada abaixo.

### Decisão 6 — Sintaxe dos campos

**Status:** decidida — tipo antes do nome, no formato `tipo nome;`.

Exemplo:

```c
struct Pessoa {
	int idade;
	str nome;
};
```

Essa forma segue o padrão já usado nas declarações MiniC e a convenção familiar de C. Também permite reaproveitar a ordem conceitual atual: reconhecer tipo, depois identificador. Não há motivo suficiente para introduzir a sintaxe alternativa `nome: tipo;`.

#### Alternativa considerada — Nome antes do tipo

```c
struct Pessoa {
	idade: int;
	nome: str;
};
```

Foi descartada por divergir do padrão existente no MiniC e não oferecer benefício suficiente para justificar uma nova convenção.

### Decisão 7 — Inicialização de variáveis

**Status:** decidida — opção B: uma variável declarada sem inicializador recebe o valor zero do seu tipo.

#### Decisão e justificativa

O autor escolheu permitir `struct Pessoa joao;` e inicializar os campos com os defaults definidos para seus tipos. Isso evita valores locais indeterminados, ao contrário do comportamento de C para variáveis locais automáticas sem inicializador.

Valores padrão normativos:

- `int` → `0`
- `float` → `0.0`
- `bool` → `false`
- `str` → `""`
- arrays → vazios (decisão 14)

Structs devem ter definição completa com pelo menos um campo (decisões 4 e 9); declaração incompleta e struct vazio não fazem parte da linguagem inicial.

Inicializadores explícitos, como agregados `{ ... }`, estão fora do escopo inicial e não precisam ser projetados nem decididos agora. A escolha de defaults não impede uma extensão futura.

#### Revisão da regra — zero para toda declaração, sem exceção

A regra original desta decisão era restrita a structs: "MiniC já exige inicializador para variáveis dos tipos atuais, e a declaração sem `=` é uma regra específica para variáveis struct". O autor revisou essa restrição por considerá-la uma assimetria difícil de justificar — `int x;` seria erro de sintaxe enquanto `struct Pessoa p;` seria válido.

A regra agora é única e sem exceções:

> **Toda declaração sem inicializador recebe o valor zero do seu tipo.**

```c
int x;                // 0
float f;              // 0.0
bool b;               // false
str s;                // ""
int[] a;              // []
struct Pessoa p;      // cada campo no zero do seu tipo
```

**Motivos da revisão:**

- **Uma regra, sem caso especial.** Não há exceção para lembrar, documentar ou testar separadamente.
- **Nenhuma máquina nova.** `Decl.init` continua sendo `Option`; não surge nó de AST para literal agregado, e portanto não surge um conjunto novo de regras sobre campo faltando, sobrando, duplicado ou com tipo divergente do declarado. A complexidade de um inicializador agregado está na validação dos campos, não na sintaxe.
- **Nada existente quebra.** `int x = 1;` continua válido; a regra apenas passa a admitir a forma sem `=`.

**Consequência aceita conscientemente:** `int[] a;` produz um array vazio, e como o MiniC não tem `append` nem `resize`, a única forma de preenchê-lo passa a ser atribuição por índice — que falha enquanto o array estiver vazio. Um programa que escreve `int[] a; a[0] = 1;` descobre o problema em runtime, e não em tempo de parsing. O autor considera esse custo aceitável em troca da simplicidade da regra.

**Sequenciamento:** o parser passa a aceitar a forma sem `=` neste milestone. A materialização dos valores zero é responsabilidade do type checker e do interpreter, que ficam para o milestone seguinte.


### Decisão 8 — Permitir um `struct` como campo de outro `struct`

**Status:** decidida — não permitir structs dentro de structs nesta primeira versão.

#### Decisão e justificativa

Os campos podem usar os tipos padrão já decididos (tipos escalares e arrays existentes, exceto `void`), mas não podem ter tipo `struct`, nem diretamente nem por meio de array de structs. A feature inicial ficará limitada a structs cujos campos não sejam outros structs.

A justificativa é manter o escopo didático e de implementação focado na declaração de structs e no acesso aos campos. Árvores binárias e outras estruturas recursivas precisam de indireção, inicialização de nós, representação de filhos ausentes e regras de mutabilidade/cópia. Arrays de structs poderiam vir a oferecer uma forma de indireção, mas também exigiriam generalizar o parser e o type checker e decidir como criar e alterar essas estruturas. Esses assuntos ficam adiados, não descartados como impossíveis.

#### A linguagem tem ponteiros ou referências?

Não. No MiniC atual não há tipo de ponteiro, operador de endereço (`&`) ou operador de desreferência (`*`). O `Box` usado no código Rust para representar nós recursivos da AST é um detalhe interno da implementação; não significa que programas MiniC tenham ponteiros.

Assim, se um campo for de outro tipo `struct`, a interpretação natural é **composição por valor**: o valor do struct externo contém diretamente o valor do struct interno. Isso é perfeitamente possível quando a composição é finita. Por exemplo:

```c
struct Endereco {
	str rua;
};

struct Pessoa {
	struct Endereco endereco;
};
```

`Pessoa` contém um `Endereco`, que contém uma `str`; a estrutura tem tamanho e profundidade finitos. Portanto, **não é realmente complicado permitir um struct dentro de outro** no caso comum. A implementação precisará, claro, dar suporte a valores de struct no AST/checker/interpreter, mas a ausência de ponteiros não impede composição.

O caso difícil é uma composição que volta ao próprio tipo:

```c
struct Pessoa {
	struct Pessoa progenitor;
};
```

Sem ponteiros/referências, cada `Pessoa` teria de conter outra `Pessoa`, que teria de conter outra, sem fim. O mesmo problema aparece indiretamente, como `A` contendo `B` e `B` contendo `A`. Essas formas precisam ser rejeitadas se os structs forem armazenados por valor.

#### Aplicando isso ao exemplo de árvore binária

O tipo conceitual seria algo como:

```c
struct No {
	int valor;
	struct No esquerda;
	struct No direita;
};
```

Mas, se `esquerda` e `direita` forem valores `No` embutidos diretamente, cada nó tem de conter dois nós, cada um contendo outros dois, e assim por diante. Não existe um nó folha finito nessa representação. Além disso, um nó não consegue dizer “não tenho filho”, pois não há um valor de ausência para esses campos.

Logo, **um limite numérico de profundidade, sozinho, não resolve o problema**. Para interromper a árvore numa folha, a linguagem ainda precisaria de alguma forma de representar filhos ausentes e de armazenar os filhos indiretamente. Uma representação fixa e truncada até uma profundidade máxima seria possível em princípio, mas seria um tipo diferente e pouco prático: desperdiça espaço, limita todas as árvores e não representa uma árvore arbitrária.

O problema não é que MiniC tenha uma proibição geral de valores aninhados. O problema específico é tentar representar uma estrutura recursiva de tamanho variável como uma sequência de valores contendo uns aos outros **por valor**, sem uma camada de indireção.

#### Opção A — Permitir composição por valor, sem ciclos

Um campo pode usar `struct OutroTipo`, desde que a composição resultante seja finita.

**Prós**

- Permite modelar dados naturalmente compostos, como pessoa/endereço ou pedido/item.
- É uma capacidade familiar em C para membros struct por valor.
- O modelo de execução continua conceitualmente simples: o campo contém outro valor.

**Contras / regras adicionais**

- O type checker precisa verificar que o tipo do campo existe e que não se forma um ciclo por valor.
- Como structs incompletos foram excluídos, uma regra simples seria só permitir usar um struct já definido completamente antes do struct que o contém. Isso também impede ciclos no caminho de definições, mas proíbe referências para tipos definidos depois.
- Se quisermos permitir definições fora dessa ordem, será necessário coletar as definições e analisar o grafo de dependências para encontrar ciclos.
- Ainda será necessário adicionar valores de struct ao interpreter e definir como inicializar/copy/atribuir esses campos.

Esta opção permite `Pessoa` conter um `Endereco`, mas **não** permite que um `No` contenha filhos `No` para formar uma árvore de profundidade arbitrária. Ela não atende ao exemplo de árvore binária dinâmico.

#### Opção B — Não permitir structs como campos nesta primeira versão — escolhida

Os campos podem usar os tipos existentes decididos acima (escalares e arrays, exceto `void`), mas não `struct OutroTipo`.

**Prós**

- Reduz o escopo inicial: não há dependências entre definições de struct nem ciclos de composição a validar.
- Permite implementar e aprender primeiro declaração, variável struct e acesso aos seus campos.

**Contras**

- Impede estruturas de dados compostas que são um uso comum de structs.
- Pode levar a uma segunda extensão da linguagem para habilitar composição pouco depois.

Esta opção também não atende árvores binárias diretamente. Ela mantém a primeira implementação menor e deixa recursão/indireção para uma feature posterior.

#### Opção C — Adicionar referências/ponteiros para campos recursivos

Permitir que os filhos sejam referências a outros nós, em vez de nós embutidos por valor. A forma superficial da sintaxe ainda precisaria ser escolhida, por exemplo `No*`, `No&` ou um tipo de referência próprio do MiniC. Também seria necessário representar a ausência de filho (um valor nulo/opcional) ou adotar outra convenção para folhas.

**Prós**

- Representa naturalmente árvores de profundidade variável: cada nó armazena referências para zero, um ou dois outros nós.
- É o modelo que mais diretamente corresponde à ideia de “apontar para filhos”.
- Pode ser reutilizado para listas ligadas, grafos e outras estruturas recursivas.

**Contras / esforço**

- Ponteiros e referências seriam uma feature nova da linguagem, não apenas uma mudança no parser de `struct`.
- Seria preciso definir criação/alocação de nós, valor nulo ou opção, leitura e escrita através de referências e comportamento de cópia.
- Referências introduzem identidade e aliasing: duas variáveis podem apontar para o mesmo nó; alterar por uma referência afeta a outra.
- Se forem ponteiros com alocação manual, entram liberação de memória e possíveis referências inválidas. Se forem referências gerenciadas, ainda é preciso escolher e implementar o modelo de memória e ciclos.
- O type checker, o interpreter, a AST e os testes precisam acompanhar essas regras.

**Esforço relativo:** alto para ponteiros/referências genéricos; possivelmente moderado/alto para um mecanismo deliberadamente restrito a referências gerenciadas de `struct`, mas isso ainda é uma decisão e feature de linguagem própria.

#### Opção D — Usar identificadores/índices em um armazenamento de nós (arena)

Em vez de expor ponteiros, o runtime guardaria os nós numa coleção e cada campo filho guardaria um identificador/índice para outro nó, com uma forma de indicar “sem filho”. Conceitualmente: `esquerda: None` ou `esquerda: NodeId(12)`.

**Prós**

- Pode evitar ponteiros crus e liberação manual de memória.
- Dá identidade estável aos nós e permite árvores recursivas.
- Pode ser mais controlado e previsível num interpretador didático.

**Contras / esforço**

- Ainda introduz referências indiretas e identidade, só que representadas como handles/índices.
- Exige um heap/arena no runtime e operações para criar, localizar e modificar nós.
- A linguagem precisa expor algum modo de criar nós e representar filhos ausentes; apenas guardar índices não define uma sintaxe amigável.
- Atribuição/cópia passa a exigir decisão entre copiar o nó e copiar o identificador que aponta para ele.

**Esforço relativo:** médio/alto. Pode ser menor e mais seguro que ponteiros genéricos, mas exige desenhar uma abstração de referências/handles.

#### Opção E — Representar filhos com arrays dinâmicos de nós, sem ponteiros explícitos

Uma alternativa para o exemplo binário seria cada nó guardar uma lista de filhos, vazia para uma folha e com dois elementos para um nó interno:

```c
struct No {
	int valor;
	struct No[] filhos;
};
```

Isso não adiciona ponteiros à sintaxe MiniC. A indireção vem do próprio array dinâmico: em Rust, o runtime atual representa arrays como `Vec<Value>`, cujo conteúdo fica armazenado separadamente do valor que contém o vetor. O array pode estar vazio numa folha e conter dois nós num nó interno.

**Prós**

- Pode representar árvores finitas de profundidade variável sem expor ponteiros ou referências na linguagem.
- Reaproveita a ideia de array que já existe em MiniC e o armazenamento dinâmico que já existe no interpreter.
- A forma vazia do array fornece uma representação natural para “sem filhos”; não é necessário um valor `null` para cada ponteiro filho.
- Evita introduzir identidade compartilhada: em princípio, os filhos são valores contidos na árvore, não aliases para os mesmos nós.

**Contras / trabalho ainda necessário**

- O parser e o type checker atuais só conhecem arrays de tipos nativos; arrays de `struct` precisariam ser generalizados.
- O MiniC ainda não tem inicializador de struct. Seria preciso decidir como criar `No` e preencher `valor` e `filhos`.
- Arrays são de tamanho fixo depois de criados no runtime atual: indexação pode substituir um elemento, mas não há operação de append/resize. Isso pode bastar para árvores construídas completas na inicialização, mas é insuficiente para acrescentar filhos dinamicamente sem uma nova operação ou função nativa.
- Para uma árvore estritamente binária, o type checker/runtime precisaria impor que `filhos` tenha no máximo dois itens, ou a linguagem aceitaria uma árvore com número arbitrário de filhos.
- Como os valores são clonados em várias operações do interpreter, cópia e atualização de subárvores podem ter semântica de cópia por valor e custo crescente; isso é diferente de ponteiros que compartilham nós.

**Esforço relativo:** médio. Reaproveita arrays dinâmicos em vez de criar uma feature genérica de referência, mas requer arrays de structs, construção de structs e uma estratégia de mutação/adição de filhos. Pode ser uma forma didática de árvore se não for necessário compartilhar o mesmo nó em vários lugares.

#### Opção F — Limitar a profundidade da árvore

Impor, por exemplo, profundidade máxima de 10.

**Prós**

- Parece limitar recursos e pode ser útil como proteção contra estruturas excessivas.

**Contras**

- Não resolve por si só a representação por valor: ainda falta representar filhos ausentes e interromper a expansão recursiva do tipo.
- Se o limite for aplicado ao runtime, ainda será necessário um mecanismo de indireção para alcançar nós filhos.
- Uma profundidade fixa limita o uso e não corresponde a árvores arbitrárias.

**Conclusão técnica:** um limite de profundidade pode ser uma restrição adicional depois de escolher uma representação indireta; não é substituto para ponteiros, referências, handles ou uma coleção dinâmica usada como camada de indireção.

#### Arrays de structs

Arrays de struct são uma questão relacionada, mas distinta de um campo escalar `struct OutroTipo`. O suporte atual do parser reconhece arrays de tipos nativos; incluir `struct Nome[]` pode ser decidido como parte desta decisão ou ficar para uma extensão posterior. Um array dinâmico de structs autorreferenciais não é automaticamente impossível num interpretador: valores finitos poderiam terminar em arrays vazios, como na opção E. Porém, isso exigiria definir a representação e a inicialização desses valores; arrays dão armazenamento indireto no runtime, mas não são ponteiros/referências com identidade compartilhada expostos na linguagem.

#### Recomendação

Para structs simples e finitos, a opção A seria viável sem ponteiros. Para uma árvore binária de profundidade variável, composição direta por valor não basta; seria necessário algum mecanismo de indireção. Há ao menos três caminhos: referências gerenciadas/genéricas (opção C), handles numa arena (opção D) ou arrays dinâmicos de nós (opção E). A opção B foi escolhida para manter a feature inicial menor; as demais alternativas ficam como possibilidades futuras, não como parte desta implementação. A opção F, limite de profundidade, não substitui nenhum mecanismo de indireção.

#### Motivo registrado para a apresentação

Essa é uma justificativa didática defensável: o escopo inicial ensina declaração de structs, campos dos tipos já existentes e acesso a campos sem também exigir ponteiros, referências ou armazenamento recursivo. O projeto declara explicitamente a limitação — structs não podem conter campos de tipo struct — e explica que estruturas recursivas, como árvores, dependem de uma futura feature de indireção ou arrays de structs.

Não dá para garantir como um professor específico avaliará a escolha, mas ela é tecnicamente coerente e fácil de explicar: a limitação é consciente, decorre do modelo atual da linguagem e não é apresentada como uma impossibilidade geral. Arrays de structs e árvores binárias poderão ser avaliados como extensão separada, se houver necessidade.

### Decisão 9 — Permitir structs sem campos

**Status:** decidida — exigir pelo menos um campo.

#### Decisão e justificativa

O autor escolheu a opção A porque um struct sem campos não tem um valor ou dado claro a representar no escopo atual. A regra também acompanha C padrão, que exige ao menos um membro em uma definição de struct. Tipos marcadores vazios, como os permitidos em C++, ficam fora do escopo inicial.

Esta questão é diferente de declaração incompleta. `struct Pessoa;` não fornece a definição dos campos e já foi excluído; `struct Pessoa {};` fornece uma definição completa cuja lista de campos está vazia. Também é diferente de declarar uma variável struct sem inicializá-la: o primeiro caso pergunta se o tipo pode ter zero campos; o segundo pergunta como um objeto de um tipo definido recebe seu valor.

#### Comparação com C padrão

C padrão exige pelo menos um membro na definição de um `struct`; uma lista vazia como `struct Pessoa {};` não é uma definição ISO C válida (alguns compiladores aceitam como extensão). C++ permite tipos `struct`/`class` vazios. Portanto, proibir structs sem campos também mantém esta parte mais próxima do C padrão.

#### Opção A — Exigir pelo menos um campo — escolhida

Exemplo válido:

```c
struct Pessoa {
	int idade;
};
```

`struct Pessoa {};` seria erro de sintaxe ou validação.

**Prós**

- Segue C padrão e mantém a ideia de struct como agrupamento de dados.
- Evita criar valores que não carregam informação nem têm campos acessíveis.
- A validação é pequena: a lista de campos não pode estar vazia.

**Contras**

- Não permite structs vazios como tipos marcadores, recurso possível em C++.
- Se algum uso futuro precisar de um tipo marcador, será necessária uma extensão.

#### Opção B — Permitir zero campos

Exemplo:

```c
struct Marcador {};
```

**Prós**

- Permite usar o tipo como marcador ou como valor unitário.
- Aproxima essa escolha de C++ e de algumas linguagens com tipos unitários.

**Contras**

- Diverge de C padrão.
- Não há caso de uso identificado no MiniC; a variável não teria campos para ler ou modificar.
- A igualdade, impressão, tamanho e inicialização de valores vazios ainda precisariam de semântica definida.

#### Recomendação registrada antes da decisão

Sem necessidade de tipo marcador, a opção A parecia mais coerente com o escopo e com a inspiração em C padrão. A decisão é separada da inicialização de variáveis; structs com campos serão inicializados com valores padrão conforme a decisão 7.

#### Narrativa para apresentação

> Também avaliamos se uma definição completa poderia ter zero campos. Escolhemos exigir pelo menos um: um struct vazio não tinha um propósito claro para esta feature, e C padrão também exige ao menos um membro. Assim mantivemos um escopo menor e evitamos definir a semântica de valores sem campos.

### Decisão 10 — Sintaxe de acesso aos campos

**Status:** decidida — usar o operador ponto (`.`).

#### Decisão e justificativa

O autor escolheu a opção A. Acesso a campo será escrito como `p.idade`; a mesma expressão pode ser usada para leitura e como destino de atribuição (`p.idade = 42`). Como MiniC não tem ponteiros, o operador `->` fica fora do escopo.

O acesso usa ponto entre o valor struct e o nome do campo.

#### Opção A — Operador ponto: `p.idade` — escolhida

```c
print(p.idade);
p.idade = 42;
```

**Prós**

- É a notação familiar de C para acessar membro de um objeto struct.
- Como MiniC não tem ponteiros, só precisamos considerar acesso com `.`, não `->`.
- Funciona bem como postfix de expressões e combina naturalmente com indexação: `pontos[0].x`.
- A mesma forma pode representar leitura e aparecer como destino de atribuição.

**Contras / pontos a implementar**

- A AST precisa representar um acesso a campo, além dos nós atuais `Ident` e `Index`.
- O parser de expressões precisa aceitar o postfix `.` seguido de um identificador; o parser de lvalue/atribuição também precisa aceitar essa cadeia.
- O type checker tem de verificar que a expressão-base é um struct e que o campo existe.

#### Alternativa considerada — Função ou operação explícita de acesso

Forma ilustrativa: `get(p, "idade")` para leitura e uma forma distinta para escrita.

**Prós**

- Não introduz um operador postfix novo na gramática.
- Poderia tratar acesso como operação de biblioteca.

**Contras**

- É pouco natural para uma linguagem C-like.
- Uma função comum não é um destino de atribuição; escrita exigiria uma operação adicional ou comportamento especial.
- O nome do campo como string adia erros de campo inexistente para análise especial ou runtime.

#### Narrativa para apresentação

> Escolhemos `.` para acessar campos por ser a notação familiar de C e funcionar tanto para leitura quanto para atribuição. O parser poderá compô-lo com outros postfixes, como indexação. Não adotamos uma função `get`, pois ela não serviria naturalmente como destino de atribuição.

### Decisão 11 — Visibilidade e ordem das definições de `struct`

**Status:** decidida — uma definição completa de `struct` deve aparecer antes de qualquer uso do tipo.

#### Decisão e justificativa

O autor escolheu a opção A porque é uma regra simples e familiar de C: declarar o tipo antes de usá-lo. Isso também dá significado semântico à ordem dos itens que decidimos preservar no programa. Como declarações incompletas foram excluídas, não haverá uma declaração antecipada `struct Pessoa;` para anunciar o nome antes da definição completa.

Como a lista de itens de topo preserva a ordem, a regra escolhida usa essa sequência para determinar quando cada tipo se torna visível. A alternativa rejeitada permitiria o uso antes da definição.

#### Opção A — Exigir que o `struct` esteja definido antes do uso — escolhida

```c
struct Pessoa {
	int idade;
};

void cumprimentar(struct Pessoa p) {
	print(p.idade);
}
```

**Prós**

- Regra simples de explicar: o tipo precisa estar definido antes de ser usado.
- Aproxima-se da visibilidade textual de C para tipos completos.
- Combina com a lista ordenada de itens e com a decisão de não suportar declarações incompletas.
- O checker pode acumular definições de tipos à medida que percorre os itens do programa.

**Contras**

- A ordem do arquivo passa a afetar a validade do programa; mover uma função acima da definição pode torná-la inválida.
- Funções não podem usar um tipo definido mais tarde, mesmo que não haja dependência recursiva entre structs.

#### Opção B — Permitir uso antes da definição (referências para frente)

```c
void cumprimentar(struct Pessoa p) {
	print(p.idade);
}

struct Pessoa {
	int idade;
};
```

**Prós**

- A ordem entre funções e definições de struct não limita o programador.
- O checker pode primeiro coletar todas as definições completas e depois verificar funções.
- Como structs dentro de structs foram proibidos, não há dependências entre definições de struct neste escopo.

**Contras**

- Diverge da regra textual simples de declarar um tipo antes de usá-lo.
- Exige uma passagem anterior de coleta/registro de structs antes da verificação de funções.
- A lista ordenada continua preservada para outros fins, mas não determina a visibilidade dos tipos.

#### Alternativa considerada — Permitir uso antes da definição

O programa poderia coletar todas as definições completas antes de verificar qualquer função. Isso permitiria referências para frente, mas não foi escolhido porque o autor prefere a regra mais simples de declaração antes do uso.

#### Consequência para o checker

Ao percorrer os itens do programa na ordem, o checker registra um tipo struct quando encontra sua definição. Uma função ou declaração só pode usar tipos que já estejam registrados naquele ponto. Tipos definidos depois não são visíveis retroativamente.

Em termos simples, a definição é global para o restante do arquivo, não local a um bloco. Por exemplo:

```c
struct Pessoa { int idade; };
void usar(struct Pessoa p) { print(p.idade); } // válido: a definição veio antes
```

Se `usar` viesse antes da definição, seria rejeitada. Não haverá declaração de struct dentro de funções ou blocos.

Como o espaço de nomes foi escolhido como compartilhado (decisão 12), o nome do tipo também não pode colidir com nome de variável ou função no escopo em que ambos estiverem visíveis. Como os tipos são declarados no nível superior, eles permanecem visíveis dentro dos blocos seguintes; assim, uma variável local com o mesmo nome de uma tag já definida também será rejeitada. Esta é a interpretação simples da regra de espaço de nomes compartilhado.

#### Narrativa para apresentação

> Escolhemos exigir que cada struct seja completamente definido antes de seu uso. Essa regra segue uma expectativa familiar de C, combina com a ordem preservada no AST e evita uma passagem global de coleta de tipos ou declarações incompletas.

### Decisão 12 — Espaço de nomes para structs

**Status:** decidida — nomes de struct compartilham o espaço de nomes com variáveis e funções.

#### Decisão e justificativa

O autor escolheu a opção A para manter a linguagem e sua implementação simples, pois o escopo do projeto já é amplo. No mesmo escopo, um identificador não pode nomear tanto um struct quanto uma variável ou função.

Isso significa que o nome usado depois de `struct` não pode coincidir, no mesmo escopo, com o nome de uma variável ou função. Por exemplo, a declaração a seguir seria rejeitada:

```c
struct Pessoa {
	int idade;
};

int Pessoa = 3;
```

#### Opção A — Um espaço de nomes compartilhado — escolhida

O nome `Pessoa` não poderia ser reutilizado por uma variável ou função no mesmo escopo.

**Prós**

- Regra mais simples de explicar e implementar: nomes não colidem entre categorias.
- Pode se encaixar em uma tabela de símbolos única, semelhante ao ambiente genérico atual do MiniC.

**Contras**

- Proíbe nomes que poderiam ser distinguidos sintaticamente por `struct Pessoa` versus `Pessoa`.
- É mais restritivo que C, que mantém tags de struct em um espaço de nomes separado dos identificadores ordinários.

#### Opção B — Espaço de nomes separado para tags de struct

O nome depois de `struct` identifica uma tag; nomes de variável/função continuam no espaço ordinário. Assim, `struct Pessoa` e uma variável chamada `Pessoa` podem coexistir.

**Prós**

- Segue a separação de tags e identificadores ordinários de C.
- A palavra-chave obrigatória `struct` deixa explícita a categoria do nome no código-fonte.

**Contras**

- Exige manter e consultar categorias de nomes separadas no checker.
- É uma regra adicional de escopo/nomeação que o MiniC ainda não precisa para os exemplos básicos.

#### Alternativa considerada — Espaço de nomes separado para tags de struct

Em C, tags de struct ocupam um espaço de nomes separado dos identificadores ordinários, então `struct Pessoa` e uma variável `Pessoa` podem coexistir. Essa alternativa foi considerada, mas rejeitada para manter MiniC simples.

#### Consequência

O registro de nomes do checker deverá rejeitar uma colisão entre um nome de tipo struct e uma variável/função no mesmo espaço de nomes. A política de escopos continua seguindo as regras gerais já usadas por MiniC.

#### Narrativa para apresentação

> Escolhemos compartilhar o espaço de nomes entre structs, variáveis e funções. C separa tags de struct, mas reproduzir essa distinção exigiria mais regras e estruturas no checker; para o escopo didático atual, a simplicidade foi considerada mais valiosa.

### Decisão 13 — Nomes de campos duplicados

**Status:** decidida — rejeitar campos com nomes duplicados dentro do mesmo struct.

#### Decisão e justificativa

O autor confirmou a opção A. Cada campo precisa ter um nome único dentro da definição. Isso evita ambiguidade em acessos como `p.idade`, acompanha C e é simples de verificar durante a análise do struct.

Uma definição pode tentar declarar o mesmo campo mais de uma vez?

```c
struct Pessoa {
	int idade;
	float idade;
};
```

#### Opção A — Rejeitar nomes de campo duplicados — escolhida

Cada campo do struct precisa ter nome único.

**Prós**

- Elimina ambiguidade em acessos como `p.idade`.
- Segue a regra de C para membros ordinários de uma mesma struct.
- Validação simples durante a análise da definição.

**Contras**

- Não há desvantagem prática identificada no escopo atual; renomear campos é a solução para representar dados distintos.

#### Opção B — Permitir nomes duplicados

**Prós**

- Nenhum benefício claro identificado para MiniC.

**Contras**

- `p.idade` ficaria ambíguo ou exigiria uma regra adicional de seleção.
- O checker/interpreter e mensagens de erro ficariam mais complexos.
- Diverge de C, que não permite membros ordinários duplicados no mesmo struct.

#### Alternativa considerada — Permitir nomes duplicados

Permitir nomes duplicados exigiria uma regra extra para resolver acessos ambíguos e não oferece benefício identificado, então foi rejeitado.

#### Narrativa para apresentação

> Exigimos nomes únicos para os campos de cada struct. Dois campos com o mesmo nome tornariam `obj.campo` ambíguo; rejeitar duplicatas mantém o acesso simples e segue a convenção de C.

### Decisão 14 — Valor padrão de campos array

**Status:** decidida — arrays começam vazios.

#### Decisão e justificativa

O autor escolheu a opção A. Um campo array inicializado implicitamente começa como array vazio. Para arrays multidimensionais, o array externo começa vazio também (por exemplo, `int[][]` começa sem linhas). O tipo não especifica um tamanho, então o MiniC não inventará comprimento nem elementos.

A decisão 7 escolheu inicialização automática dos campos. Para campos escalares, os valores padrão considerados são `0`, `0.0`, `false` e `""`; a decisão 14 definiu que campos array começam vazios.

**Alcance ampliado pela revisão da decisão 7:** a regra "array começa vazio" não é exclusiva de campos. Ela vale para qualquer array declarado sem inicializador, inclusive a variável comum `int[] a;`, que passa a ser válida e vale `[]`. O mecanismo é o mesmo; só o alcance mudou. Ver a revisão em [Decisão 7](#decisão-7--inicialização-de-variáveis).

#### Opção A — Array vazio — escolhida

Um campo `int[] valores;` começa como `[]`. Arrays multidimensionais também começam vazios no nível externo; por exemplo, `int[][] matriz` começa sem linhas.

**Prós**

- É determinístico e usa o menor armazenamento.
- Segue uma expectativa comum para coleções dinâmicas.
- Não inventa comprimento nem elementos que o autor não especificou.

**Contras**

- Acesso por índice falha até que elementos existam; o MiniC atual não oferece operação de append/resize.
- O campo array começa válido, mas pode não ser imediatamente útil para guardar elementos sem substituição por índice ou inicialização explícita futura.

#### Opção B — Arrays com elementos padrão

Inicializar o array com um ou mais elementos do valor padrão de seu tipo, por exemplo `[0]` ou uma quantidade fixa definida pela linguagem.

**Prós**

- Poderia permitir acesso por índice logo após a declaração, dependendo do tamanho escolhido.

**Contras**

- O tipo `T[]` não informa tamanho; a linguagem teria de inventar um comprimento padrão global ou acrescentar tamanho à declaração.
- Um comprimento arbitrário pode desperdiçar memória e surpreender quem espera array vazio.
- Arrays multidimensionais exigiriam decidir o tamanho de cada dimensão.

#### Alternativa considerada — Arrays com elementos padrão

Arrays preenchidos com valores padrão exigiriam escolher um comprimento não indicado no tipo. Essa alternativa foi rejeitada; o problema de não haver append/resize pode ser tratado em uma decisão futura sobre operações de array.

### Decisão 15 — Uso de structs em assinaturas de função

**Status:** decidida — structs não podem ser parâmetros nem tipos de retorno de funções nesta versão.

#### Decisão e justificativa

O autor escolheu a opção C para reduzir o escopo inicial. A feature cobre definição de structs, variáveis struct e acesso/modificação de seus campos, mas structs não atravessam a interface de funções.

Esta restrição evita, por enquanto, implementar passagem, retorno e cópia de valores struct entre chamadas. Tipos nativos continuam permitidos em parâmetros e retornos normalmente.

#### Opção A — Permitir structs como parâmetros e retornos por valor — não escolhida

Exemplo ilustrativo:

```c
struct Pessoa {
	int idade;
};

struct Pessoa aniversario(struct Pessoa p) {
	p.idade = p.idade + 1;
	return p;
}
```

**Prós**

- Trata structs como valores de primeira classe, de modo uniforme com tipos primitivos.
- Permite funções reutilizáveis que recebem ou produzem structs.
- Sem referências na linguagem, passagem por valor mantém semântica simples: a função recebe uma cópia; mudanças no parâmetro não alteram automaticamente o argumento original.

**Contras / esforço**

- O type checker precisa aceitar `struct Nome` nas listas de parâmetros e tipos de retorno e verificar compatibilidade.
- O interpreter precisa passar, retornar e copiar valores struct corretamente.
- Atribuição/retorno de estruturas grandes pode copiar dados; isso é aceitável para um MiniC didático, mas deve ser entendido como semântica por valor.

#### Opção B — Permitir structs como parâmetros, mas não como retorno — não escolhida

**Prós**

- Permite funções consultar ou modificar a cópia local de um struct recebido.
- Reduz parte do trabalho de retorno e de composição de expressões.

**Contras**

- É uma restrição pouco uniforme: uma função pode receber o tipo, mas não devolvê-lo.
- Sem referências, uma função não consegue atualizar o struct do chamador; retorná-lo por valor seria a forma natural de produzir o valor alterado.

#### Opção C — Não permitir structs em parâmetros nem retornos nesta versão — escolhida

**Prós**

- Menor escopo de type checker e interpreter; structs ficam restritos a variáveis locais e acesso a campos.

**Contras**

- Structs não podem ser usados como interface entre funções, limitando bastante sua utilidade.
- O parser poderá reconhecer a grafia no contexto de parâmetro, mas o type checker precisará rejeitá-la, ou a gramática terá que restringi-la.

#### Consequência para parser e type checker

O parser/type checker deve rejeitar `struct Nome` em parâmetros e tipos de retorno. O tipo pode ser usado em declarações de variáveis dentro de funções; o valor permanece local ao corpo, e seus campos podem ser acessados ali.

#### Narrativa para apresentação

> Para manter a primeira versão pequena, decidimos não permitir structs em parâmetros nem retornos de função. O suporte inicial fica concentrado em definir tipos, declarar variáveis locais e acessar seus campos. A integração com chamadas pode ser considerada depois.

### Decisão 16 — Igualdade de structs

**Status:** decidida — `==` e `!=` não aceitam valores struct.

Comparar campos individualmente continua válido, por exemplo `a.idade == b.idade`. Não será implementada igualdade estrutural nesta versão.

### Decisão 17 — Impressão de structs

**Status:** decidida — `print` não aceita um struct completo.

Campos individuais podem ser impressos, por exemplo `print(p.idade)`. Não é necessário definir um formato textual para structs.

### Decisão 18 — Atribuição entre structs inteiros

**Status:** decidida — proibir atribuição struct-para-struct.

Uma instrução como `a = b;` será rejeitada se ambos forem valores struct. Atribuições de campos continuam permitidas, por exemplo `a.idade = b.idade;`. Structs não são aceitos em parâmetros nem retornos de função (decisão 15), então a cópia de agregados fica fora desta versão.

### Decisão 19 — Nome único e redefinição de structs

**Status:** decidida — cada nome de struct terá uma única definição no programa.

Uma segunda definição completa com o mesmo nome será rejeitada. O tipo fica visível depois da definição, conforme a decisão 11.

## Inventário de decisões e questões restantes

As decisões 1–19 estão fechadas. Restam principalmente detalhes técnicos de implementação, não novas escolhas de sintaxe ou semântica.

### Questões técnicas a avaliar durante implementação

1. **Consumo completo da entrada:** `program()` retorna o texto restante, e o CLI atualmente não verifica que ele esteja vazio. Recomendo tornar o parsing do programa estrito para uma declaração top-level malformada gerar erro, em vez de ser silenciosamente ignorada. Isso é uma correção de diagnóstico do parser, transversal à feature.
2. **Combinação de postfixes:** testar `p.valores[i]`. `lista[i].campo` continua fora porque arrays de structs foram excluídos.
3. **Localização de erros:** o AST atual não guarda linha/coluna. Para manter simples, mensagens sem localização podem bastar nesta feature; spans podem ficar para depois.
4. **Limites de contexto:** testar que structs são aceitos em declarações de variáveis, mas rejeitados em parâmetros e tipos de retorno de função (decisão 15).

### Avaliação de simplicidade e possíveis cortes

As escolhas com maior impacto aproximado são:

1. **Defaults automáticos (decisões 7 e 14):** exige construir um valor de cada tipo de campo, mas os defaults estão explícitos e são diretos.
2. **Acesso e atribuição de campos:** são o uso central de structs e devem ser mantidos.
3. **Proibições de igualdade, impressão e atribuição de structs inteiros (decisões 16–18):** já simplificam bastante o runtime, pois não será necessário definir comparação, formato de impressão ou cópia por assignment.

O corte de structs em assinaturas de função já foi escolhido. Eu manteria a declaração, os defaults e o acesso a campos: juntos formam o núcleo útil da feature.

Este inventário é um mapa de trabalho, não uma decisão de arquitetura. As decisões da linguagem estão fechadas; os itens restantes são detalhes técnicos e de diagnóstico a resolver durante a implementação.

---

### Decisão — Reatribuição e alteração de campos em structs

**Status:** decidida — após a declaração, alterações em variáveis `struct` são feitas exclusivamente campo a campo; reatribuição completa com literal (`p = Pessoa { ... };`) não é permitida.

- **Permitido:** alterar campos individualmente via operador ponto, por exemplo:
  ```c
  pessoa.nome = "joao";
  ```
- **Proibido:** reatribuir a variável struct inteira com um literal após a declaração (`p = Pessoa { ... };`), assim como atribuir outro struct (`a = b;`, conforme a Decisão 18).

**Justificativa:** um `struct` só recebe um valor inteiro na declaração via literal (`Pessoa p = Pessoa { ... };`) ou nasce sem inicializador com campos *undefined* (`Pessoa p;`). Depois disso, toda modificação ocorre campo a campo (`pessoa.nome = "joao";`), mantendo o modelo simples e coerente com a proibição de atribuição de structs inteiros.
