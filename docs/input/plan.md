> **Pochodzenie:** tekst wklejony przez @Bartek w sesji Claude Code 2026-10-10 i przeniesiony do repo bez zmian merytorycznych. Cztery osadzone diagramy (architektura, przepływ review, graf !482, roadmapa) nie przeszły do wersji tekstowej. W ich miejscu są znaczniki *[osadzony diagram: …]*.

# Guided Code Review — plan narzędzia

Oct 10, 2026 · @Bartek

## Cel i zasady projektowe

Narzędzie prowadzi recenzenta przez zmianę w kolejności, która najmniej kosztuje zrozumienie. Wnioski z badań nad review zamienia w domyślne zachowania interfejsu. AI proponuje, człowiek decyduje, a GitLab pozostaje źródłem prawdy.

- **Trasa zamiast listy plików.** Diff czyta się w kolejności: intencja, kontrakt, testy, implementacja, UI, konfiguracja. Kolejność alfabetyczna jest tylko trybem alternatywnym.
- **Małe porcje pilnowane przez UI.** Licznik linii i czasu sesji. Przy ok. 400 LOC lub 60 minutach narzędzie proponuje przerwę albo podział na przejścia (heurystyka z badania SmartBear/Cisco).
- **Najpierw reguły deterministyczne, potem AI.** Analizatory, SARIF z buildu i reguły zespołu działają zawsze. AI dokłada hipotezy tam, gdzie reguły nie sięgają.
- **Uwaga AI to hipoteza z dowodem.** Każda wskazuje linie, kategorię i pewność. Nic nie trafia do GitLaba bez akceptacji człowieka.
- **GitLab jako backend.** Wątki, drafty, approvale i pipeline'y zostają w GitLabie. Lokalnie trzymamy tylko cache i stan sesji, bez własnego serwera.
- **Lokalnie i szybko.** Diffy liczone z lokalnego repo, API tylko po metadane.
- **Reguły zespołu w repo.** Checklisty, prompty i reguły ryzyka są wersjonowane razem z kodem i przechodzą review jak kod.

## Build vs buy

Własne narzędzie ma sens tylko wtedy, gdy wygrywa prowadzeniem człowieka i regułami zespołu. Samo „AI komentuje MR” jest już dostępne gotowe. Dlatego faza 0 zaczyna się od tygodnia testów istniejących opcji.

| Opcja | Co daje | Czego brakuje względem guided review |
| --- | --- | --- |
| GitLab web UI (z Duo Code Review, jeśli macie licencję) | natywne wątki, drafty, approvale | brak trasy i checklist kontekstowych; AI zależne od licencji Duo |
| CodeRabbit (obsługuje GitLab) | automatyczne AI review w MR | bot komentujący, nie asystent recenzenta; kod u kolejnego dostawcy |
| Claude Code w GitLab CI/CD | review w pipeline, na Waszym dostępie do Claude | brak interaktywnego widoku dla człowieka |
| Rider | dobry diff, inspekcje, nawigacja | brak trasy i reguł zespołu; integrację z MR GitLaba sprawdźcie w Waszej wersji |

Rekomendacja: budować, ale nie konkurować z botami. Claude Code w CI może być „pierwszym sitem” na każdym MR. Narzędzie lokalne jest dla człowieka, który podejmuje decyzję.

## Architektura

Rekomendacja: aplikacja desktop w Tauri 2, na start pod Windows (WebView2). Gorąca ścieżka (Git, diff, cache) w małym rdzeniu w Ruście, UI w React + TypeScript z CodeMirror 6. Analiza C# trafia do sidecara .NET, czyli tam, gdzie zespół ma kompetencje.

*[osadzony diagram: architektura · komponenty i połączenia]*

UI rozmawia wyłącznie z rdzeniem. Rdzeń sam decyduje, co liczy lokalnie, a co pobiera z GitLaba lub zleca Claude, więc interfejs nigdy nie czeka na sieć.

