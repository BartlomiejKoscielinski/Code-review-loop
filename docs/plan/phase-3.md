# Faza 3: AI i semantyka

**Cel:** AI dokłada się do zmierzonego punktu odniesienia z faz 1 i 2. Claude Code daje briefing, hipotezy per krok, strażnika testów, pytania o zaznaczenie i redakcję komentarzy. Roslyn daje semantykę C#, a wykonanie kodu daje dowody: testy z MR na starym kodzie, pokrycie zmienionych linii, mutacje.

**Zasada z `docs/input/plan.md`:** najpierw reguły deterministyczne, potem AI. Uwaga AI to hipoteza z dowodem i nic nie trafia do GitLaba bez akceptacji człowieka.

Faza ma trzy niezależne strumienie: 3.1 AI, 3.2 Roslyn, 3.3 dowody z wykonania. Wspólny jest tylko kamień 3.0. Strumienie 3.2 i 3.3 zależą od ADR z P2-30.

## Fakty, na których opiera się faza

Sprawdzone przez `fact-checker` 2026-10-10 w dokumentacji Claude Code ([CLI reference](https://code.claude.com/docs/en/cli-reference), [headless](https://code.claude.com/docs/en/headless), [permissions](https://code.claude.com/docs/en/permissions), [memory](https://code.claude.com/docs/en/memory)):

- `--bare` nie czyta loginu OAuth ani keychaina, wymaga klucza API. Przy seatach Claude Code nie da się go użyć.
- `-p` bez `--bare` uruchamia hooki z `.claude/settings.json` i serwery z `.mcp.json` katalogu roboczego.
- Z katalogu dodanego przez `--add-dir` nie wczytują się hooki ani `CLAUDE.md` (domyślnie). **Wczytują się jednak skille, komendy i subagenci z `.claude/`.** Dlatego kodu MR nie podajemy przez `--add-dir` wprost (patrz P3-01).
- Do zawężenia sesji służą:
  - `--tools` (ogranicza narzędzia wbudowane);
  - `--disallowedTools "mcp__*"`;
  - `--strict-mcp-config` z `--mcp-config`;
  - `--setting-sources`;
  - `--permission-mode dontAsk`.

  `--allowedTools` tylko zatwierdza narzędzia bez pytania, nie ogranicza ich zestawu.
- `--output-format json` z `--json-schema` zwraca wynik w `structured_output`, a JSON ma `session_id` i `total_cost_usd` (szacunek po stronie klienta).
- `--max-budget-usd` liczy od tego szacunku, a wydatek może przekroczyć limit.
- `MSBuildWorkspace` ([kod Roslyn](https://github.com/dotnet/roslyn/blob/main/src/Workspaces/MSBuild/BuildHost/Build/ProjectBuildManager.cs)) wykonuje design-time build (targety `Compile`/`CoreCompile`) w procesie BuildHost, czyli uruchamia taski i targety z repozytorium.

## Spike fazy

| ID | Nazwa | Pytanie | Kryterium sukcesu | Limit | Odblokowuje |
| --- | --- | --- | --- | --- | --- |
| S4 | `dotnet-sidecar-packaging` | Jak spakować sidecar .NET 10 z Roslyn jako `externalBin` Tauri i czy `Microsoft.Build.Locator` znajdzie MSBuild na maszynach zespołu? Wymaga zainstalowanego SDK, a zachowanie w trybach publikacji jest **NIEZWERYFIKOWANE**. | sidecar startuje z instalatora na czystym Windows z .NET SDK i ładuje projekt testowy; na maszynie bez SDK daje czytelny komunikat | 6 h | P3-11, P3-12 |

## Kamienie milowe

| Kamień | Zadania | Po nim działa |
| --- | --- | --- |
| 3.0 Fundament AI | P3-01 … P3-04b, P3-21, P3-22 | izolowane, tanie, cache'owane wywołania `claude -p` z walidacją wyniku i zestawem ewaluacyjnym |
| 3.1 Funkcje AI | P3-05 … P3-10b | briefing, hipotezy, strażnik testów, pytania, redakcja, AI w interdiffie |
| 3.2 Roslyn | P3-11 … P3-16 | sidecar, graf semantyczny, wywołania, bindingi XAML, duplikaty |
| 3.3 Dowody z wykonania | P3-17 … P3-20 | testy na base, pokrycie, mutacje, brakujące współzmiany |

## Zadania

### 3.0 Fundament AI

#### P3-01 · Spike i ADR: izolacja `claude -p` na seatach

- **Zależy od:** —
- **Zakres:** spike `claude-cli-readonly-isolation` na Windows i ADR. Hipoteza do sprawdzenia:
  - sesja startuje w pustym katalogu roboczym narzędzia;
  - flagi: `--setting-sources user`, `--settings` z `disableAllHooks`, `--strict-mcp-config` z pustą konfiguracją MCP, `--tools "Read,Grep,Glob"`, `--disallowedTools "mcp__*"`, `--permission-mode dontAsk`, `--disable-slash-commands`;
  - kod MR to migawka wybranych plików w katalogu cache narzędzia, bez katalogów `.claude/`, plików `CLAUDE.md`, `CLAUDE.local.md` i `.mcp.json`. Dopiero ten katalog trafia do `--add-dir`;
  - środowisko procesu z listy dozwolonych zmiennych (bez tokenu GitLaba).

  Spike próbuje złamać izolację repozytorium testowym z wrogim `.claude/settings.json` (hook zapisujący plik), skillem, subagentem, `.mcp.json`, `CLAUDE.md` z poleceniem i prompt injection w kodzie.

  Spike sprawdza też:
  - które ustawienia z poziomu użytkownika (`~/.claude`: hooki, pluginy, skille, subagenci, MCP) nadal się ładują przy tych flagach;
  - jak uruchomić `claude` na Windows bez powłoki. Instalacja przez npm daje shim `.cmd`, który wymaga `cmd.exe`. Wymagany wariant instalacji i sposób wykrywania pliku wykonywalnego są **NIEZWERYFIKOWANE**.
- **Kryteria akceptacji:**
  - spike zamknięty z logami: żaden wrogi element się nie wykonał ani nie wczytał (sprawdzenie plików-pułapek i transkryptu `stream-json`), a próba użycia `Bash`, `Edit` i `WebFetch` kończy się odmową;
  - próba `Read` poza migawką (repo dewelopera, `%USERPROFILE%`, baza Lupy) kończy się odmową;
  - kryterium twarde: z poziomu użytkownika nie ładuje się żaden hook, plugin, skill, subagent ani serwer MCP. Jeśli flagi tego nie zapewniają, ADR opisuje wykrywanie takiej konfiguracji i odmowę startu sesji AI z komunikatem (pytanie Q11 w `open-questions.md`);
  - ADR ustala sposób uruchamiania `claude` bez `cmd.exe`;
  - ADR „Proponowany” z przeglądem `security-reviewer`, zaakceptowany przez człowieka przed P3-02;
  - potwierdzenie admina Claude Enterprise, że użycie seatów przez narzędzie wewnętrzne jest zgodne z umową, i ustalenie retencji danych (pytania z `open-questions.md`); wynik zapisany w ADR.
- **Rozmiar:** spike + ADR. Limit spike'a: 8 h.
- **Przegląd:** security.

#### P3-02 · Runner `claude -p`

- **Zależy od:** P3-01 (zaakceptowany ADR)
- **Zakres:** `lupa-ai`:
  - abstrakcja dostawcy z implementacją `ClaudeCodeCli` według ADR; dostawca z kluczem API jako zapas, bez implementacji;
  - uruchomienie bez powłoki, z flagami z ADR, z listą dozwolonych zmiennych środowiska, limitem czasu i anulowaniem (zabicie drzewa procesów);
  - parsowanie `stream-json` do zdarzeń postępu, wynik `structured_output` walidowany schematem;
  - wykrycie braku `claude` lub braku logowania z komunikatem dla użytkownika;
  - atrapa `claude` w `lupa-testkit` (mały binarny program zwracający zapisane odpowiedzi).
- **Kryteria akceptacji:**
  - testy na atrapie: argumenty procesu dokładnie według ADR (porównanie listy); środowisko bez tokenu; odpowiedź niezgodna ze schematem daje błąd, nie częściowy wynik; przekroczenie czasu zabija proces; zdarzenia postępu docierają przed końcem;
  - test: treść MR nigdy nie trafia do argumentów (prompt idzie przez stdin albo plik w katalogu cache).
- **Rozmiar:** ~380 LOC.
- **Przegląd:** security.

#### P3-03 · Prompty, delimitacja i walidacja wyników

- **Zależy od:** P3-02, P3-21, P2-04
- **Zakres:**
  - prompty z `.review/prompts/*.md` z gałęzi docelowej, z wbudowanymi domyślnymi i wersją promptu;
  - budowa promptu: instrukcje oddzielone od danych MR (opis, diff, komentarze, nazwy plików) blokami z losowym znacznikiem na każde wywołanie, z neutralizacją znacznika wewnątrz danych i z jawną informacją, że dane to nie polecenia;
  - schematy JSON wyników dla każdego zadania AI;
  - walidacja semantyczna:
    - linie muszą istnieć w diffie celu;
    - pliki muszą należeć do celu;
    - etykiety z listy Conventional Comments;
    - limity długości;
    - odpowiedź bez wymaganych pól jest odrzucana.
- **Kryteria akceptacji:** testy: dane MR zawierające znacznik końca bloku nie zamykają bloku; wynik wskazujący linię spoza diffu jest odrzucony z logiem; prompt z `.review/` z gałęzi MR nie jest używany.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security.

#### P3-04a · Cache i limity kosztu

- **Zależy od:** P3-03
- **Zakres:** wyniki kluczowane (`head_sha`, wersja promptu, model, hash wejścia). `--max-budget-usd` per wywołanie i dzienny limit w ustawieniach z `total_cost_usd`, w UI oznaczone jako szacunek. Licznik kosztu w ustawieniach.
- **Kryteria akceptacji:** testy: ponowne otwarcie celu nie uruchamia `claude` (atrapa liczy wywołania); zmiana wersji promptu liczy nowy wynik; przekroczenie dziennego limitu blokuje nowe wywołania z komunikatem.
- **Rozmiar:** ~250 LOC.

#### P3-04b · Runner ewaluacji

- **Zależy od:** P3-04a
- **Zakres:** `lupa-eval` (binarka w `crates/lupa-ai`), format zestawu w `.review/eval/` (para commitów w repozytorium testowym + oczekiwane findingi z liniami i etykietą), metryki precyzja i recall per zadanie i wersja promptu, raport Markdown. Wymaga prawdziwego `claude`, więc działa ręcznie, poza CI. Syntetyczny zestaw 10 przypadków w `crates/lupa-testkit` (bez kodu zespołu).
- **Kryteria akceptacji:** `lupa-eval` na atrapie z zapisanymi odpowiedziami daje znane wartości precyzji i recall; niepoprawny plik zestawu daje błąd z nazwą przypadku.
- **Rozmiar:** ~300 LOC.

#### P3-21 · Migawka plików MR dla Claude

- **Zależy od:** P3-01 (zaakceptowany ADR), P1-06
- **Zakres:**
  - katalog w cache narzędzia z wybranymi plikami celu (pliki z diffu i pliki wskazane przez trasę lub graf);
  - pliki kopiowane z obiektów Gita, nie z working tree ani worktree. Dla celu `WorkingTree` treść z migawki P1-07;
  - wykluczenia bez względu na wielkość liter: `.claude/`, `CLAUDE.md`, `CLAUDE.local.md`, `.mcp.json` i inne nazwy z ADR;
  - bez symlinków i junction: obiekt linku z Gita jest pomijany;
  - limity rozmiaru i liczby plików, manifest migawki.
- **Kryteria akceptacji:** testy na repo z `lupa-testkit`: `.Claude/Settings.json` i `claude.md` są pominięte; symlink do katalogu użytkownika nie daje dostępu do celu linku; plik powyżej limitu jest pominięty z wpisem w manifeście; ścieżki z `..` nie wychodzą poza katalog migawki.
- **Rozmiar:** ~250 LOC.
- **Przegląd:** security.

#### P3-22 · Zestaw ewaluacyjny zespołu

- **Zależy od:** P3-04b
- **Zakres:** 20 MR z kodu MAUI z wstawionymi błędami w repo MAUI w `.review/eval/`, w formacie z P3-04b, w tym odtworzenie !482. Błędy dobiera zespół, a Claude przygotowuje przypadki i oczekiwane findingi. Typowe błędy dla MAUI:
  - zdarzenia bez odsubskrybowania;
  - `async void`;
  - bindingi;
  - czas życia w DI;
  - uprawnienia Android.

  Właściciel: patrz Q12.
- **Kryteria akceptacji:** `lupa-eval` przechodzi na zestawie bez błędów formatu; raport bazowy (precyzja, recall) dla domyślnych promptów zapisany w repo MAUI; każdy przypadek ma opis błędu i linie.
- **Rozmiar:** dane testowe w repo MAUI, poza limitem LOC kodu.

### 3.1 Funkcje AI

#### P3-05 · Briefing AI

- **Zależy od:** P3-04a, P2-14
- **Zakres:**
  - **na ekranie briefingu:**
    - sekcje „Co diff faktycznie robi” i „Rozbieżności z opisem” z linkami do kroków trasy;
    - nazwy wątków i kroków oraz „dlaczego teraz” w miejsce nazw deterministycznych;
    - propozycja innej kolejności tylko jako sugestia do akceptacji;
  - **liczenie:** w tle po prefetchu (P2-03) z limitem kosztu;
  - **w skrzynce:** status „Claude Code · v3 · z cache” albo „wstępny, z opisu MR”.

  Kolejność kroków nadal liczy algorytm deterministyczny.
- **Kryteria akceptacji:** testy na atrapie: odpowiedź wskazująca nieistniejący krok jest odrzucona; brak AI (błąd, limit, brak logowania) zostawia briefing deterministyczny z informacją; testy komponentów dla stanów z makiety `Briefing`: „Rozbieżności z opisem · 2” z linkami do kroków 2 i 6, trasa z nazwami i „dlaczego teraz” od AI, briefing „wstępny, z opisu MR” przed zakończeniem pipeline'u, brak AI.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security (odpowiedź modelu jest niezaufana).

#### P3-06 · Hipotezy per krok: generowanie

- **Zależy od:** P3-05
- **Zakres:**
  - wywołanie per krok trasy z kontekstem kroku, pozycji checklisty i diagnostyk z SARIF (P2-33);
  - wynik: etykieta Conventional Comments, `blocking` lub `non-blocking`, linie, uzasadnienie, pewność, szkic komentarza;
  - twardy limit liczby na krok i próg pewności z `config.yaml`;
  - hipoteza powielająca regułę deterministyczną jest odrzucana. Duplikat to hipoteza w tym samym pliku, z liniami nakładającymi się na pozycję checklisty, diagnostykę SARIF albo zgłoszenie strażnika testów, w tej samej kategorii (mapa kategorii reguł na etykiety w `config.yaml`).
- **Kryteria akceptacji:** testy na atrapie: powyżej limitu zostają hipotezy o najwyższej pewności; poniżej progu nie są pokazywane; duplikat reguły checklisty odrzucony; wyniki deterministyczne dla tej samej odpowiedzi atrapy.
- **Rozmiar:** ~300 LOC.

#### P3-07 · Hipotezy per krok: interfejs

- **Zależy od:** P3-06, P2-16
- **Zakres:**
  - kropka w gutterze i wpis w prawej kolumnie, bez modali;
  - `a` „Akceptuj jako draft” (draft z oznaczeniem pochodzenia „z hipotezy AI”), `x` „Odrzuć” z powodem z krótkiej listy, „Pokaż dowód”, „Cofnij”;
  - nagłówek „Hipotezy AI · N” z informacją o wersji i cache.
- **Kryteria akceptacji:** testy komponentów dla stanów z makiety `Main` (akceptacja, odrzucenie z powodem, cofnięcie); test: hipoteza nie trafia do GitLaba bez akceptacji (test przepływu z atrapą API liczącą wywołania); powód odrzucenia zapisany lokalnie dla P4-03.
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security (odpowiedź modelu jest niezaufana).

#### P3-08 · Strażnik testów

- **Zależy od:** P3-06
- **Zakres:**
  - najpierw reguła deterministyczna: usunięte asercje (`Assert.`, `Should`, `Verify`), `Skip`, zmienione wartości oczekiwane, nowe `if` zależne od wartości testowych w kodzie produkcyjnym;
  - potem osobne pytanie do AI, gdy diff dotyka testów lub snapshotów;
  - wynik w briefingu, w kroku testów („strażnik testów: usunięta asercja”) i w bramce („zmiana w testach potwierdzona przez Ciebie”).
- **Kryteria akceptacji:** testy reguły deterministycznej na przykładach C# (xUnit, NUnit, FluentAssertions); testy części AI na atrapie: zgłoszenie osłabionej asercji pokazuje się z linią i dowodem, a zgłoszenie na linii wykrytej już przez regułę łączy się z nim zamiast dublować; bramka wymaga potwierdzenia, gdy strażnik coś zgłosił.
- **Rozmiar:** ~300 LOC.

#### P3-09 · Zapytaj o zaznaczenie

- **Zależy od:** P3-05
- **Zakres:** `?` na zaznaczeniu otwiera panel pytań w prawej kolumnie. Kontynuacja sesji briefingu przez `--resume`, odpowiedź strumieniowa, renderowana bezpiecznym Markdownem. Historia pytań przy celu. Pytanie może zmienić tylko stan panelu, nie drafty.
- **Kryteria akceptacji:** testy na atrapie: drugie pytanie używa `session_id` z briefingu; anulowanie zatrzymuje proces; odpowiedź z HTML renderuje się jako tekst.
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security (odpowiedź modelu jest niezaufana).

#### P3-10a · Redakcja komentarzy i notatka podsumowująca

- **Zależy od:** P3-07, P2-20
- **Zakres:** z roboczej notatki recenzenta powstaje komentarz w formacie Conventional Comments (podgląd, edycja, akceptacja). W bramce szkic notatki podsumowującej z komentarzy recenzenta, oznaczony „Szkic od AI na podstawie Twoich komentarzy”.
- **Kryteria akceptacji:** testy na atrapie: szkic nie zastępuje tekstu recenzenta bez akceptacji; odpowiedź z HTML i linkami `javascript:` renderuje się jako tekst; test komponentu: pole notatki w bramce ze szkicem, edycją i publikacją.
- **Rozmiar:** ~250 LOC.
- **Przegląd:** security (odpowiedź modelu trafia do GitLaba po akceptacji).

#### P3-10b · AI w interdiffie

- **Zależy od:** P3-07, P2-25
- **Zakres:** w interdiffie hipotezy tylko dla zmienionych hunków („Claude Code · tylko zmienione hunki”) i ocena „AI potwierdza poprawkę” dla wątków blokujących, jako informacja, nie rozwiązanie wątku.
- **Kryteria akceptacji:** test: do AI trafiają tylko zmienione hunki (sprawdzenie treści promptu); testy komponentów dla stanów z makiety `Interdiff`: „Brak nowych uwag w 2 zmienionych hunkach”, „Autor odpowiedział i zmienił kod. AI potwierdza poprawkę”; ocena AI nie zmienia stanu wątku.
- **Rozmiar:** ~250 LOC.
- **Przegląd:** security.

### 3.2 Roslyn

#### P3-11 · Szkielet sidecara .NET

- **Zależy od:** P1-03, S4
- **Zakres:**
  - `sidecar/Lupa.Sidecar` (.NET 10, `Nullable`), protokół żądanie–odpowiedź jako JSON w liniach po stdio, wersja kontraktu;
  - dołączenie do aplikacji jako sidecar Tauri ([dokumentacja Tauri 2: sidecar](https://v2.tauri.app/develop/sidecar/)), uruchamiany przez rdzeń z API Rusta, bez capability dla webview (zgodnie z ADR 0002);
  - sposób pakowania według spike'a S4;
  - start na żądanie, restart po awarii, limit czasu;
  - środowisko bez tokenu;
  - komendy .NET w `CLAUDE.md`, propozycja wzorców do `allow` w opisie zmiany (wpisuje człowiek); krok w CI.
- **Kryteria akceptacji:** `dotnet build sidecar`, `dotnet test sidecar`, `dotnet format --verify-no-changes sidecar` przechodzą; test Rust: awaria sidecara w trakcie żądania daje błąd i restart; test: niezgodna wersja kontraktu daje czytelny błąd.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security (uruchamianie procesu, brak capability dla webview).

#### P3-12 · Ładowanie projektów do Roslyn

- **Zależy od:** P3-11, P2-30 (zaakceptowany ADR), P2-31
- **Zakres:** ładowanie solution z worktree base i head według ADR z P2-30. Ładowanie przez `MSBuildWorkspace` wykonuje kod z repozytorium, więc instancja sidecara, która ładuje projekty, działa w strefie D według ADR z P2-30, a jej wyniki rdzeń traktuje jako niezaufane (walidacja ścieżek, limitów i schematu odpowiedzi).

  Wariant z ADR, jeśli zostanie wybrany: ewaluacja projektów gałęzi docelowej i same pliki `.cs` z MR. Wtedy MR zmieniający `.csproj`, `Directory.Build.*` lub `.targets` dostaje flagę „semantyka niepełna”.

  Fallback do Tree-sitter, gdy ładowanie się nie uda.
- **Kryteria akceptacji:** test integracyjny na projekcie testowym MAUI-podobnym (biblioteka + testy); test: projekt z targetem wykonującym komendę nie uruchamia jej poza strefą D (plik-pułapka); błąd ładowania daje fallback i informację w UI.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security.

#### P3-13 · Graf semantyczny dla trasy

- **Zależy od:** P3-12, P2-12
- **Zakres:** krawędzie z Roslyn (referencje symboli między zmienionymi jednostkami, implementacje interfejsów, wywołania) na base i head. Zastępują krawędzie heurystyczne z P2-09, a trasa korzysta z nich bez zmian w algorytmie. Różnice między grafami Tree-sitter i Roslyn są logowane do strojenia.
- **Kryteria akceptacji:** test: !482 daje tę samą trasę z oboma grafami; przypadek z przeciążeniami metod, który Tree-sitter myli, jest rozwiązany poprawnie przez Roslyn.
- **Rozmiar:** ~300 LOC.

#### P3-14 · Wywołania zmienionej metody i powiązane testy

- **Zależy od:** P3-12
- **Zakres:** w prawej kolumnie dla bieżącej jednostki: miejsca wywołań w całym repo (także poza MR), testy, które ją wywołują, linki do pliku i linii („Otwórz w Riderze”).
- **Kryteria akceptacji:** testy na projekcie testowym: wywołania przez interfejs i bezpośrednie; wynik dla metody bez wywołań; czas odpowiedzi < 500 ms po załadowaniu workspace.
- **Rozmiar:** ~250 LOC.

#### P3-15 · XAML ↔ ViewModel

- **Zależy od:** P3-12, P2-10
- **Zakres:** mapowanie `x:DataType` i wyrażeń `{Binding ...}` na właściwości ViewModelu. Wynik deterministyczny w panelu i bramce: binding do usuniętej lub przemianowanej właściwości, binding bez `x:DataType`.
- **Kryteria akceptacji:** testy: usunięta właściwość z bindingiem daje uwagę blokującą; binding do właściwości dziedziczonej działa; ścieżka zagnieżdżona (`Item.Name`) rozwiązana.
- **Rozmiar:** ~350 LOC.

#### P3-16 · Duplikaty

- **Zależy od:** P3-12
- **Zakres:** porównanie nowych metod z istniejącymi w repo (znormalizowane drzewo składni, odcisk tokenów). Wynik jako hipoteza w panelu: „podobne do `Helpers.SafeInvoke` (87%)”.
- **Kryteria akceptacji:** testy: kopia metody ze zmienionymi nazwami zmiennych wykryta; metody o podobnej strukturze, ale innej logice poniżej progu; czas < 2 s dla repo 5 000 plików z cache.
- **Rozmiar:** ~300 LOC.

### 3.3 Dowody z wykonania

Wszystkie zadania tego kamienia uruchamiają kod z MR, więc działają w strefie D przez `lupa-exec` (P2-32), w tle, z cache po SHA i są opcjonalne (włączane w ustawieniach repo).

#### P3-17 · Testy z MR na starym kodzie

- **Zależy od:** P2-32
- **Zakres:** wykrycie nowych i zmienionych testów w MR, uruchomienie ich na head i na base z kodem produkcyjnym z base (testy z head). Test, który przechodzi na base, nie sprawdza zmiany: dowód dla strażnika testów (P3-08) i sprawdzenie w kroku „Całość” i w bramce.
- **Kryteria akceptacji:** test integracyjny na projekcie testowym: nowy test sprawdzający zmianę failuje na base; test niezależny od zmiany przechodzi na base i zostaje oznaczony; brak kompilacji testu na base ma osobny stan „nie da się uruchomić na base”.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security.

#### P3-18 · Pokrycie zmienionych linii

- **Zależy od:** P3-17
- **Zakres:** pokrycie z coverlet, nałożone na diff jako znacznik na marginesie dla linii dodanych i zmienionych. Na VSTest to `--collect:"XPlat Code Coverage"` (cobertura), na Microsoft Testing Platform `coverlet.MTP` ([dokumentacja coverlet](https://github.com/coverlet-coverage/coverlet/blob/master/Documentation/VSTestIntegration.md)). Wariant zadanie wybiera według projektu testowego.
- **Kryteria akceptacji:** test na projekcie testowym: linia wykonana i niewykonana oznaczone poprawnie; raport cobertura parsowany jako niezaufany (limit rozmiaru, ścieżki spoza worktree ignorowane).
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security.

#### P3-19 · Mutacje w zmianie

- **Zależy od:** P3-17
- **Zakres:** Stryker.NET z `--since:<base>` ([konfiguracja Stryker.NET](https://stryker-mutator.io/docs/stryker-net/configuration)) tylko dla zmienionego kodu, uruchamiany ręcznie z UI („Uruchom mutacje”). Przeżyte mutanty jako hipotezy w panelu („brakująca asercja?”) z linią i mutacją.
- **Kryteria akceptacji:** test integracyjny na projekcie testowym: mutant w nowym kodzie bez asercji przeżywa i pojawia się w panelu; limit czasu przerywa przebieg z wynikiem częściowym.
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security.

#### P3-20 · Brakujące współzmiany

- **Zależy od:** P2-05
- **Zakres:** z historii 12 miesięcy pary plików zmieniane razem (wsparcie i pewność, progi w `config.yaml`). Gdy w celu zmienił się jeden z pary, wpis w briefingu i w kroku „Całość”: „`ScannerPage.xaml.cs` zwykle zmienia się ze `ScannerViewModel.cs`, tu nie”. Krawędzie słabe „co-change” w grafie.
- **Kryteria akceptacji:** testy na repo z zaplanowaną historią: para powyżej progów wykryta, poniżej nie; commity z ponad 50 plikami pominięte w statystyce.
- **Rozmiar:** ~250 LOC.

## Bramka wyjścia z fazy 3

Progi to propozycja planu. Zespół zatwierdza je przed startem fazy (pytanie w `open-questions.md`).

1. Zestaw ewaluacyjny z P3-22 (20 MR z kodu MAUI z wstawionymi błędami): precyzja hipotez ≥ 0,6 i recall ≥ 0,5 przy domyślnym progu pewności.
2. Przez 2 sprinty odsetek zaakceptowanych hipotez ≥ 40%, a najczęstszy powód odrzucenia ma plan poprawki w `.review/`.
3. Mediana szacowanego kosztu AI na MR mieści się w limicie ustalonym przez zespół.
4. Test przepływu potwierdza, że bez akceptacji człowieka nic z AI nie trafia do GitLaba.
5. ADR z P3-01 zaakceptowany, `security-reviewer` bez otwartych znalezisk wysokich i krytycznych.
6. Metryki z P1-30 policzone ponownie i porównane z bazą w raporcie fazy.
