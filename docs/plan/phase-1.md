# Faza 1: czytnik zmian

**Cel:** dowolną zmianę (lokalne zmiany, commit, zakres lub branch, merge request) otwierasz w Lupie i czytasz ją szybciej niż w przeglądarce GitLaba. Diff jest identyczny z Gitem i GitLabem.

**Poza zakresem:** zapis do GitLaba, trasa, reguły zespołu, AI. Kolejność plików w tej fazie to drzewo katalogów, a „krok” w nawigacji `n`/`p` oznacza plik.

**Wynik dla zespołu:** narzędzie do czytania MR i do przeglądu własnych zmian przed pushem.

## Spike'i na start fazy

Spike'i nie są fazą 0. To krótkie eksperymenty, które Claude wykonuje przed zadaniami zależnymi od ich wyniku. Zasady są w `spikes/README.md`, zakłada je `/spike start`.

| ID | Nazwa | Pytanie | Kryterium sukcesu | Limit | Odblokowuje |
| --- | --- | --- | --- | --- | --- |
| S1 | `codemirror-merge-external-diff` | Czy `@codemirror/merge` przyjmie hunki policzone w Ruście, czy budujemy widok z dwóch `EditorView` i własnych dekoracji? Ile fps daje plik 10 000 linii w WebView2? Jak mierzyć prawdziwą aplikację Tauri na Windows (start do pierwszego ekranu, fps w WebView2), skoro Playwright na buildzie Vite tego nie obejmuje? Narzędzia pomiaru są **NIEZWERYFIKOWANE** i ustala je spike. | widok obok siebie z hunkami z zewnątrz pokazuje te same linie co `git diff --histogram`; przewijanie ≥ 55 fps p95 na maszynie referencyjnej; powtarzalny pomiar startu aplikacji (rozrzut < 10% w 5 przebiegach) | 8 h | ADR widoku diffa, P1-18a, P1-28 |
| S2 | `gix-diff-parity` | Czy `gix` z `imara-diff` (histogram) daje hunki identyczne z `git diff --histogram` i jak szybko liczy największy MR z repo MAUI? Czy `gix` obsługuje status working tree z `core.autocrlf`, `.gitattributes` i filtrami (np. LFS) zgodnie z `git status`? | 100% zgodności na 200 losowych parach plików; lista plików największego MR z ostatnich 6 miesięcy < 150 ms z ciepłego cache; status working tree zgodny z `git status --porcelain` na 5 przypadkach z CRLF i LFS, a jeśli nie, decyzja o `git` CLI dla working tree | 8 h | ADR biblioteki Git, P1-06, P1-08 |
| S3 | `gitlab-instance-capabilities` | Jaka jest wersja instancji GitLaba zespołu i które z endpointów z `docs/input/plan.md` działają? Progi z kodu GitLaba (`fact-checker`, 2026-10-10): `draft_notes` z `position` od 16.3, `bulk_publish` od 15.11, a z notatką i `reviewer_state` od 19.2, `scope=reviews_for_me` od 18.4. | tabela endpoint → działa / nie działa / od wersji, z wynikiem wywołań na instancji | 3 h | P1-14b, zakres fazy 2 |

S3 wymaga tokenu do instancji zespołu, więc uruchamia go człowiek albo Claude na maszynie dewelopera z tokenem w zmiennej środowiskowej. Token nie trafia do plików spike'a.

## Kamienie milowe

| Kamień | Zadania | Po nim działa |
| --- | --- | --- |
| 1.0 Fundament | P1-01 … P1-05 | puste okno, CI, IPC, SQLite, lista repozytoriów |
| 1.1 Git i diff | P1-06 … P1-11 | diff dowolnej pary commitów i working tree w rdzeniu |
| 1.2 Źródła | P1-12 … P1-16 | cztery cele review otwierane z ekranu „Otwórz” |
| 1.3 Widok | P1-17a … P1-27 | pełny czytnik z klawiaturą, viewed, wątkami GitLaba |
| 1.4 Wydanie | P1-28 … P1-30 | budżety zmierzone, instalator, metryki bazowe |

## Zadania

