# Faza 2: całe review w Lupie

**Cel:** recenzent przechodzi w Lupie cały cykl: skrzynka, briefing, trasa, bramka, publikacja w GitLabie, a po nowym pushu interdiff. Trasa i ryzyko są deterministyczne (Tree-sitter, reguły zespołu, historia Gita), bez AI. Autor robi self-review lokalnych zmian tą samą trasą przed pushem.

**Poza zakresem:** AI (miejsca na treści AI w makietach pokazują wyniki deterministyczne albo są ukryte), Roslyn, testy na base, pokrycie, mutacje.

**Zależność od wersji GitLaba (D1):** jeśli instancja wykryta przez P1-14b nie obsługuje `reviewer_state` w `bulk_publish` albo wersji MR, odpowiednie zadania mają wariant zastępczy opisany w zadaniu, a bramka fazy odnotowuje ograniczenie.

## Kamienie milowe

| Kamień | Zadania | Po nim działa |
| --- | --- | --- |
| 2.0 Skrzynka | P2-01 … P2-03, P2-35 | lista MR do review z rozmiarem, pipeline'em, ryzykiem i czasem |
| 2.1 Reguły zespołu | P2-04 … P2-07 | `.review/`, mapa ryzyka, hotspoty, checklisty przy hunkach |
| 2.2 Trasa | P2-08 … P2-14 | jednostki, graf, trasa, lewa kolumna, briefing |
| 2.3 Komentarze i publikacja | P2-15 … P2-22b | drafty na liniach, wątki, bramka, publikacja, offline |
| 2.4 Interdiff | P2-23 … P2-25, P2-36 | „co się zmieniło od mojego review”, także po rebase |
| 2.5 Tryby i kontekst | P2-26 … P2-29 | self-review, blame, spójność wzorca, przeniesiony kod |
| 2.6 Wykonanie kodu MR | P2-30 … P2-33 | różnica ostrzeżeń base ↔ head |
| 2.7 Dystrybucja | P2-34, P2-37 | auto-update, konfiguracja organizacji |

## Zadania

### 2.0 Skrzynka

#### P2-01 · Skrzynka: dane i odświeżanie