| Decyzja | Wybór | Alternatywa | Dlaczego |
| --- | --- | --- | --- |
| Powłoka | Tauri 2 | Electron; Avalonia | mały narzut pamięci; webowe komponenty diffa są najlepsze na rynku |
| Git | gitoxide lub libgit2 (odczyt) + `git` CLI (fetch, worktree) | LibGit2Sharp | odczyt obiektów w milisekundach, bez procesu na każde wywołanie |
| Algorytm diff | histogram (imara-diff) + diff słów w hunku | Myers | czytelniejsze hunki w kodzie, ten sam wariant co `git --histogram` |
| Widok diffa | CodeMirror 6 + `@codemirror/merge` | Monaco diff editor | lekki, łatwe dekoracje (findingi, SARIF, wątki), zwijanie niezmienionych |
| Struktura kodu | Tree-sitter (C#, XAML, TypeScript) | regexy | nagłówek hunka z nazwą metody, reguły dopasowane do składni |
| Semantyka C# | sidecar .NET z Roslyn | tylko SARIF z buildu | wywołania zmienionej metody, analizatory na zmienionych plikach |
| Stan lokalny | SQLite | pliki JSON | sesje, „viewed”, cache wyników kluczowany SHA |
| Sekrety | Windows Credential Manager | plik konfiguracyjny | token GitLaba nie ląduje na dysku jawnym tekstem |

Dlaczego nie wtyczka do Ridera: Kotlin, UX ograniczony do paneli IDE i cykl wydawniczy platformy. Zamiast tego każdy hunk ma „Otwórz w Riderze” na konkretnej linii (launcher z parametrem `--line`).

Jeśli zespół chce zostać przy samym .NET: Avalonia + AvaloniaEdit + LibGit2Sharp jest wykonalne, ale z niższym sufitem UX widoku diffa. Rust w rekomendowanym wariancie ogranicza się do Gita, diffa i IPC, czyli małej i stabilnej części.

## Integracja z Git i GitLab

Diffy liczymy lokalnie z Gita, a GitLab API służy do metadanych, wątków i publikacji review. Każde review to para (base, head), opcjonalnie z poprzednią wersją do interdiffu.

- **Branch vs branch:** `merge-base(target, source)` → `source`.
- **Commit lub zakres:** `A..B` albo pojedynczy commit względem rodzica.
- **Merge request:** fetch refu `refs/merge-requests/<iid>/head` do lokalnego repo. Base, start i head bierzemy z `diff_refs` MR, żeby diff był identyczny z GitLabem, a komentarze trafiały w te same linie.
- **Interdiff („co się zmieniło od mojego review”):** zapamiętujemy `head_sha` przy publikacji. Po zwykłym pushu pokazujemy `head_stary..head_nowy`, po rebase porównanie łatek w stylu `git range-diff`.
- **Worktree per MR:** `git worktree add` w katalogu cache narzędzia. Build, analizatory i testy działają tam, nie w working copy dewelopera.

| Potrzeba | Endpoint REST v4 |
| --- | --- |
| Skrzynka „do mojego review” | `GET /merge_requests?scope=reviews_for_me&state=opened` |
| Metadane i `diff_refs` | `GET /projects/:id/merge_requests/:iid` |
| Wersje MR (interdiff) | `GET /projects/:id/merge_requests/:iid/versions` |
| Istniejące wątki inline | `GET /projects/:id/merge_requests/:iid/discussions` |
| Komentarz roboczy na linii | `POST .../merge_requests/:iid/draft_notes` z `position` (`base_sha`, `start_sha`, `head_sha`, ścieżki, `old_line`/`new_line`, `line_range`) |
| Publikacja całego review | `POST .../draft_notes/bulk_publish` z notatką podsumowania i `reviewer_state` (`reviewed` lub `requested_changes`) |
| Approve | `POST .../merge_requests/:iid/approve` |
| Status pipeline'u | `GET .../merge_requests/:iid/pipelines` |

Drafty są widoczne tylko dla autora do publikacji, więc recenzent buduje review spokojnie i publikuje je jednym ruchem ([Draft Notes API](https://docs.gitlab.com/api/draft_notes/)). `bulk_publish` ustawia stan recenzenta, ale nie zapisuje formalnego approve, stąd osobne wywołanie.

Kotwiczenie komentarzy to najczęstsze źródło błędów. Draft zawsze tworzymy względem aktualnych `diff_refs`. Gdy autor wypchnie nową wersję w trakcie review, narzędzie przelicza linie draftów albo prosi o ich potwierdzenie.

Auth: OAuth z PKCE (bez ręcznych tokenów) lub Personal Access Token ze scope `api`, trzymany w keychainie. URL instancji jest konfigurowalny dla self-managed. Odświeżanie skrzynki przez polling z `updated_after`, bo webhooki nie dotrą do aplikacji lokalnej ([Merge requests API](https://docs.gitlab.com/api/merge_requests/)).

**Wersja instancji.** GitLab jest self-managed w nieznanej wersji. Przy starcie narzędzie odczytuje ją z `GET /api/v4/metadata` lub `GET /api/v4/version` i wyłącza funkcje, których instancja nie obsługuje, z czytelnym komunikatem. Spike w fazie 0 ustala minimalne wersje dla używanych endpointów.

## Warstwa AI na Claude Enterprise

AI działa przez Claude Code w trybie nieinteraktywnym (`claude -p`) jako proces potomny rdzenia. Ma dostęp tylko do odczytu, kontekst repo i reguły zespołu. Zwraca ustrukturyzowane hipotezy, nigdy automatycznie publikowane komentarze.

**Wywołanie.** `--output-format stream-json` daje postęp na żywo, a `--output-format json` z `--json-schema` zwraca wynik zgodny ze schematem w polu `structured_output`. Follow-upy w tej samej sesji idą przez `--resume <session_id>` ([dokumentacja trybu headless](https://code.claude.com/docs/en/headless)).

**Uprawnienia.** W sesji review tylko `Read`, `Grep`, `Glob` i `--permission-mode dontAsk`. Bez `Bash` i `Edit`: recenzent ocenia kod, nie zmienia go.

**Bezpieczeństwo: kod z MR to niezaufane dane.** Bez `--bare` sesja `-p` uruchamia hooki z `.claude/settings.json` i serwery z `.mcp.json` katalogu roboczego, bez pytania o zaufanie. Gałąź MR mogłaby więc wykonać kod na maszynie recenzenta. Dlatego konfiguracja AI pochodzi z gałęzi docelowej, katalogiem roboczym sesji jest zaufany folder narzędzia, a sesja dostaje tylko narzędzia do odczytu. Seaty nie działają w trybie `--bare`.

**Dostęp i rozliczenie: decyzja z 10.10.2026.**

| Ścieżka | Jak | Plusy | Do sprawdzenia |
| --- | --- | --- | --- |
| Klucz API organizacji (Claude Console albo Bedrock, Vertex, Foundry) | `--bare` + `ANTHROPIC_API_KEY` lub `apiKeyHelper` | pełna izolacja od konfiguracji repo, przewidywalne limity | osobny budżet API |
| Login Claude Code z seatów Enterprise | `claude -p` bez `--bare` | bez osobnego klucza | tryb `--bare` nie używa loginu, więc izolacja trudniejsza; dokumentacja Agent SDK zabrania oferowania loginu claude.ai w produktach firm trzecich bez zgody Anthropic |

**Wybrana ścieżka: seaty Claude Code.** Organizacja nie ma dostępu do API, więc narzędzie wywołuje `claude -p` z loginem dewelopera. Tryb `--bare` odpada, bo nie używa loginu, dlatego izolacja jest twardym wymaganiem, nie opcją:

- katalog roboczy sesji to zaufany folder narzędzia z konfiguracją z gałęzi docelowej;
- kod MR jest dostępny przez `--add-dir`, tylko do odczytu;
- uprawnienia to `Read`, `Grep`, `Glob` i `--permission-mode dontAsk`;
- spike w fazie 0 sprawdza, że hooki i `.mcp.json` z worktree MR się nie wczytują.

Ścieżka z kluczem API zostaje w abstrakcji dostawcy jako zapas.

Narzędzie wewnętrzne dla własnego zespołu to inna sytuacja niż produkt dla klientów, ale potwierdźcie to z adminem Enterprise lub opiekunem konta ([Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview)). Rdzeń ma abstrakcję dostawcy, więc zmiana ścieżki nie dotyka UI.

**Zadania AI w kolejności wywołań:**

1. **Briefing** (raz na wersję MR): intencja z opisu MR i issue kontra to, co diff faktycznie robi; rozbieżności; mapa ryzyka plików; proponowana trasa (grupy hunków, kolejność, uzasadnienie); szacowany czas.
2. **Findingi per krok trasy:** etykieta Conventional Comments, blocking lub non-blocking, linie, uzasadnienie, pewność, szkic komentarza. Twardy limit liczby na krok i próg pewności, żeby nie zalać recenzenta.
3. **Strażnik testów:** osobne pytanie, gdy diff dotyka testów lub snapshotów. Czy asercje osłabiono, czy pojawił się special-casing wartości testowych.
4. **„Zapytaj o zaznaczenie”:** interaktywnie, w tej samej sesji co briefing, żeby kontekst MR został zachowany.
5. **Redakcja komentarza:** z roboczej notatki recenzenta powstaje konkretny komentarz w formacie Conventional Comments. Recenzent go edytuje i akceptuje.

**Koszt i cache.** Wynik JSON zawiera `total_cost_usd` (szacunek po stronie klienta), a `--max-budget-usd` ucina zbyt drogie przebiegi. Wyniki cache'ujemy per (`head_sha`, wersja promptu, model), więc ponowne otwarcie MR nic nie kosztuje.

**Ewaluacja.** Zestaw MR z celowo wstawionymi błędami z Waszego kodu MAUI. Każda zmiana promptu przechodzi ten zestaw z pomiarem precyzji i recall, zanim trafi do zespołu.

## UX guided review

Recenzent przechodzi pięć etapów: Skrzynka, Briefing, Trasa, Bramka, Publikacja. W każdym momencie widzi, co zostało, ile czasu minęło i co blokuje approve.

*[osadzony diagram: przepływ review · 5 etapów, 2 pętle]*

Trasa to główny widok, w którym recenzent spędza większość czasu. Bramka odsyła do niej, dopóki są otwarte uwagi blokujące, a nowy push autora wraca do skrzynki jako interdiff.

1. **Skrzynka.** MR „do mnie” z rozmiarem (LOC, pliki), ryzykiem, statusem pipeline'u i znacznikiem „nowa wersja od Twojego review”. Domyślne sortowanie: najdłużej czekające na mnie. Refy MR są fetchowane w tle, zanim ktoś kliknie.
2. **Briefing.** Jeden ekran: cel z opisu MR, co diff faktycznie zmienia, rozbieżności, mapa ryzyka plików, trasa z szacowanym czasem. Powyżej ok. 400 LOC narzędzie proponuje podział na przejścia.
3. **Trasa (główny widok).** Trzy kolumny:
   - lewa: kroki trasy, każdy z nazwą i powodem kolejności, plus pasek postępu; tryb „wszystkie pliki” jako alternatywa;
   - środek: diff side-by-side (unified przy wąskim oknie), diff słów, zwinięty niezmieniony kontekst, nagłówek hunka z nazwą symbolu, przeniesiony kod przygaszony z linkiem do źródła, istniejące wątki GitLaba inline;
   - prawa: kontekst bieżącego hunka, czyli dopasowane pozycje checklisty, diagnostyki SARIF z tych linii, findingi AI (akceptuj jako draft, odrzuć z powodem, pokaż dowód), wywołania zmienionej metody, powiązane testy.
4. **Bramka.** Przed decyzją: nierozwiązane uwagi blokujące, nieodwiedzone kroki, niepotwierdzone pozycje checklisty, status pipeline'u, zmiany w testach. Decyzja: Approve, Reviewed albo Requested changes, z notatką podsumowania (szkic od AI, edytowalny).
5. **Publikacja.** Jednym ruchem `bulk_publish` i opcjonalnie approve. Po nowym pushu autora MR wraca do skrzynki jako interdiff: tylko nowe zmiany, wątki przypięte do nowych linii, „viewed” zachowane dla niezmienionych plików.

**Komentarze.** Inline pod linią lub zakresem, z szablonami Conventional Comments (`issue`, `suggestion`, `question`, `nitpick` + `blocking` lub `non-blocking`). Propozycja kodu jako sugestia GitLaba. Wszystko jest draftem do momentu publikacji.

**Klawiatura przede wszystkim.**

| Klawisz | Akcja |
| --- | --- |
| `j` / `k` | następny / poprzedni hunk |
| `n` / `p` | następny / poprzedni krok trasy |
| `c` | komentarz na linii lub zaznaczeniu |
| `v` | oznacz plik jako przejrzany |
| `a` / `x` | akceptuj / odrzuć finding AI |
| `?` | zapytaj AI o zaznaczenie |
| `o` | otwórz w Riderze w tej linii |
| `Ctrl+K` | paleta poleceń |

**Higiena uwagi.** Licznik LOC i czasu sesji jest zawsze widoczny. Po ok. 60 minutach lub 400 LOC pojawia się delikatna sugestia przerwy, a stan zapisuje się do wznowienia w tym samym miejscu. Findingi AI nie przerywają pracy: kropka w gutterze i wpis w panelu, bez okien modalnych.

**Tryb nauki.** Dla nowych osób w projekcie AI tłumaczy, co robi dany fragment. Jest wyraźnie oddzielony od trybu review, żeby wyjaśnienia nie udawały oceny.

## Co jeszcze daje lokalne repo

Lokalne repo daje trzy rzeczy, których nie ma przeglądarka GitLaba: historię zmienianego kodu, uruchomienie obu wersji i pełną semantykę C#. Największy zwrot dają testy z MR uruchomione na starym kodzie, różnica ostrzeżeń między base a head i tryb self-review dla autora.

| Obszar | Funkcja | Co daje recenzentowi | Faza |
| --- | --- | --- | --- |
| Historia | Hotspoty: liczba zmian i commitów „fix” w zmienionych plikach z ostatnich 12 miesięcy | ryzyko w briefingu oparte na faktach, nie tylko na ścieżkach | 2 |
| Historia | Brakujące współzmiany: pliki, które zwykle zmieniają się razem, a w MR zmienił się jeden | „ScannerPage.xaml.cs zwykle zmienia się ze ScannerViewModel.cs, tu nie” | 3 |
| Historia | Blame zmienionych linii i CODEOWNERS | wiesz, kogo zapytać i kiedy poprosić o drugiego recenzenta | 2 |
| Wykonanie | Różnica ostrzeżeń: build i analizatory na base i head, porównanie SARIF | tylko ostrzeżenia wprowadzone przez ten MR, bez długu całego projektu | 2 |
| Wykonanie | Testy z MR na starym kodzie: nowe i zmienione testy uruchomione na base | test, który przechodzi bez zmiany, niczego nie sprawdza: dowód dla strażnika testów | 3 |
| Wykonanie | Pokrycie zmienionych linii z coverlet, nałożone na diff | margines pokazuje linie, których żaden test nie wykonał | 3 |
| Wykonanie | Mutacje tylko w zmianie: Stryker.NET z `--since:<base>` | przeżyte mutanty w nowym kodzie wskazują brakujące asercje | 3 |
| Wykonanie | Build worktree i deploy na emulator Androida jednym przyciskiem | dowód działania kamery i uprawnień bez przełączania gałęzi | 4 |
| Semantyka | Graf zależności z Roslyn na base i head, także poza MR | wpływ zmiany i podstawa kolejności trasy | 3 |
| Semantyka | Spójność wzorca: wyszukanie w repo wzorca, który MR zmienia | „Device → MainThread zmienione w 1 miejscu, zostały 4 inne” | 2 |
| Semantyka | Duplikaty: nowy kod porównany z istniejącymi helperami | wykrywa, że AI napisało od nowa coś, co już jest w projekcie | 3 |
| Semantyka | XAML ↔ ViewModel: `x:DataType` i bindingi zmapowane na właściwości | bindingi do zmienionych lub usuniętych właściwości | 3 |
| Tryby pracy | Self-review autora: ta sama trasa na lokalnych zmianach względem main, przed pushem | zmiany od Claude Code przechodzą review, zanim powstanie MR | 2 |
| Tryby pracy | Praca offline: diff, trasa i drafty bez sieci | drafty czekają w kolejce na synchronizację z GitLabem | 2 |
| Tryby pracy | Analizy deterministyczne tylko lokalnie | do Claude trafia wyłącznie to, co wysyła sesja Claude Code | 1 |

**Testy z MR na starym kodzie** zamieniają podejrzenie w dowód. Jeśli nowy test przechodzi na base, nie sprawdza zmiany, czyli dokładnie ten problem, który opisuje ImpossibleBench.

**Różnica SARIF** robi z analizatorów bramkę bez hałasu. Recenzent widzi tylko to, co wprowadził ten MR, nawet w projekcie z długiem ostrzeżeń.

**Self-review autora** przesuwa część review przed MR. Badanie SmartBear/Cisco wiązało adnotacje autora z mniejszą liczbą defektów, a przy pracy z Claude Code to naturalny krok przed pushem.

Koszt: build i testy na dwóch wersjach trwają. Narzędzie liczy je w tle po pobraniu refu, z cache po SHA, i buduje tylko projekty bibliotek oraz testów, bez pakowania aplikacji Android.

## Jak powstaje trasa

Trasę liczy deterministyczny algorytm na grafie zależności zmian, a Claude Code tylko nazywa kroki i uzasadnia kolejność. Ta sama wersja MR daje każdemu w zespole tę samą trasę, więc „krok 3” znaczy to samo dla wszystkich.

**Co mówią badania.**

- Najważniejsze jest grupowanie powiązanych części zmiany. Do tego wniosku doszli Baum i in. w teorii optymalnej kolejności czytania zmian ([ICSME 2017](https://www.merlin.uzh.ch/publication/show/15587)).
- Podział według relacji definicja–użycie działa w praktyce. ClusterChanges z Microsoftu znalazł ponad jedną nietrywialną część w prawie 42% z 1 000 zmian z Office 2013. 16 z 20 deweloperów uznało podział za poprawny ([podsumowanie](https://blog.acolyer.org/2015/06/26/helping-developers-help-themselves-automatic-decomposition-of-code-review-changes/)).
- Kolejność alfabetyczną za optymalną uważa tylko 10,2% z 1 355 ankietowanych deweloperów. 66% chce grupowania według zależności lub własnej kolejności ([ICSE 2026](https://conf.researchr.org/details/icse-2026/icse-2026-research-track/310/Breaking-the-Alphabet-Rethinking-File-Ordering-in-Code-Review)).
- Pliki wyżej na liście dostają więcej komentarzy, przy słabej, ale istotnej korelacji (9 475 review IntelliJ IDEA, [EASE 2023](https://ar5iv.labs.arxiv.org/html/2306.06956)). Ryzyko powinno więc trafiać na początek.
- Testy czytane przed kodem dały tyle samo defektów w produkcji i więcej w testach, ale mniej uwag o utrzymywalności (93 deweloperów, [ICSE 2019](https://2019.icse-conferences.org/details/icse-2019-Technical-Papers/29/Test-Driven-Code-Review-An-Empirical-Study)).
- Rozdzielenie splątanych zmian nie zmieniło liczby defektów ani czasu, ale zmniejszyło fałszywe alarmy z 6 do 1 (28 osób, [PeerJ CS](https://peerj.com/articles/cs-193/)).

**Algorytm.** W fazie 2 graf powstaje z Tree-sitter i reguł ścieżek, w fazie 3 z pełnego Roslyn w sidecarze .NET.

1. **Odsiej szum.** Pliki generowane (wzorce z `.review/config.yaml` i nagłówek `<auto-generated>`), czyste przeniesienia i zmiany samego formatowania trafiają do zwiniętego kroku „kosmetyka”.
2. **Potnij na jednostki.** Każdy hunk przypisujemy do symbolu: metody, właściwości, klasy albo elementu XAML. Kilka hunków jednej metody to jedna jednostka.
3. **Zbuduj graf.** Krawędzie silne: definicja → użycie, test → testowany symbol, XAML ↔ code-behind ↔ ViewModel (`x:Class`, `x:DataType`), rejestracja w DI → typ. Krawędzie słabe: ten sam plik, historyczne współzmiany.
4. **Znajdź wątki.** Spójne składowe grafu po krawędziach silnych, jak w ClusterChanges. Jeden wątek to jedna logiczna zmiana. Dwa niezależne wątki trafiają do briefingu jako sugestia podziału MR.
5. **Ułóż wątki.** Wątek, od którego zależą inne, idzie pierwszy. Przy remisie decyduje wyższe ryzyko. Wątki trywialne, np. same zmiany nazw, idą na koniec.
6. **Ułóż kroki w wątku.** Szablon warstw z konfiguracji: kontrakt → testy → logika → ViewModel → widok → kod platformowy → DI i konfiguracja. W warstwie definicje idą przed użyciami, a cykle zależności łączą się w jeden krok.
7. **Dopasuj do budżetu.** Czas kroku to LOC × mnożnik ryzyka × mnożnik nowości, czyli czy recenzent znał plik. Powyżej ok. 60 minut lub 400 LOC trasa dzieli się na przejścia, zawsze na granicy wątku.
8. **Dodaj kroki zamykające.** Przedostatni to zwinięta kosmetyka. Ostatni to „Całość”: testy na base i head, brakujące współzmiany i pytanie, czy intencja z opisu jest zrealizowana.
9. **Dopisz słowa.** Claude Code nazywa wątki i kroki oraz pisze „dlaczego teraz”. Może zaproponować inną kolejność, ale tylko jako sugestię do akceptacji.

**Od czego zacząć, na czym skończyć.** Start to najmniejszy element, który definiuje słownik zmiany: kontrakt, czyli interfejs, sygnatura albo DTO. Bez kontraktu startujemy od zmienionych testów, a bez testów od definicji, od której zależy najwięcej zmian w MR. Koniec to okablowanie (DI, manifest, csproj), kosmetyka i krok „Całość”, z którego prowadzi przejście do bramki.

**Dlaczego od definicji do użyć.** Briefing już daje widok z góry: intencję i to, co diff robi. Trasa może więc czytać od dołu. Przy wywołującym znasz już kontrakt wywoływanego i oceniasz tylko użycie, np. czy async void łapie anulowanie. To decyzja projektowa, nie wynik badań, które potwierdzają grupowanie, ale nie kierunek. Dlatego kierunek jest ustawieniem w `.review/config.yaml`: od definicji albo od punktu wejścia.

**Testy zaraz po kontrakcie.** Przy kodzie od AI ryzyko osłabionych testów jest największe, a uruchomienie testu na starym kodzie daje dowód. Koszt według ICSE 2019 to mniej uwag o utrzymywalności kodu produkcyjnego. Zespół może przesunąć testy za implementację w konfiguracji.

**Przykład: trasa dla !482.**

*[osadzony diagram: graf zależności !482 · 6 kroków trasy]*

Kontrakt nie zależy od żadnej innej zmiany, więc to start. Testy idą zaraz po nim jako specyfikacja logiki z kroku 3. MauiProgram tylko używa typów z kroków 3 i 4, więc zamyka trasę.

| Krok | Jednostki | Co zdecydowało |
| --- | --- | --- |
| 1 · Intencja i kontrakt | `ICameraFrameSource` (nowy parametr) | warstwa „kontrakt”; definicja używana w krokach 3 i 4 |
| 2 · Testy | `FrameAnalyzerTests` | krawędź test → `Analyze`; strażnik testów: usunięta asercja |
| 3 · Analiza klatek | `FrameAnalyzer.Analyze`, `Dispose` | definicja wywoływana przez `ScannerPage` w kroku 4 |
| 4 · Cykl życia strony | `ScannerPage.OnAppearing`, `OnDisappearing`, `OnFrameReady` | używa kontraktu i `Analyze`; reguła zespołu „zdarzenia” |
| 5 · UI i bindingi | `ScannerPage.xaml` | para XAML ↔ code-behind; warstwa „widok” |
| 6 · Konfiguracja i DI | `MauiProgram` (FrameAnalyzer jako transient) | warstwa „DI”; rejestruje typy z kroków 3 i 4 |
| Zwinięte | `Resource.designer.cs` | plik generowany |
| Całość | testy na base i head, współzmiany, intencja | krok zamykający przed bramką |

Graf jest spójny, więc !482 to jeden wątek. Gdyby MR zmieniał też np. ustawienia języka bez związku z kamerą, briefing pokazałby dwa wątki i zaproponował podział.

**Zmiany w trakcie.** Po nowym pushu trasa przelicza się tylko dla zmienionych jednostek, a odwiedzone kroki bez zmian zostają odhaczone. Recenzent może przeciągnąć krok albo skoczyć do pliku. Trasa zapamiętuje odwiedziny i nie przestawia kolejności w trakcie sesji.

## Wydajność i czytelność

UI nigdy nie czeka na sieć ani na AI. Wszystko, co da się policzyć lokalnie, liczy się lokalnie i poza wątkiem interfejsu. Poniższe budżety to cele projektowe do weryfikacji w spike'ach fazy 0.

| Operacja | Budżet |
| --- | --- |
| Start aplikacji do skrzynki (z cache) | < 1 s |
| Otwarcie MR z cache | < 300 ms |
| Przejście do następnego hunka | < 16 ms (jedna klatka) |
| Przewijanie pliku 10 000 linii | 60 fps |
| Wyniki AI | strumieniowo, bez blokowania widoku |

**Techniki:**

- Diff i podświetlanie składni w rdzeniu lub web workerze. Wyniki kluczowane SHA bloba: bloby są niezmienne, więc cache nie wymaga unieważniania.
- Wirtualizacja listy plików, kroków i samego diffa. Edytory montowane leniwie, tylko dla widocznych plików.
- Prefetch refów MR ze skrzynki w tle. Briefing AI może liczyć się po fetchu, zanim recenzent otworzy MR, z limitem kosztu.
- Pliki generowane (`*.Designer.cs`, `Resource.designer.cs`, lockfile'e, migracje) zwijane automatycznie według reguł z repo.

**Czytelność:**

- Font z wyraźnym rozróżnieniem 0/O i l/1, np. JetBrains Mono jak w Riderze. Interlinia ok. 1,5 i regulowany rozmiar.
- Kolory diffa także w wariancie przyjaznym daltonistom (niebieski/pomarańczowy). Motyw jasny i ciemny spójny z Riderem.
- Hierarchia: zmienione słowa mocniej niż zmienione linie, kontekst przygaszony. Wątki i findingi w osobnym paśmie, nie na kodzie.
- Tryby gęstości „komfort” i „kompakt”. Panel kontekstu chowany jednym klawiszem.

## Praca zespołowa

Wspólna jest konfiguracja (w repo) i stan review (w GitLabie). Lokalnie zostaje tylko stan sesji i cache, więc nie potrzeba własnego serwera.

```text
.review/
  config.yaml            # progi LOC i czasu, pliki generowane, kolejność warstw trasy
  checklists/maui.yaml   # pozycje + reguły dopasowania (regex lub zapytanie Tree-sitter)
  checklists/ai-code.yaml
  rules/risk.yaml        # ścieżki -> poziom ryzyka i wymagane checklisty
  prompts/briefing.md    # prompty wersjonowane jak kod
  prompts/findings.md
  eval/                  # MR z wstawionymi błędami do testów promptów
```

Przykładowa reguła, która podpina pytanie pod konkretną zmienioną linię:

```yaml
- id: event-unsubscribe
  match: { lang: csharp, regex: '\+=\s*[A-Za-z_]\w*\s*;' }
  scope: changed-lines
  ask: "Czy jest odpowiadające -= w OnDisappearing/OnNavigatedFrom albo słaba referencja?"
  severity: blocking
- id: android-permissions
  match: { path: 'Platforms/Android/**' }
  require_checklist: permissions
```

**Konfiguracja z gałęzi docelowej.** Narzędzie czyta `.review/` z gałęzi, do której MR jest kierowany, nie z gałęzi MR. Autor nie osłabi reguł we własnym MR, a konfiguracja AI nie pochodzi z niezaufanego kodu.

**Pętla uczenia.** Odrzucony finding wymaga powodu wybieranego z krótkiej listy. Raz na sprint zespół przegląda najczęstsze powody i błędy znalezione po merge, a poprawki trafiają jako MR do `.review/`.

**Dystrybucja.** Instalator z wewnętrznego kanału i auto-update przez updater Tauri. URL GitLaba i dostawca AI ustawiane per organizacja.

## Plan faz

Pięć faz, każda zamknięta bramką z mierzalnym kryterium. AI wchodzi dopiero w fazie 3, gdy viewer i zapis review są sprawdzone na prawdziwych MR.

*[osadzony diagram: roadmapa · 5 faz, 4 bramki]*

Każda faza zostawia działające narzędzie. Od fazy 1 zespół szybciej czyta MR, od fazy 2 robi w nim całe review, a AI dokłada się do zmierzonego punktu odniesienia.

## Ryzyka, metryki sukcesu, otwarte pytania

Największe ryzyka to szum AI i wykonanie niezaufanej konfiguracji z gałęzi MR. Oba adresujemy w architekturze, nie w instrukcji obsługi.

| Ryzyko | Skutek | Mitygacja |
| --- | --- | --- |
| Szum AI | recenzenci ignorują panel | limit findingów na krok, próg pewności, odrzucenie z powodem, zestaw ewaluacyjny |
| Hooki i MCP z gałęzi MR | obcy kod wykonany u recenzenta | konfiguracja z gałęzi docelowej, `--bare`, tylko narzędzia odczytu |
| Błędne kotwiczenie komentarzy | komentarz w złej linii | drafty względem aktualnych `diff_refs`, ostrzeżenie przy nowej wersji w trakcie |
| Ścieżka dostępu do Claude i polityka danych | blokada wdrożenia | ścieżka wybrana (seaty Claude Code); zgodność z umową i retencję danych potwierdzić z adminem w fazie 0 |
| Stack Rust + TS w zespole .NET | wiedza u jednej osoby | Rust tylko dla Gita, diffa i IPC; logika domenowa w TS i .NET; testy kontraktów IPC |
| Rubber-stamping mimo narzędzia | fałszywe poczucie bezpieczeństwa | bramka przed approve, widoczny licznik LOC i czasu |

**Metryki sukcesu** (bazę mierzymy w fazie 0 z API GitLaba):

- mediana czasu do pierwszego review;
- liczba rund na MR;
- błędy znalezione po merge, z issue trackera;
- odsetek zaakceptowanych findingów AI;
- mediana rozmiaru MR (narzędzie powinno ją obniżać, nagradzając małe zmiany);
- krótka ankieta satysfakcji recenzentów co sprint.

**Otwarte pytania:**

- [ ] Czy organizacja ma dostęp do API (Console lub chmura), czy tylko seaty Claude Code? Czy użycie seatów przez narzędzie wewnętrzne jest zgodne z Waszą umową? Odpowiedź: dostęp tylko do claude code
- [ ] GitLab.com czy self-managed, i w jakiej wersji?: odpowiedz:self-managed - nie pamiętam wersji
- [ ] Windows, macOS czy oba (przy MAUI zwykle oba, bo iOS wymaga Maca)? Odpowiedź:Na ten momement windows
- [ ] Czy CI może publikować SARIF jako artefakt? Wtedy narzędzie nie musi budować lokalnie. odpowiedź: nie wiem
- [ ] Jakie są wymogi retencji danych przy wysyłaniu kodu do Claude? Odpowiedz: nie wiem

## Źródła

- [Claude Code: tryb headless (`claude -p`)](https://code.claude.com/docs/en/headless)
- [Claude Agent SDK: overview](https://code.claude.com/docs/en/agent-sdk/overview)
- [GitLab: Draft Notes API](https://docs.gitlab.com/api/draft_notes/)
- [GitLab: Merge requests API](https://docs.gitlab.com/api/merge_requests/)
- Progi 400 LOC i 60 minut oraz ryzyko osłabiania testów przez AI: raport o code review z wcześniejszej części tej rozmowy.