Format każdego zadania: zależności, zakres, kryteria akceptacji (co uruchomić i jaki wynik), szacowany rozmiar zmiany, przeglądy. Każde zadanie kończy `/self-review`. Zadania oznaczone **security** przegląda też `security-reviewer`.

### 1.0 Fundament

#### P1-01 · Szkielet monorepo i aplikacji Tauri

- **Zależy od:** —
- **Zakres:** Cargo workspace z `app/src-tauri` i pustym `crates/lupa-testkit`. npm workspaces z `app` i `packages/domain`, Vite + React + TypeScript `strict`, ESLint, Prettier, Vitest. Okno Tauri 2 z ekranem „Lupa” i restrykcyjnym CSP. Komendy z `architecture.md` w `package.json` i w sekcji „Komendy” w `CLAUDE.md`. Propozycja wzorców do `allow` w opisie zmiany. Wpisuje je człowiek, agent nie zmienia własnych uprawnień.
- **Kryteria akceptacji:**
  - `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run build`, `npm test`, `npm run lint`, `npm run typecheck` kończą się kodem 0;
  - `npm run app:build` na Windows tworzy instalator, a aplikacja po instalacji pokazuje ekran „Lupa”;
  - jeden test smoke w każdym ekosystemie, który failuje po usunięciu testowanego elementu;
  - `node --test .claude/hooks/hooks.test.mjs` nadal przechodzi.
- **Rozmiar:** ~350 LOC, głównie konfiguracja.

#### P1-02 · CI na Windows

- **Zależy od:** P1-01
- **Zakres:** repo Lupy jest na GitHubie (`README.md`, „Mapa repo”), więc workflow GitHub Actions na hostowanym runnerze Windows (jeśli zespół przeniesie repo, zadanie zmienia platformę, patrz Q9 w `open-questions.md`): format, clippy, testy Rust, lint, typecheck, testy TS, testy hooków. Cache zależności. Bez sekretów.
- **Kryteria akceptacji:** workflow przechodzi na PR z tej zmiany; celowo zepsuty test (w osobnym commicie testowym, potem cofniętym) daje czerwony status.
- **Rozmiar:** ~120 LOC.

#### P1-03 · Kontrakt IPC

- **Zależy od:** P1-01
- **Zakres:** wzorzec komendy Tauri z `Result<T, IpcError>`, zdarzenia postępu, anulowanie zadania. Generowanie typów do `packages/ipc` (wybór między `tauri-specta` a `ts-rs` w opisie zmiany). Walidacja odpowiedzi w UI (`zod`). Skrypt `npm run check:ipc`, który failuje, gdy typy są nieaktualne, dodany do CI. Przykładowa komenda `app_info`.
- **Kryteria akceptacji:**
  - zmiana pola w Rust bez regeneracji typów daje błąd `npm run check:ipc`;
  - test Rust: `IpcError` z błędu zawierającego token (wzorzec `glpat-`) nie zawiera tokenu po serializacji;
  - test TS: odpowiedź niezgodna ze schematem daje kontrolowany błąd, nie wyjątek w renderze.
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security (granica webview ↔ rdzeń, capabilities Tauri).

#### P1-04 · Magazyn SQLite

- **Zależy od:** P1-01
- **Zakres:** `lupa-store`: plik bazy w katalogu danych aplikacji, migracje w kodzie z numerem wersji, tabele `repos`, `review_sessions`, `viewed`, `cache_entries`. Katalog cache (diffy, worktree, pliki tymczasowe) z limitem rozmiaru i sprzątaniem LRU.
- **Kryteria akceptacji:** `cargo test -p lupa-store`: migracja od zera, migracja idempotentna przy ponownym starcie, odrzucenie bazy z nowszej wersji z czytelnym błędem, sprzątanie LRU usuwa najstarsze wpisy po przekroczeniu limitu.
- **Rozmiar:** ~300 LOC.

#### P1-05 · Repozytoria w Lupie

- **Zależy od:** P1-03, P1-04
- **Zakres:** dodanie lokalnego repo (wybór folderu), wykrycie remote GitLaba (URL instancji, ścieżka projektu) z `git remote`, gałąź docelowa domyślna (`origin/HEAD` albo ustawienie). Normalizacja ścieżek Windows (backslash, wielkość liter). Ekran listy repozytoriów.
- **Kryteria akceptacji:**
  - test: `C:\Repo\App` i `c:/repo/app/` dają jeden wpis;
  - test: remote SSH i HTTPS tej samej instancji dają ten sam projekt;
  - ścieżka spoza repozytorium Git daje komunikat, nie wyjątek.
