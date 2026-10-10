# Spike S2: `gix-diff-parity`

- Status: zamknięty
- Start i koniec: 2026-10-10
- Limit: 8 h, faktycznie ok. 1,5 h

## Pytanie

Czy `gix` (odczyt repo) z `imara-diff` (histogram) daje te same zmiany co `git diff --histogram` i jest dość szybki na dużym repo .NET MAUI? Jak liczyć status working tree, żeby zgadzał się z `git status` (`core.autocrlf`, `.gitattributes`, filtry)? Odblokowuje P1-06, P1-07 i P1-08.

## Hipoteza

`imara-diff` z heurystyką wcięć daje hunki identyczne z Gitem, a `gix` liczy listę zmienionych plików największej zmiany poniżej 150 ms.

## Metoda

- Program w `src/main.rs` (kod jednorazowy, nie trafia do aplikacji). Uruchamianie z katalogu spike'a: `cargo build --release`.
- `parity <repo> <n>`: n par blobów (stary, nowy) z prawdziwych commitów historii (pliki `.cs`, `.xaml`, `.csproj`, `.json`, `.md`, `.yml`, `.props`). Dla każdej pary porównuje zakresy zmienionych linii z `imara-diff` 0.2.0 (`Algorithm::Histogram` + `postprocess_lines`) z `git diff --histogram -U0`. Dodatkowo porównuje tekst unified diff (`-U3`).
- `perf <repo> <from> <to>`: `gix` 0.89 otwiera repo, liczy diff drzew i diff wszystkich zmienionych plików tekstowych. Dla porównania mierzy `git diff --numstat`.
- Repozytoria: `CommunityToolkit/Maui` (1192 pliki, 600 commitów, `--depth 600`) i `dotnet/maui` (26 412 plików, `--depth 120`).
- Środowisko: kontener Linux, Rust 1.97, git 2.43, build release.

## Kryterium sukcesu

- 100% zgodności zakresów zmian na 200 parach;
- lista plików największej zmiany < 150 ms;
- status working tree zgodny z `git status` przy CRLF.

## Wynik

**Werdykt: hipoteza potwierdzona, z jednym zastrzeżeniem.**

### Zgodność z Gitem

| Repo | Pary | Zakresy zgodne | Różnice zakresów | Różnice tekstu unified (`-U3`) |
| --- | --- | --- | --- | --- |
| CommunityToolkit.Maui | 200 | 200 | 0 | 18 |
| CommunityToolkit.Maui | 1000 | 997 | 3 | 64 |
| dotnet/maui | 300 | 300 | 0 | 8 |

- **Różnice tekstu unified** wynikają z błędu drukarki `BasicLineDiffPrinter` w `imara-diff`. Przy hunku blisko początku pliku dokłada jedną linię kontekstu ponad 3 (przykład: `ParentWindowTests.cs`, imara `@@ -2,7 +2,8 @@` z 4 liniami kontekstu, git `@@ -2,6 +2,7 @@`). Zmienione linie są te same. Lupa nie używa tej drukarki, bo kontekst i zwijanie liczy sama (P1-08), więc to nas nie dotyczy.
- **3 różnice zakresów na 1000** to przesunięcia „ślizgacza”. Ten sam blok dodanych lub usuniętych linii jest pokazany o 1–3 linie wyżej lub niżej, a sumy `+`/`−` są identyczne (np. `PopupServiceTests.cs`: imara usuwa linie 604–605, git 603–604). Częstość to 0,3%.

### Wydajność (`gix` + `imara-diff`, build release)

| Repo, zakres | Zmienione pliki | Diff drzew | Diff wszystkich plików tekstowych | `git diff --numstat` |
| --- | --- | --- | --- | --- |
| Toolkit, 100 commitów | 450 | 10 ms | 33 ms (268 plików) | 51 ms |
| Toolkit, 599 commitów | 1241 | 16 ms | 34 ms (611 plików) | 118 ms |
| dotnet/maui, 20 commitów | 59 | 16 ms | 11 ms (21 plików) | 20 ms |
| dotnet/maui, 60 commitów | 675 | 96 ms | 80 ms (378 plików) | 114 ms |
| dotnet/maui, 119 commitów | 3105 | 31 ms | 222 ms (1014 plików) | 1533 ms |

Otwarcie repo trwa < 1 ms. Lista plików nawet dla 3105 zmian mieści się w 150 ms. Diff liczymy leniwie per plik, więc koszt pełnego diffu nie blokuje otwarcia.

### Working tree

- `git status --porcelain=v2 -z` na dotnet/maui (26 412 plików): 0,10–0,15 s na Linuksie.
- Plik `.cs` z dopisanym `\r\n` przy `* text=auto` i `*.cs text`: git zgłasza zmianę zgodnie z atrybutami, także z `core.autocrlf=true`.
- Obsługi filtrów (`.gitattributes`, LFS, `core.autocrlf`) w `gix` nie implementowałem. CLI Gita robi to dokładnie tak jak Git, bo to Git.

## Rekomendacja (do zapisania w planie)

1. **Diff:** `imara-diff` 0.2 bezpośrednio (`Algorithm::Histogram` + `postprocess_lines`), bez jego drukarki unified. Hunki i kontekst liczy `lupa-diff`.
2. **Odczyt repo:** `gix` (odczyt obiektów, drzew, diff drzew). `git2` niepotrzebny.
3. **Working tree:** status z `git status --porcelain=v2 -z`, a identyfikatory treści z `git hash-object --stdin-paths` (bez `-w`, czyli bez zapisu do repo dewelopera). Daje to zgodność z filtrami i `core.autocrlf` bez własnej implementacji. Na Windows sprawdzić czas na repo zespołu w P1-13 (budżet 300 ms).
4. **Ryzyko dla komentarzy w MR (P2-15):** w ok. 0,3% plików nasz diff może pokazać blok przesunięty o linię względem GitLaba. Komentarz na linii, którą my pokazujemy jako zmienioną, a GitLab jako kontekst, dostałby wtedy inny typ pozycji. P2-15 sprawdza pozycję draftu z diffem wersji MR z API (pojedyncza wersja MR z `GET .../versions/:id`; to, że zwraca diffy, potwierdza zadanie w dokumentacji) przed wysłaniem i w razie rozbieżności używa pozycji GitLaba.