- **Zależy od:** P1-14b, P1-15
- **Zakres:**
  - zakładki „Do mojego review”, „Moje MR”, „Obserwowane” (MR przypięte lokalnie);
  - odświeżanie przez polling z `updated_after` (interwał w ustawieniach, wstrzymany przy braku sieci);
  - stan synchronizacji w pasku („zsynchronizowany 12 s temu”);
  - dane w SQLite, więc skrzynka startuje z cache bez sieci.

  Filtr „Do mojego review”: `scope=reviews_for_me` na GitLabie 18.4 i nowszym, a na starszych `reviewer_username=<ja>` ([Merge requests API](https://docs.gitlab.com/api/merge_requests/)). Wybór według wersji z P1-14b.
- **Kryteria akceptacji:** testy na atrapie API: wybór filtru według wersji (18.3 i 18.4), pierwsze pobranie, przyrostowe z `updated_after`, MR zamknięty znika, brak sieci pokazuje dane z cache i stan „offline”; start do skrzynki z cache < 1 s (`bench/`).
- **Rozmiar:** ~350 LOC.

#### P2-02 · Skrzynka: ekran

- **Zależy od:** P2-01, P1-17a
- **Zakres:** ekran według makiety `Inbox`:
  - kolumny: MR, rozmiar (LOC, pliki, pasek), ryzyko z powodem (z P2-05, do tego czasu „—”), pipeline, czas oczekiwania;
  - sortowanie „najdłużej czeka” domyślnie; filtry „Wysokie ryzyko”, „Nowa wersja od mojego review”, „Czeka ponad dzień”;
  - wyszukiwanie po MR, gałęzi i autorze;
  - panel podglądu z rozmiarem, szacowanym czasem i informacją „od Twojego ostatniego review”;
  - `↑`/`↓` wybór, `Enter` briefing, `t` „Od razu do trasy”;
  - plakietka „Duży: N przejść” i szacowany czas z P2-35.
- **Kryteria akceptacji:** testy komponentów dla stanów z makiety (pipeline czerwony z ostrzeżeniem, w toku, zielony; pierwsze review; nowa wersja); test Playwright: wybór MR i wejście do briefingu wyłącznie z klawiatury.
- **Rozmiar:** ~380 LOC.

#### P2-03 · Prefetch w tle i pipeline

- **Zależy od:** P2-01
- **Zakres:** kolejka zadań w tle w `lupa-review`: limit równoległości, priorytet dla MR zaznaczonego w skrzynce, anulowanie, ponowienia. Fetch refów MR i policzenie list plików przed kliknięciem. Status pipeline'u. Znacznik „nowa wersja od Twojego review” powstaje w P2-23, ryzyko i szacowany czas w P2-35.
- **Kryteria akceptacji:** testy: 10 MR w kolejce przy limicie 2 nie uruchamia więcej niż 2 fetchy naraz; zmiana zaznaczenia przesuwa MR na początek kolejki; anulowanie zatrzymuje proces `git`; status pipeline'u odświeża się z pollingiem skrzynki.
- **Rozmiar:** ~300 LOC.

### 2.1 Reguły zespołu

#### P2-04 · `.review/` z gałęzi docelowej

- **Zależy od:** P1-06
- **Zakres:** `lupa-rules`:
  - wczytanie `.review/config.yaml`, `checklists/*.yaml` i `rules/risk.yaml` z obiektów Gita gałęzi docelowej (nigdy z gałęzi MR ani z working tree);
  - schemat z wersją, walidacja z komunikatami o linii i polu, wartości domyślne przy braku katalogu;
  - limity rozmiaru plików i liczby reguł; regexy kompilowane z limitem rozmiaru (ochrona przed ReDoS);
  - wybór parsera YAML z uzasadnieniem w opisie zmiany.

  Dla celów lokalnych i commitów gałęzią docelową jest gałąź ustawiona dla repo.
- **Kryteria akceptacji:**
  - test: MR zmieniający `.review/rules/risk.yaml` nie zmienia ryzyka liczonego dla tego MR;
  - test: niepoprawny plik daje błąd z numerem linii i Lupa działa dalej na domyślnych;
  - test: regex o wykładniczym czasie odrzucony albo ograniczony czasem.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security (konfiguracja z niezaufanej gałęzi).

#### P2-05 · Mapa ryzyka i hotspoty

- **Zależy od:** P2-04
- **Zakres:**
  - ryzyko pliku z `rules/risk.yaml` (ścieżki → poziom i wymagane checklisty);
  - hotspoty z historii 12 miesięcy: liczba zmian i commitów z „fix” w temacie, liczone w tle z cache po `HEAD` gałęzi docelowej;
  - ryzyko MR jako maksimum z powodem („Platforms/Android, zmienione testy”).
- **Kryteria akceptacji:** testy na repo z `lupa-testkit` z zaplanowaną historią: znane liczby zmian i fixów; powód ryzyka wymienia regułę, która zadziałała; liczenie historii repo z 10 000 commitów < 2 s (pomiar w teście integracyjnym).
- **Rozmiar:** ~300 LOC.

#### P2-06 · Dopasowanie checklist do hunków

- **Zależy od:** P2-04, P1-22
- **Zakres:** dopasowanie reguł z `checklists/*.yaml` do zmienionych linii: `regex` z `scope: changed-lines`, `path` (glob bez rozróżniania wielkości liter, z backslashami) i zapytanie Tree-sitter. Wynik: pozycja checklisty z `ask`, `severity` i liniami. `require_checklist` z mapy ryzyka.
- **Kryteria akceptacji:** testy na przykładach z `docs/input/plan.md` (`event-unsubscribe` na `+=` w C#, `android-permissions` na `Platforms/Android/**`); reguła nie dopasowuje linii kontekstu ani usuniętych, chyba że to ustawiono.
- **Rozmiar:** ~300 LOC.

#### P2-07 · Checklisty w panelu kontekstu

- **Zależy od:** P2-06, P1-17a
- **Zakres:** prawa kolumna: pozycje checklisty dopasowane do bieżącego hunka, „potwierdź” i „pomiń z powodem”, znacznik „reguła” w gutterze. Stan zapisany w sesji review.
- **Kryteria akceptacji:** testy komponentów: potwierdzenie i pominięcie z powodem trafiają do stanu sesji; pozycja `blocking` bez decyzji jest widoczna dla bramki (P2-19).
- **Rozmiar:** ~250 LOC.

### 2.2 Trasa

#### P2-08 · Jednostki zmiany

- **Zależy od:** P1-22
- **Zakres:** przypisanie hunków do symboli (metoda, właściwość, klasa, element XAML). Kilka hunków jednej metody to jedna jednostka. Hunk poza symbolem staje się jednostką „poziom pliku”.
- **Kryteria akceptacji:** testy: trzy hunki w `Analyze` i `Dispose` dają dwie jednostki; hunk z `using` to jednostka poziomu pliku; identyfikator jednostki jest stabilny przy ponownym liczeniu.
- **Rozmiar:** ~200 LOC.

#### P2-09 · Krawędzie grafu: C#

- **Zależy od:** P2-08
- **Zakres:** `lupa-syntax`:
  - krawędzie silne definicja → użycie w zbiorze zmienionych jednostek (dopasowanie nazw z uwzględnieniem przestrzeni nazw i `using`, bez pełnej semantyki);
  - test → testowany symbol (konwencja `XTests` → `X`, wywołania w ciele testu);
  - słabe: ten sam plik.

  Każda krawędź ma powód do pokazania w UI.
- **Kryteria akceptacji:** testy na przykładzie !482 z `docs/input/plan.md` odtworzonym w `lupa-testkit`: krawędzie `ICameraFrameSource` → `FrameAnalyzer`, `FrameAnalyzerTests` → `FrameAnalyzer.Analyze`, `FrameAnalyzer.Analyze` → `ScannerPage.OnFrameReady`; brak krawędzi między niezwiązanymi metodami o tej samej nazwie w różnych przestrzeniach nazw.
- **Rozmiar:** ~380 LOC.

#### P2-10 · Krawędzie grafu: XAML i DI

- **Zależy od:** P2-09
- **Zakres:** XAML ↔ code-behind (`x:Class`), XAML → ViewModel (`x:DataType`), rejestracja w DI → typ (`AddTransient<T>`, `AddSingleton<T>`, `AddScoped<T>` i warianty z interfejsem). Historyczne współzmiany zostają na fazę 3 (P3-20).
- **Kryteria akceptacji:** testy: `ScannerPage.xaml` ↔ `ScannerPage.xaml.cs`; `MauiProgram` → `FrameAnalyzer`; `x:DataType` z przestrzenią nazw `clr-namespace` rozwiązuje się na typ.
- **Rozmiar:** ~300 LOC.

#### P2-11 · Algorytm trasy: wątki i kolejność

- **Zależy od:** P2-10
- **Zakres:** `packages/domain`, kroki 1–5 algorytmu z `docs/input/plan.md`:
  - odsiew szumu do kroku „kosmetyka”;
  - wątki jako spójne składowe po krawędziach silnych;
  - kolejność wątków: zależne po swoich zależnościach, przy remisie wyższe ryzyko, trywialne na końcu.
- **Kryteria akceptacji:** testy: !482 to jeden wątek; MR z niezwiązaną zmianą ustawień języka daje dwa wątki i flagę „sugestia podziału”; test właściwości: permutacja kolejności wejścia daje identyczną trasę (1 000 losowych permutacji z ustalonym ziarnem).
- **Rozmiar:** ~300 LOC.

#### P2-12 · Algorytm trasy: warstwy, budżet i kroki zamykające

- **Zależy od:** P2-11
- **Zakres:** kroki 6–8 algorytmu:
  - szablon warstw z `config.yaml` (kontrakt → testy → logika → ViewModel → widok → kod platformowy → DI i konfiguracja), w warstwie definicje przed użyciami, cykle łączone w jeden krok;
  - przydział jednostek do warstw według reguł z `config.yaml`, z domyślnymi:
    - kontrakt: interfejsy, rekordy i klasy DTO;
    - testy: projekty i pliki testowe;
    - ViewModel: `*ViewModel.cs`;
    - widok: code-behind `*.xaml.cs`, potem `*.xaml`;
    - kod platformowy: `Platforms/**`;
    - DI i konfiguracja: `MauiProgram.cs`, `*.csproj`, manifesty;
    - logika: reszta;
  - krawędź XAML → code-behind jest skierowana (XAML używa code-behind), więc para nie tworzy cyklu i daje dwa kroki;
  - kierunek ustawiany w `config.yaml`: od definicji albo od punktu wejścia;
  - czas kroku: LOC × mnożnik ryzyka × mnożnik nowości (czy recenzent zmieniał lub przeglądał plik w ostatnich 6 miesiącach);
  - podział na przejścia przy ok. 60 min lub 400 LOC, zawsze na granicy wątku;
  - kroki „kosmetyka” i „Całość”.

  Nazwy kroków deterministyczne: warstwa i główny symbol, np. „Kontrakt · ICameraFrameSource”.
- **Kryteria akceptacji:** test: trasa dla !482 ma 6 kroków w kolejności z tabeli w `docs/input/plan.md` (`ICameraFrameSource` → kontrakt, `FrameAnalyzerTests` → testy, `FrameAnalyzer` → logika, `ScannerPage.xaml.cs` → widok krok 4, `ScannerPage.xaml` → widok krok 5, `MauiProgram` → DI), z `Resource.designer.cs` w krokach zwiniętych i krokiem „Całość” na końcu; test: MR 900 LOC z dwoma wątkami dzieli się na dwa przejścia na granicy wątku; test: zmiana kierunku w konfiguracji odwraca kolejność w warstwie.
- **Rozmiar:** ~350 LOC.

#### P2-13 · Trasa w lewej kolumnie

- **Zależy od:** P2-12, P1-21, P1-23
- **Zakres:**
  - kroki z nazwą, plikami, LOC i powodem kolejności („Teraz, bo cykl życia strony w kroku 4 opiera się na tej logice”), hunki rozwinięte w bieżącym kroku;
  - pasek postępu i odwiedziny; znaczniki „checklista: zdarzenia”, „zmiana w testach”;
  - przełącznik „Trasa / Wszystkie pliki”; przeciąganie kroku; `n`/`p` po krokach.

  Trasa nie przestawia się w trakcie sesji.
- **Kryteria akceptacji:** testy komponentów dla stanów z makiety `Main`: krok bieżący z rozwiniętymi hunkami („1 · sygnatura”, „2 · ciało Analyze”, „3 · Dispose()”), krok odwiedzony, krok ze znacznikiem checklisty, krok „zmiana w testach potwierdzona”, grupa „Zwinięte automatycznie”, tryb „Wszystkie pliki (7)”; test: przeciągnięcie kroku zapisuje własną kolejność w sesji, a nowe dane z rdzenia jej nie nadpisują.
- **Rozmiar:** ~380 LOC.

#### P2-14 · Briefing deterministyczny

- **Zależy od:** P2-05, P2-12, P1-27
- **Zakres:** ekran według makiety `Briefing`:
  - nagłówek MR (autor, gałęzie, pliki, `+`/`−`, pipeline, „nowa wersja od Twojego review”);
  - cel według opisu MR (bezpieczny Markdown);
  - proponowana trasa z czasem; rozmiar sesji z informacją o zwiniętych plikach generowanych; mapa ryzyka plików;
  - sugestia podziału MR przy dwóch wątkach;
  - `Enter` „Zacznij trasę”, „Wszystkie pliki alfabetycznie”.

  Sekcje „Co diff faktycznie robi” i „Rozbieżności z opisem” są ukryte do fazy 3. Ten sam ekran działa dla celów lokalnych (bez opisu MR).
- **Kryteria akceptacji:** testy komponentów: MR z opisem, bez opisu, cel lokalny, dwa wątki; test Playwright: skrzynka → briefing → trasa z klawiatury.
- **Rozmiar:** ~350 LOC.

### 2.3 Komentarze i publikacja

#### P2-15 · Pozycja komentarza GitLaba

- **Zależy od:** P1-15, P1-26
- **Zakres:** funkcja w `lupa-gitlab` licząca `position` z `diff_refs` (`base_sha`, `start_sha`, `head_sha`), ścieżek i `old_line`/`new_line` oraz `line_range` dla zakresu. Kotwiczenie komentarzy to według `docs/input/plan.md` najczęstsze źródło błędów, dlatego zadanie ma wyłącznie logikę i testy. Spike S2 pokazał, że w ok. 0,3% plików nasz diff może przesunąć blok zmian o linię względem Gita („ślizgacz”). Dlatego przed wysłaniem draftu pozycja jest sprawdzana z diffem wersji MR z API, a przy rozbieżności wygrywa pozycja GitLaba.
- **Kryteria akceptacji:** test: przy rozbieżności ślizgacza (linia usunięta u nas, kontekst w diffie GitLaba) draft dostaje pozycję z diffu GitLaba; testy tablicowe: linia dodana, usunięta, kontekst, zakres przez dodane i kontekstowe, plik ze zmianą nazwy, plik usunięty, pierwszy i ostatni wiersz pliku, plik bez końcowego nowego wiersza; wartości pól zgodne z dokumentacją Draft Notes API dla każdego przypadku.
- **Rozmiar:** ~300 LOC.

#### P2-16 · Edytor komentarza i drafty lokalne

- **Zależy od:** P2-15, P1-18a
- **Zakres:**
  - `c` na linii lub zaznaczeniu otwiera edytor inline pod linią;
  - szablony Conventional Comments (`issue`, `suggestion`, `question`, `nitpick` + `blocking`/`non-blocking`);
  - propozycja kodu jako blok sugestii GitLaba z zaznaczonych linii;
  - podgląd Markdown bezpiecznym komponentem;
  - draft zapisany lokalnie natychmiast; licznik „Drafty” w pasku górnym.
- **Kryteria akceptacji:** testy komponentów: szablon wstawia prefiks etykiety; sugestia zawiera zaznaczone linie strony „head”; zamknięcie aplikacji z otwartym edytorem nie gubi treści.
- **Rozmiar:** ~350 LOC.

#### P2-17 · Synchronizacja draftów z GitLabem

- **Zależy od:** P2-16
- **Zakres:** tworzenie, edycja i usuwanie `draft_notes` ([Draft Notes API](https://docs.gitlab.com/api/draft_notes/)) dla celów MR, stan synchronizacji per draft, ponowienia. `position` w draft notes działa od GitLaba 16.3. Na starszej instancji drafty zostają lokalnie, a publikacja tworzy wątki z pozycją jednym ruchem w P2-21 (endpoint i jego zachowanie sprawdza zadanie w dokumentacji). Dla celów lokalnych drafty zostają notatkami lokalnymi (P2-26).
- **Kryteria akceptacji:** testy na atrapie API: utworzenie z poprawnym `position`; edycja i usunięcie; dla wersji < 16.3 brak wywołań `draft_notes` z `position`, a publikacja w P2-21 tworzy jeden wątek z pozycją na każdy lokalny draft (endpoint wątków z pozycją potwierdzony przez `fact-checker` przed kodem); błąd 4xx zostawia draft lokalnie z komunikatem; brak duplikatów po ponowieniu po timeoucie (idempotencja po lokalnym identyfikatorze).
- **Rozmiar:** ~300 LOC.

#### P2-18 · Odpowiedzi i rozwiązywanie wątków

- **Zależy od:** P2-17, P1-26
- **Zakres:** „Odpowiedz” (`r`) i „Rozwiąż wątek” (`e`) na istniejących wątkach GitLaba (skróty z makiet `Main` i `Interdiff`). Odpowiedź jako draft, publikowana razem z review. Rozwiązanie wątku przez parametr draftu albo osobne wywołanie, według dokumentacji i wersji instancji (fakt **NIEZWERYFIKOWANY**, sprawdza `fact-checker` przed zadaniem).
- **Kryteria akceptacji:** wynik weryfikacji mechanizmu rozwiązania wątku (`fact-checker`) z linkiem w opisie zmiany przed kodem; testy na atrapie API dla odpowiedzi i rozwiązania według tego wyniku; skróty `r` i `e` działają na wątku z fokusem; stan „Wątek rozwiązany, opublikuje się razem z review” z cofnięciem, jak w makiecie `Interdiff`.
- **Rozmiar:** ~250 LOC.

#### P2-19 · Reguły bramki

- **Zależy od:** P2-07, P2-13, P2-17
- **Zakres:** `packages/domain`, czysta funkcja: stan sesji → lista sprawdzeń i dostępne decyzje.
  - **Sprawdzenia:**
    - otwarte uwagi blokujące (drafty i wątki) z linkiem do kroku;
    - nieodwiedzone kroki;
    - pozycje checklisty bez decyzji;
    - zmiany w testach bez potwierdzenia;
    - status pipeline'u;
    - nieprzeczytane wątki GitLaba;
    - LOC i czas sesji.
  - **Decyzje:** `Approve` niedostępne przy otwartej uwadze blokującej; `Requested changes` rekomendowane, gdy jest uwaga blokująca; `Reviewed` zawsze dostępne.
- **Kryteria akceptacji:** testy tablicowe dla każdej kombinacji, w tym stany z makiety `Gate` (5 z 6 sprawdzeń, 1 uwaga blokująca, checklista 8 z 9 z pominięciem z powodem).
- **Rozmiar:** ~250 LOC.

#### P2-20 · Ekran bramki

- **Zależy od:** P2-19
- **Zakres:** ekran według makiety `Gate`:
  - sprawdzenia z akcją „Wróć do kroku N”;
  - lista draftów z edycją i usuwaniem;
  - wybór decyzji z opisem, dlaczego opcja jest niedostępna;
  - notatka podsumowująca (w tej fazie pisana ręcznie, szablon z listy uwag blokujących);
  - przycisk publikacji z nazwą decyzji.
- **Kryteria akceptacji:** testy komponentów dla stanów z makiety `Gate`: „Sprawdzenia · 5 z 6” z otwartą uwagą blokującą i `Approve` niedostępnym, wszystkie sprawdzenia zielone z `Approve` dostępnym, decyzja `Reviewed`, decyzja `Requested changes` z etykietą „rekomendowane”, ekran „Opublikowano w GitLabie”; test Playwright: trasa → bramka → wybór decyzji z klawiatury.
- **Rozmiar:** ~350 LOC.

#### P2-21 · Publikacja

- **Zależy od:** P2-20
- **Zakres:**
  - `bulk_publish` z notatką i `reviewer_state` (`reviewed` albo `requested_changes`), dostępne od GitLaba 19.2; dla `Approve` dodatkowo `approve` po udanej publikacji (`bulk_publish` nie zapisuje formalnego approve);
  - wariant zastępczy dla GitLaba 15.11–19.1: `bulk_publish` bez notatki, potem notatka jako osobny komentarz MR. Stanu recenzenta nie ustawiamy, a UI informuje o ograniczeniu. Jeśli `fact-checker` potwierdzi endpoint stanu recenzenta dla wersji zespołu, dochodzi on jako trzecie wywołanie;
  - zapis `head_sha` i wersji MR;
  - ekran „Opublikowano” z „Następny MR w skrzynce”.
- **Kryteria akceptacji:**
  - testy na atrapie API: kolejność wywołań dla 19.2 (`bulk_publish` z notatką i `reviewer_state`, potem `approve`) i dla 18.x (`bulk_publish`, komentarz z notatką, potem `approve`); częściowa awaria (publikacja OK, approve błąd) daje stan do ponowienia tylko approve;
  - drugi klik „Opublikuj” nie wysyła drugi raz;
  - publikacja odrzucona, gdy `head_sha` w GitLabie różni się od tego w sesji (przejście do P2-22a).
- **Rozmiar:** ~300 LOC.

#### P2-22a · Nowa wersja w trakcie review

- **Zależy od:** P2-21
- **Zakres:** nowy push w trakcie review: przeliczenie pozycji draftów na nowe `diff_refs`. Linie niezmienione przenoszone automatycznie, zmienione do potwierdzenia przez recenzenta. Ostrzeżenie w pasku górnym. Kotwiczenie to według `docs/input/plan.md` najczęstsze źródło błędów, dlatego zadanie jest osobne.
- **Kryteria akceptacji:** testy tablicowe: draft na linii bez zmian przenosi się z nowym numerem; draft na linii zmienionej dostaje stan „do potwierdzenia”; draft w pliku usuniętym w nowej wersji dostaje stan „plik usunięty”; draft z zakresem częściowo zmienionym jest do potwierdzenia.
- **Rozmiar:** ~300 LOC.

#### P2-22b · Praca offline

- **Zależy od:** P2-22a
- **Zakres:** bez sieci drafty trafiają do kolejki, synchronizacja po powrocie, blokada publikacji do czasu synchronizacji. Konflikt z nową wersją MR przechodzi przez P2-22a.
- **Kryteria akceptacji:** testy: utrata sieci w połowie synchronizacji nie gubi ani nie dubluje draftów; publikacja jest zablokowana z komunikatem, dopóki kolejka nie jest pusta.
- **Rozmiar:** ~200 LOC.

### 2.4 Interdiff

#### P2-23 · Interdiff po zwykłym pushu

- **Zależy od:** P2-21, P1-24
- **Zakres:** ekran według makiety `Interdiff`:
  - przełącznik „Od mojego review · vN → vM” i „Cały MR”;
  - zwykły push (stary head jest przodkiem nowego): diff `head_stary..head_nowy`;
  - grupy „Do przejrzenia” i „Bez zmian od vN” (z zachowanym „viewed”);
  - LOC i szacowany czas interdiffu;
  - znacznik „nowa wersja od Twojego review” w skrzynce i briefingu (porównanie `head_sha` z zapisanym w P2-21, z liczbą commitów i LOC).
- **Kryteria akceptacji:** testy: rozpoznanie zwykłego pushu po przodku; plik bez zmian zachowuje „viewed”; zmiana pliku zdejmuje znacznik; znacznik w skrzynce pojawia się tylko, gdy `head_sha` różni się od zapisanego.
- **Rozmiar:** ~350 LOC.

#### P2-24 · Interdiff po rebase

- **Zależy od:** P2-23
- **Zakres:** gdy stary head nie jest przodkiem nowego: porównanie łatek per plik (diff stary `base_stary..head_stary` kontra nowy `base_nowy..head_nowy`), z odfiltrowaniem zmian wniesionych przez nową bazę. Widok „zmiana zmiany” w stylu `git range-diff`, z opisem metody w UI („rebase: porównuję łatki, nie heady”).
- **Kryteria akceptacji:** testy na repo z `lupa-testkit`: rebase bez zmian w łatce daje pusty interdiff; rebase z jedną poprawką pokazuje tylko ją; zmiany z nowej bazy nie pojawiają się w interdiffie.
- **Rozmiar:** ~380 LOC.

#### P2-25 · Wątki i komentarze między wersjami

- **Zależy od:** P2-23, P2-18
- **Zakres:**
  - mapowanie pozycji wątków z vN na vM („wątek przeniesiony z v3 R46 na v4 R46–47”);
  - stany: „rozwiązane przez zmianę” (linia zmieniona przez autora), „linia bez zmian, bez odpowiedzi”;
  - panel „Twoje komentarze z vN” z akcjami „Rozwiąż” i „Przypomnij autorowi” (draft odpowiedzi);
  - informacja dla bramki, czy wątek blokujący jest nadal otwarty.
- **Kryteria akceptacji:** testy mapowania na przykładach z makiety `Interdiff` (R46 → R46–47 przy zmianie linii, R52 → R53 przy przesunięciu); testy komponentów panelu dla stanów: wątek blokujący z odpowiedzią autora, wątek rozwiązany z cofnięciem, „bez odpowiedzi” z „Przypomnij autorowi”, nitpick z odpowiedzią i „Rozwiąż”, brak otwartych uwag blokujących.
- **Rozmiar:** ~350 LOC.

### 2.5 Tryby i kontekst

#### P2-26 · Self-review i review bez MR

- **Zależy od:** P2-14, P2-20, P1-13
- **Zakres:**
  - dla celów `WorkingTree`, `Commit` i `Range` ta sama trasa, checklisty i bramka;
  - zamiast publikacji zakończenie „Notatki”: lista komentarzy z plikiem i linią, eksport do Markdown i schowka; dla `WorkingTree` lista „do poprawy przed pushem”;
  - notatki zostają przy celu i wracają przy ponownym otwarciu;
  - przy lokalnych zmianach notatka, której linia się zmieniła, dostaje stan „linia zmieniona”.
- **Kryteria akceptacji:** test Playwright: lokalne zmiany → trasa → komentarz → bramka → eksport Markdown z poprawnymi ścieżkami i liniami; test: decyzja `Approve` jest niedostępna dla celów bez MR.
- **Rozmiar:** ~300 LOC.

#### P2-27 · Blame i CODEOWNERS

- **Zależy od:** P1-06, P2-07
- **Zakres:** w panelu kontekstu dla bieżącego hunka: autorzy i daty zmienianych linii (blame strony „base”) oraz właściciele z `CODEOWNERS` gałęzi docelowej. Sugestia „poproś o drugiego recenzenta”, gdy właściciel nie jest recenzentem MR.
- **Kryteria akceptacji:** testy blame na repo z `lupa-testkit` zgodne z `git blame --porcelain`; testy dopasowania `CODEOWNERS` (sekcje, wzorce z `**`, ostatnia reguła wygrywa).
- **Rozmiar:** ~300 LOC.

#### P2-28 · Spójność wzorca

- **Zależy od:** P2-08
- **Zakres:** dla hunków, które zastępują wywołanie lub typ (usunięty token X, dodany Y w tej samej jednostce), wyszukanie pozostałych wystąpień X w drzewie „head” (Tree-sitter, bez komentarzy i stringów). Wynik w panelu kontekstu: „Device.BeginInvokeOnMainThread → MainThread zmienione w 1 miejscu, zostały 4 inne” z listą miejsc.
- **Kryteria akceptacji:** testy: wykrycie zamiany na przykładzie z `docs/input/plan.md`; wystąpienia w komentarzach i stringach pominięte; wyszukanie w repo 5 000 plików < 1 s z cache po blobach.
- **Rozmiar:** ~300 LOC.

#### P2-29 · Przeniesiony kod

- **Zależy od:** P1-10, P1-18a
- **Zakres:** wykrywanie bloków przeniesionych między plikami i w pliku (próg minimalnej liczby linii, porównanie po znormalizowanych białych znakach). Przeniesiony kod przygaszony, z linkiem „przeniesione z `Plik.cs:120`”. Zmiany wewnątrz przeniesionego bloku nadal widoczne.
- **Kryteria akceptacji:** testy: przeniesienie metody do innego pliku; przeniesienie ze zmianą jednej linii; bloki poniżej progu nie są oznaczane.
- **Rozmiar:** ~300 LOC.

### 2.6 Wykonanie kodu MR

Różnica ostrzeżeń wymaga buildu base i head, czyli wykonania kodu z repozytorium MR. `CLAUDE.md` pozwala na to tylko ścieżką zatwierdzoną w ADR.

#### P2-30 · Spike i ADR: model zaufania dla wykonania kodu MR

- **Zależy od:** —
- **Zakres:** spike `mr-code-execution-isolation` i ADR. Pytania:
  - które warianty działają na Windows zespołu: osobny proces z oczyszczonym środowiskiem; osobne konto lokalne; Windows Sandbox; AppContainer; artefakt SARIF z CI zamiast lokalnego buildu;
  - jak blokować sieć i dostęp do repo dewelopera;
  - jak przygotować kopię kodu do wykonania, żeby nie dawała dostępu do `.git` repo dewelopera (worktree, osobny klon, `git archive`), z testem wrogiego builda zapisującego `.git/hooks` i `.git/config`;
  - czy semantyka „head” może korzystać z ładowania projektów gałęzi docelowej (ładowanie przez `MSBuildWorkspace` to design-time build w procesie BuildHost, czyli wykonanie tasków i targetów) (kod już po review) i samych plików `.cs` z MR, z flagą, gdy MR zmienia `.csproj`, `Directory.Build.*` albo `.targets`.

  ADR opisuje model zaufania, granicę strefy D z `architecture.md`, limity czasu i to, co wraca jako niezaufane dane.
- **Kryteria akceptacji:** spike zamknięty z dowodami (`/spike close`); ADR w statusie „Proponowany” z przeglądem `security-reviewer` bez otwartych znalezisk wysokich i krytycznych; decyzja człowieka (status „Zaakceptowany”) przed P2-32.
- **Rozmiar:** spike + ADR, bez kodu aplikacji. Limit spike'a: 12 h.
- **Przegląd:** security.

#### P2-31 · Kopia kodu per cel w katalogu cache

- **Zależy od:** P1-04, P1-15, P2-30 (zaakceptowany ADR)
- **Zakres:** kopia kodu base i head w katalogu cache narzędzia (nigdy w working copy dewelopera), metodą z ADR z P2-30. `git worktree add` z `docs/input/plan.md` współdzieli `.git` z repo dewelopera, więc kod z MR uruchomiony w worktree mógłby zapisać hooki lub `core.hooksPath` w repo dewelopera. ADR rozstrzyga między worktree z ochroną, osobnym klonem a eksportem drzewa (`git archive`). Dalej: z limitem liczby i miejsca, sprzątaniem (`worktree prune`) i blokadą równoległego użycia. Ścieżki krótkie (limit długości ścieżek Windows).
- **Kryteria akceptacji:** testy: worktree powstaje wewnątrz katalogu cache także przy nazwach gałęzi z `..`, `/` i znakami spoza ASCII; po przekroczeniu limitu najstarszy worktree jest usuwany; usunięcie kopii nie dotyka repo dewelopera (test sprawdza `git status` repo źródłowego); skrypt w kopii, który próbuje zapisać `.git/hooks` i `.git/config`, nie zmienia repo dewelopera.
- **Rozmiar:** ~250 LOC.
- **Przegląd:** security.

#### P2-32 · Runner buildu w izolacji

- **Zależy od:** P2-30 (zaakceptowany ADR), P2-31
- **Zakres:** `lupa-exec`:
  - uruchomienie `dotnet build` projektów bibliotek i testów (bez pakowania aplikacji Android) na base i head według ADR z P2-30, z diagnostykami do SARIF przez właściwość `ErrorLog` ([opcje kompilatora C#](https://learn.microsoft.com/dotnet/csharp/language-reference/compiler-options/errors-warnings)). W wierszu poleceń przecinek trzeba zakodować: `-p:ErrorLog=<plik>.sarif%2Cversion=2.1`, bo bez `version` powstaje SARIF 1;
  - środowisko z listy dozwolonych zmiennych (bez tokenu), limit czasu, anulowanie, postęp w tle;
  - cache wyników po SHA.

  Jeśli ADR wybierze SARIF z CI, zadanie pobiera artefakt zamiast buildu lokalnego.
- **Kryteria akceptacji:**
  - test: środowisko procesu nie zawiera zmiennych spoza listy dozwolonych (atrapa procesu wypisuje środowisko);
  - test: przekroczenie limitu czasu zabija drzewo procesów;
  - test integracyjny na projekcie testowym .NET z ostrzeżeniem w head: SARIF zawiera ostrzeżenie.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security.

#### P2-33 · Różnica ostrzeżeń

- **Zależy od:** P2-32
- **Zakres:**
  - porównanie SARIF base i head: odcisk ostrzeżenia z `ruleId`, ścieżki i znormalizowanego tekstu linii (nie numeru linii);
  - tylko ostrzeżenia wprowadzone przez zmianę: znacznik „analizator” w gutterze, wpis w panelu kontekstu, sprawdzenie w bramce;
  - w interdiffie stan „rozwiązane w v4”;
  - SARIF parsowany jako niezaufane dane, z limitem rozmiaru.
- **Kryteria akceptacji:** testy: ostrzeżenie przesunięte o 10 linii nie jest nowe; nowe ostrzeżenie w zmienionej linii jest; SARIF 100 MB odrzucony z komunikatem; ścieżki w SARIF spoza worktree są ignorowane.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security.

### 2.7 Dystrybucja

#### P2-34 · Auto-update

- **Zależy od:** P1-29, kanał dystrybucji wybrany w zadaniu (D4 w `decisions.md`)
- **Zakres:** updater Tauri z wewnętrznego kanału, podpisane aktualizacje, sprawdzanie w tle i instalacja po potwierdzeniu użytkownika. Klucz podpisu poza repo (zmienna środowiskowa w CI).
- **Kryteria akceptacji:** aktualizacja z wersji N do N+1 na Windows z kanału testowego (zapis kroków i wyniku w opisie zmiany); aktualizacja z błędnym podpisem jest odrzucona.
- **Rozmiar:** ~200 LOC.
- **Przegląd:** security.

#### P2-35 · Ryzyko i szacowany czas w skrzynce

- **Zależy od:** P2-03, P2-05, P2-12
- **Zakres:** po prefetchu w tle liczenie ryzyka MR z powodem, trasy i szacowanego czasu oraz liczby przejść. Wyniki w cache po `head_sha` i wersji `.review/`. Skrzynka pokazuje je od razu po starcie.
- **Kryteria akceptacji:** testy: wynik dla tego samego `head_sha` liczony raz; zmiana `.review/` w gałęzi docelowej unieważnia wynik; MR 900 LOC dostaje plakietkę „Duży: 2 przejścia”.
- **Rozmiar:** ~200 LOC.

#### P2-36 · Trasa po nowej wersji i interdiff celów lokalnych

- **Zależy od:** P2-23, P2-13
- **Zakres:**
  - po nowym pushu trasa przelicza się tylko dla zmienionych jednostek, a odwiedzone kroki bez zmian zostają odhaczone (wymaganie z `docs/input/plan.md`, „Zmiany w trakcie”);
  - dla celów lokalnych `previous` to ostatnio przejrzana migawka, a widok „od ostatniego przeglądu” działa jak interdiff MR.
- **Kryteria akceptacji:** testy: zmiana jednej jednostki zostawia odwiedzone pozostałe kroki; nowa jednostka trafia do właściwego kroku albo do nowego kroku na końcu wątku; dla `WorkingTree` edycja jednego pliku po przeglądzie pokazuje tylko ten plik w widoku „od ostatniego przeglądu”.
- **Rozmiar:** ~300 LOC.

#### P2-37 · Konfiguracja organizacji

- **Zależy od:** P1-14b, P2-34
- **Zakres:** URL GitLaba, dostawca AI i kanał aktualizacji ustawiane per organizacja (wymaganie z `docs/input/plan.md`, „Dystrybucja”). Plik konfiguracji organizacji dołączany do instalatora albo wskazany ścieżką, z pierwszeństwem przed ustawieniami użytkownika tam, gdzie organizacja to zablokuje. Bez sekretów w pliku.
- **Kryteria akceptacji:** testy: wartość zablokowana przez organizację nie da się zmienić w ustawieniach; plik z polem wyglądającym na token jest odrzucony.
- **Rozmiar:** ~200 LOC.
- **Przegląd:** security.

## Bramka wyjścia z fazy 2

1. Przez 2 sprinty co najmniej 80% review w projekcie MAUI robionych przez członków zespołu z Lupą zostaje opublikowanych z Lupy (licznik publikacji kontra liczba review z API).
2. Ręczna kontrola 50 opublikowanych komentarzy: 0 w złej linii.
3. Interdiff poprawny na zestawie testowym (zwykły push i rebase) oraz na co najmniej 5 prawdziwych MR z nowymi wersjami.
4. Self-review lokalnych zmian użyte przez co najmniej 3 autorów przed pushem.
5. ADR z P2-30 zaakceptowany, `security-reviewer` bez otwartych znalezisk wysokich i krytycznych dla zadań security tej fazy.
6. Budżety wydajności z fazy 1 nadal spełnione (`npm run bench`).