- **Rozmiar:** ~280 LOC.

### 1.1 Git i diff

#### P1-06 · Odczyt repozytorium

- **Zależy od:** P1-01, S2
- **Zakres:** `lupa-git`: rozwiązywanie refów i SHA, `merge-base`, lista zmienionych plików między dwoma drzewami z wykrywaniem zmian nazw, odczyt blobów. `lupa-testkit`: budowanie repozytoriów testowych przez `git` CLI w katalogu tymczasowym.
- **Kryteria akceptacji:** `cargo test -p lupa-git` obejmuje: dodanie, usunięcie, zmianę nazwy z edycją, plik binarny, zmianę trybu pliku, CRLF, submoduł (pokazany jako zmiana wskaźnika), nazwę pliku z polskimi znakami i spacją. Wyniki są identyczne z `git diff --name-status -M` na tych samych repozytoriach.
- **Rozmiar:** ~380 LOC.

#### P1-07 · Migawka working tree

- **Zależy od:** P1-06
- **Zakres:** zmiany staged, unstaged i nieśledzone względem wybranej bazy, z respektowaniem `.gitignore`. Identyfikator treści liczony jak `git hash-object` (klucz cache bez commita). Normalizacja końców linii zgodna z `core.autocrlf` i `.gitattributes`.
- **Kryteria akceptacji:** testy: identyfikator pliku równy wynikowi `git hash-object`; plik ignorowany nie pojawia się; plik z CRLF w working tree i LF w indeksie przy `autocrlf=true` nie jest zmieniony; usunięty, ale nie zestage'owany plik jest widoczny jako usunięty.
- **Rozmiar:** ~300 LOC.

#### P1-08 · Diff linii

- **Zależy od:** P1-06, S2
- **Zakres:** `lupa-diff`: histogram przez `imara-diff`, hunki z konfigurowalnym kontekstem, opcja ignorowania białych znaków na początku linii („Ignoruj wcięcia” z makiety `Main`).
- **Kryteria akceptacji:** test zgodności: na 200 parach plików z `lupa-testkit` (losowane z ustalonym ziarnem) hunki są identyczne z `git diff --histogram -U3`, a z ignorowaniem wcięć zgodne z `git diff --histogram --ignore-space-change` w zakresie opisanym w teście.
- **Rozmiar:** ~300 LOC.

#### P1-09 · Diff słów w hunku

- **Zależy od:** P1-08
- **Zakres:** podział linii na tokeny (identyfikatory, liczby, operatory, białe znaki), diff tokenów dla par linii zmienionych, limit długości linii i liczby tokenów (powyżej limitu brak diffu słów, cała linia zaznaczona).
- **Kryteria akceptacji:** testy: zmiana nazwy zmiennej zaznacza tylko nazwę; linia 20 000 znaków kończy się w < 5 ms bez diffu słów; zakresy znaków są poprawne dla znaków spoza ASCII (UTF-16 dla CodeMirror).
- **Rozmiar:** ~250 LOC.

#### P1-10 · Klasyfikacja plików i limity

- **Zależy od:** P1-08
- **Zakres:** pliki generowane (wzorce domyślne `*.Designer.cs`, `Resource.designer.cs`, lockfile'e, migracje oraz nagłówek `<auto-generated>`), binarne, zmiany samego formatowania, czysta zmiana nazwy. Limity: plik powyżej progu rozmiaru i linii jest pokazany jako „za duży, otwórz w Riderze”, bez liczenia diffu.
- **Kryteria akceptacji:** testy na każdą klasę; plik 50 MB nie jest wczytywany do pamięci w całości (test z licznikiem odczytanych bajtów); wzorce działają bez rozróżniania wielkości liter i z backslashami.
- **Rozmiar:** ~250 LOC.
- **Przegląd:** security (limity rozmiaru).

#### P1-11 · Cache diffów

- **Zależy od:** P1-04, P1-08, P1-09
- **Zakres:** wyniki diffu kluczowane (blob stary, blob nowy, opcje, wersja algorytmu) w katalogu cache z indeksem w SQLite.
- **Kryteria akceptacji:** test: drugie otwarcie tej samej pary nie wywołuje diffu (licznik wywołań = 1); zmiana opcji „Ignoruj wcięcia” liczy nowy wpis; podbicie wersji algorytmu unieważnia stare wpisy.
- **Rozmiar:** ~200 LOC.

### 1.2 Źródła zmian

#### P1-12 · Cel review: commit, zakres, branch

- **Zależy od:** P1-06, ADR 0003
- **Zakres:** `lupa-review`: `ReviewTarget` dla `Commit` (względem rodzica; przy merge commicie wybór rodzica, domyślnie pierwszy), `Range` (`A..B`) i branch (`merge-base(target, source)` → `source`). Identyfikator celu stabilny między uruchomieniami.
- **Kryteria akceptacji:** testy na repozytoriach z `lupa-testkit`: lista plików i hunki zgodne z `git diff A B`, `git show <sha>` (zwykły commit) i `git diff target...source`; merge commit z wyborem drugiego rodzica zgodny z `git diff <sha>^2 <sha>`; pierwszy commit w repo (bez rodzica) pokazuje wszystkie pliki jako dodane.
- **Rozmiar:** ~300 LOC.

#### P1-13 · Cel review: lokalne zmiany

- **Zależy od:** P1-07, P1-12
- **Zakres:** `WorkingTree` w dwóch trybach: względem `HEAD` (tylko niezacommitowane) i względem `merge-base` z gałęzią docelową (cała praca na gałęzi plus niezacommitowane, domyślny tryb self-review). Odświeżanie przyciskiem i po powrocie fokusu do okna. Zachowanie „viewed” dla plików bez zmian treści.
- **Kryteria akceptacji:** testy obu trybów; po edycji pliku i odświeżeniu „viewed” znika tylko z tego pliku; odświeżenie repo z 5 000 plików bez zmian trwa < 300 ms (pomiar w teście integracyjnym na repo z `lupa-testkit`).
- **Rozmiar:** ~250 LOC.

#### P1-14a · Token GitLaba

- **Zależy od:** P1-03
- **Zakres:** `lupa-secrets`: token w Windows Credential Manager, ekran ustawień do wklejenia, sprawdzenia i usunięcia tokenu, typ `Secret` bez `Debug` i `Display` z treścią. Zakres tokenu według Q8 z `open-questions.md`.
- **Kryteria akceptacji:**
  - test: token nie występuje w `Debug`, `Display`, logach ani w `IpcError` (przeszukanie wyjścia testu);
  - token nigdy nie trafia do argumentów procesu ani do pliku (test na atrapie magazynu);
  - usunięcie tokenu w ustawieniach usuwa wpis z magazynu.
- **Rozmiar:** ~200 LOC.
- **Przegląd:** security.

#### P1-14b · Klient GitLaba (odczyt)

- **Zależy od:** P1-14a, S3
- **Zakres:** `lupa-gitlab`:
  - konfigurowalny URL instancji, TLS z weryfikacją (opcja dodania firmowego CA, bez wyłączania weryfikacji);
  - wersja instancji z `GET /api/v4/version` ([Metadata API](https://docs.gitlab.com/api/metadata/)) i mapa możliwości z wyniku S3;
  - `GET` MR, `diff_refs`, wersje MR, dyskusje, stronicowanie, limity zapytań i ponowienia;
  - redakcja tokenu w błędach.
- **Kryteria akceptacji:** testy na atrapie HTTP: stronicowanie, 401 z czytelnym komunikatem bez tokenu, 429 z ponowieniem, wyłączenie funkcji przy starszej wersji instancji, odrzucenie certyfikatu spoza zaufanych CA.
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security.

#### P1-15 · Cel review: merge request

- **Zależy od:** P1-12, P1-14b
- **Zakres:** pobranie `refs/merge-requests/<iid>/head` ([GitLab: MR troubleshooting](https://docs.gitlab.com/user/project/merge_requests/merge_request_troubleshooting/)) przez `git fetch` w tle (uwierzytelnienie z istniejącej konfiguracji Gita dewelopera, bez tokenu Lupy w argumentach i środowisku). Cel MR z `diff_refs` (`base_sha`, `start_sha`, `head_sha`). `diff_refs` wypełnia się asynchronicznie po utworzeniu MR, więc pusty wynik oznacza „liczy się”, a nie błąd. GitLab usuwa ref `head` 14 dni po zamknięciu lub merge'u MR. Wtedy Lupa próbuje pobrać commity po SHA z `diff_refs`, a gdy się nie da, pokazuje komunikat. Test zgodności z listą zmian MR z API.
- **Kryteria akceptacji:**
  - test na atrapie: cel MR używa SHA z `diff_refs`, nie z bieżącej gałęzi docelowej;
  - test: `iid` i nazwy gałęzi z API są walidowane, zanim trafią do argumentów `git` (odrzucenie wartości zaczynających się od `-` i zawierających `..`);
  - test na atrapie: pusty `diff_refs` daje stan „liczy się” i ponowienie;
  - ręczna weryfikacja na 20 ostatnich MR z repo MAUI (otwartych albo zmergowanych w ciągu 14 dni): lista plików i liczby `+`/`−` zgodne z GitLabem, wynik w opisie zmiany.
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security (argumenty procesu z danych MR).

#### P1-16 · Ekran „Otwórz”

- **Zależy od:** P1-12, P1-13, P1-15
- **Zakres:** nowy ekran (nie ma go w makietach, układ niżej). Paleta `Ctrl+K` i ekran startowy mają te same cztery wejścia:
  - **Lokalne zmiany:** wybór trybu (`HEAD` lub `merge-base`), liczba plików i LOC;
  - **Commit:** wyszukiwarka po SHA, tytule, autorze (ostatnie 500 commitów, filtrowanie na bieżąco);
  - **Branch lub zakres:** dwa pola z podpowiedziami refów;
  - **Merge request:** wklejony URL albo `!iid`.

  Pod spodem lista ostatnio otwieranych celów.
- **Kryteria akceptacji:** testy komponentów dla czterech wejść i błędów (nieznany SHA, MR z innego projektu, brak tokenu przy MR); test Playwright: od startu aplikacji do otwartego diffu commita wyłącznie z klawiatury.
- **Rozmiar:** ~350 LOC.

Układ ekranu „Otwórz”: po lewej cztery zakładki źródeł (`1`–`4`), w środku pole wyszukiwania z wynikami nawigowanymi `↑`/`↓`, po prawej podgląd wybranego celu (base → head, pliki, LOC, autor). `Enter` otwiera cel. Wygląd zgodny z tokenami makiety `Inbox`.

### 1.3 Widok

#### P1-17a · Powłoka UI

- **Zależy od:** P1-01
- **Zakres:** układ trzech kolumn z regulowaną szerokością i chowanym prawym panelem (jeden klawisz). Pasek górny: repo, cel, licznik sesji, LOC. Pasek stanu ze skrótami. Routing ekranów.
- **Kryteria akceptacji:** testy komponentów: zmiana szerokości kolumny zapamiętana między uruchomieniami; chowanie panelu klawiszem nie zmienia fokusu w diffie.
- **Rozmiar:** ~250 LOC.

#### P1-17b · Motywy, palety i gęstość

- **Zależy od:** P1-17a
- **Zakres:** tokeny kolorów z makiet: motyw jasny i ciemny, paleta diffa standard i daltonizm, gęstość komfort i kompakt. Dołączony font JetBrains Mono, regulowany rozmiar fontu.
- **Kryteria akceptacji:** zrzuty Playwright 2 motywy × 2 palety × 2 gęstości porównane z wzorcem; test kontrastu: tekst i znaki `+`/`−` ≥ 4,5:1 w każdym wariancie (skrypt liczący kontrast z tokenów).
- **Rozmiar:** ~250 LOC.

#### P1-18a · Diff obok siebie

- **Zależy od:** S1, P1-11, P1-17a
- **Zakres:** widok pliku obok siebie według wyniku S1 (`MergeView` z [`@codemirror/merge`](https://github.com/codemirror/merge) albo dwa `EditorView`), z hunkami i diffem słów z rdzenia (nie z algorytmu w przeglądarce), synchronizacją przewijania obu stron i znacznikami „base”/„head” z SHA nad kolumnami.
- **Kryteria akceptacji:** test komponentu: linie dodane, usunięte i zmienione mają klasy zgodne z hunkami z rdzenia; ten sam plik z hunkami w obu kolumnach ma wyrównane linie; przewijanie jednej strony przewija drugą do odpowiadającej linii.
- **Rozmiar:** ~300 LOC.

#### P1-18b · Podświetlanie składni i pomiar widoku

- **Zależy od:** P1-18a
- **Zakres:** podświetlanie C#, XAML/XML, JSON i TypeScript (pakiety języków CodeMirror wybrane w zadaniu, z linkami), liczone tylko dla viewportu. Scenariusz w `bench/` dla pliku 10 000 linii.
- **Kryteria akceptacji:** testy komponentów: tokeny słów kluczowych C# i atrybutów XAML mają klasy składni; pomiar: plik 10 000 linii przewija się ≥ 55 fps p95 (`bench/`, metoda z S1).
- **Rozmiar:** ~200 LOC.

#### P1-19 · Jeden widok i przełączniki

- **Zależy od:** P1-18a
- **Zakres:** widok zunifikowany (przy `unifiedMergeView` z `mergeControls: false`, bo domyślnie pokazuje przyciski akceptacji i odrzucenia zmian), przełącznik „Obok siebie / Jeden widok”, automatyczne przejście na jeden widok poniżej progu szerokości (z adnotacją „Włącza się sam w wąskim oknie” jak w makiecie `Main`), przełącznik „Ignoruj wcięcia”.
- **Kryteria akceptacji:** testy: zmiana szerokości okna poniżej progu przełącza widok i wraca po poszerzeniu, o ile użytkownik nie wybrał trybu ręcznie; pozycja przewijania i bieżący hunk zostają po przełączeniu.
- **Rozmiar:** ~250 LOC.

#### P1-20 · Zwijanie kontekstu i nagłówki hunków

- **Zależy od:** P1-18a, P1-22
- **Zakres:** zwinięty niezmieniony kontekst z rozwijaniem (całość lub po 20 linii), nagłówek hunka `@@ old → new @@` z nazwą symbolu (np. `FrameAnalyzer › Analyze(ImageProxy, CancellationToken)`).
- **Kryteria akceptacji:** testy komponentów: rozwinięcie nie przesuwa bieżącego hunka; nagłówek pokazuje symbol z `lupa-syntax`, a przy jego braku sam zakres linii.
- **Rozmiar:** ~200 LOC.

#### P1-21 · Lista plików

- **Zależy od:** P1-17a, P1-10
- **Zakres:** lewa kolumna w trybie „Wszystkie pliki”: drzewo katalogów, `+`/`−` per plik, stan „viewed”, grupa „Zwinięte automatycznie” dla plików generowanych i formatowania. Wirtualizacja listy. W środku zawsze jeden plik, następny wczytywany w tle.
- **Kryteria akceptacji:** test: lista 5 000 plików renderuje < 50 elementów DOM i przewija się bez spadków poniżej 55 fps (`bench/`); przejście do następnego pliku z cache < 50 ms.
- **Rozmiar:** ~300 LOC.

#### P1-22 · Symbole z Tree-sitter

- **Zależy od:** P1-08
- **Zakres:** `lupa-syntax`: parsowanie C# ([`tree-sitter-c-sharp`](https://crates.io/crates/tree-sitter-c-sharp)) i XAML. Dla XAML kandydatem jest [`tree-sitter-xml`](https://crates.io/crates/tree-sitter-xml), ale jego przydatność do XAML jest **NIEZWERYFIKOWANA**, a crate nie był aktualizowany od 2024. Zadanie zaczyna od testu na 20 plikach XAML z repo MAUI i zgodności z wersją `tree-sitter`. Jeśli test nie przejdzie, XAML parsuje zwykły parser XML w Ruście. Dalej: mapowanie hunka na najgłębszy symbol (namespace › klasa › metoda/właściwość/konstruktor; element XAML z `x:Name` lub typem). Cache po blobie.
- **Kryteria akceptacji:** testy na plikach C# i XAML z `lupa-testkit`: hunk w metodzie, właściwości z akcesorami, konstruktorze, lokalnej funkcji, klasie zagnieżdżonej, rekordzie; plik z błędem składni daje częściowy wynik, nie błąd; parsowanie pliku 10 000 linii < 30 ms.
- **Rozmiar:** ~350 LOC.

#### P1-23 · Klawiatura i paleta poleceń

- **Zależy od:** P1-19, P1-21
- **Zakres:** `j`/`k` (hunk), `n`/`p` (plik, od fazy 2 krok trasy), `v` (viewed), `o` (Rider), `Ctrl+K` (paleta poleceń z wyszukiwaniem akcji i plików), `?` zarezerwowany dla fazy 3, pomoc skrótów. Skróty nie działają w polach tekstowych.
- **Kryteria akceptacji:** test Playwright: każdy skrót z tabeli `docs/input/plan.md` dostępny w tej fazie wykonuje akcję; pomiar: `j` do wyrenderowanego hunka < 16 ms p95 (`bench/`).
- **Rozmiar:** ~300 LOC.

#### P1-24 · „Viewed”, licznik sesji i higiena uwagi

- **Zależy od:** P1-04, P1-21
- **Zakres:** stan „viewed” per plik i para blobów (zmiana treści zdejmuje znacznik). Licznik przeczytanych LOC i czasu aktywnej sesji (bez czasu bezczynności). Przy ok. 400 LOC lub 60 minutach delikatna sugestia przerwy w pasku, bez modala. Zapis miejsca i wznowienie po ponownym otwarciu celu.
- **Kryteria akceptacji:** testy domeny z kontrolowanym zegarem: próg czasu, próg LOC, bezczynność 5 min nie liczy się do sesji; test: zamknięcie i ponowne otwarcie aplikacji wraca do tego samego pliku i hunka.
- **Rozmiar:** ~300 LOC.

#### P1-25 · Otwórz w Riderze

- **Zależy od:** P1-23
- **Zakres:** ścieżka do `rider64.exe` w ustawieniach (szukana w katalogu `bin` instalacji i w skryptach Toolbox, bo domyślnie nie ma jej w `PATH`), uruchomienie bez powłoki z `--line <n> <plik>` ([Rider: otwieranie plików z wiersza poleceń](https://www.jetbrains.com/help/rider/Opening_Files_from_Command_Line.html)).
  - Gdy plik w working copy ma tę samą treść co strona „head”, otwieramy plik z working copy.
  - W przeciwnym razie zapisujemy blob „head” jako plik tylko do odczytu w katalogu cache i otwieramy sam plik, bez solution. Otwarcie projektu z MR w Riderze uruchomiłoby ewaluację MSBuild kodu z MR.
- **Kryteria akceptacji:** test: argumenty procesu to lista, a ścieżka z cache jest wewnątrz katalogu cache także dla nazw pliku z `..` i znakami spoza ASCII; test: brak launchera daje komunikat z linkiem do ustawień.
- **Rozmiar:** ~200 LOC.
- **Przegląd:** security (path traversal, kod z MR).

#### P1-26 · Wątki GitLaba w diffie (odczyt)

- **Zależy od:** P1-15, P1-18a
- **Zakres:** dyskusje MR przypięte do linii po `position`, w osobnym paśmie pod linią, nie na kodzie. Wątki z poprzednich wersji MR oznaczone jako nieaktualne, z linkiem do wersji. Znacznik „wątek” w gutterze.
- **Kryteria akceptacji:** testy mapowania pozycji (linia dodana, usunięta, kontekst, zakres wielu linii, plik ze zmienioną nazwą, wątek z wcześniejszej wersji); test komponentu: treść komentarza z `<script>` i `<img onerror>` renderuje się jako tekst.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security.

#### P1-27 · Bezpieczne renderowanie treści MR

- **Zależy od:** P1-17a
- **Zakres:** jeden komponent dla Markdownu z MR (opis, komentarze) bez surowego HTML, z linkami otwieranymi w przeglądarce systemowej po potwierdzeniu, bez ładowania zdalnych obrazów. Nazwy plików, gałęzi i autorów zawsze jako tekst. Reguła ESLint zakazująca `dangerouslySetInnerHTML` w `app/src`.
- **Kryteria akceptacji:** testy z zestawem wrogich danych w `lupa-testkit` (XSS, `javascript:` w linku, bardzo długi tytuł, znaki sterujące RTL w nazwie pliku): żaden nie wykonuje kodu, a znaki RTL są widoczne jako znaczniki; `npm run lint` failuje na pliku z `dangerouslySetInnerHTML`.
- **Rozmiar:** ~250 LOC.
- **Przegląd:** security.

### 1.4 Wydanie

#### P1-28 · Pomiary budżetów wydajności

- **Zależy od:** P1-18a, P1-21, P1-23
- **Zakres:** `bench/`: generator repozytorium referencyjnego (5 000 plików, MR z 60 plikami i 2 000 LOC, plik 10 000 linii), scenariusze dla każdego budżetu z `architecture.md`, raport JSON i Markdown, komenda `npm run bench`. W CI jako raport, bez blokowania. Na maszynie referencyjnej zespołu jako bramka fazy.
- **Kryteria akceptacji:** `npm run bench` daje raport z p50 i p95 dla pięciu budżetów; raport z maszyny referencyjnej dołączony do opisu zmiany.
- **Rozmiar:** ~350 LOC.

#### P1-29 · Instalator i logi

- **Zależy od:** P1-01
- **Zakres:** instalator Windows z bundlera Tauri, wersja w oknie „O programie”. Logi lokalne z rotacją, poziomem w ustawieniach i redakcją tokenów oraz treści MR (do logu trafiają identyfikatory, nie treść). Instrukcja instalacji w `README.md`.
- **Kryteria akceptacji:** instalacja i odinstalowanie na czystym Windows na koncie bez uprawnień administratora (instalacja per użytkownik; opcję bundlera zadanie sprawdza w [dokumentacji Tauri 2](https://v2.tauri.app/distribute/windows-installer/), status linku **NIEZWERYFIKOWANY**); test: logi po scenariuszu z tokenem i treścią MR nie zawierają żadnego z nich.
- **Rozmiar:** ~200 LOC.
- **Przegląd:** security (logi).

#### P1-30 · Metryki bazowe zespołu

- **Zależy od:** P1-14b
- **Zakres:** komenda w ustawieniach „Policz metryki bazowe” dla projektu, z ostatnich 3 miesięcy, z API GitLaba: mediana czasu do pierwszego review, liczba rund na MR, mediana rozmiaru MR i liczba błędów po merge (issues z etykietą ustawioną w konfiguracji, powiązane z MR; jeśli zespół nie prowadzi błędów w GitLabie, metryka zostaje pusta z adnotacją, patrz Q10). Wynik zapisany lokalnie i eksportowany do Markdown. Zastępuje pomiar bazy z fazy 0 z `docs/input/plan.md`.
- **Kryteria akceptacji:** testy na atrapie API z ustalonym zestawem MR dają znane wartości median; eksport nie zawiera tytułów ani treści MR, tylko liczby i daty.
- **Rozmiar:** ~250 LOC.

## Bramka wyjścia z fazy 1

Wszystkie warunki muszą być spełnione, a dowody są w opisie zmiany, która zamyka fazę:

1. Cztery źródła otwierają się na repo MAUI zespołu (zrzut ekranu i lista SHA).
2. Zgodność diffu: 20 ostatnich zmergowanych MR ma listę plików i liczby `+`/`−` zgodne z GitLabem. Każda rozbieżność jest wyjaśniona.
3. Budżety z `docs/input/plan.md` są spełnione na maszynie referencyjnej (raport `npm run bench`, p95).
4. Co najmniej 3 osoby z zespołu czytają MR w Lupie przez tydzień. W krótkiej ankiecie co najmniej 2 z 3 wolą Lupę od przeglądarki do czytania zmian.
5. Metryki bazowe z P1-30 są zapisane.
6. `security-reviewer` przejrzał zadania oznaczone security i nie ma otwartych znalezisk o wadze wysokiej lub krytycznej.
