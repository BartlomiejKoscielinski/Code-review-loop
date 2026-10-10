# Faza 4: dopracowanie, nauka i skala

**Cel:** narzędzie, które zespół rozwija sam: pętla uczenia z odrzuconych hipotez, metryki sukcesu wobec bazy, tryb nauki dla nowych osób, deploy MR na emulator jednym przyciskiem, audyt dostępności i bezpieczeństwa przed wersją 1.0.

## Kamienie milowe

| Kamień | Zadania | Po nim działa |
| --- | --- | --- |
| 4.0 Wykonanie | P4-01 | build i deploy MR na emulator Androida |
| 4.1 Nauka i pomiar | P4-02 … P4-05 | tryb nauki, pętla uczenia, metryki, narzędzia dla autorów reguł |
| 4.2 Jakość wydania | P4-06 … P4-09 | OAuth, wydajność na dużych MR, audyt dostępności, przegląd bezpieczeństwa |

## Zadania

#### P4-01 · Deploy MR na emulator Androida

- **Zależy od:** P2-32
- **Zakres:** przycisk „Uruchom na emulatorze” dla head. Build aplikacji MAUI dla Androida w worktree w strefie D według ADR z P2-30, instalacja na wybranym emulatorze, log z postępem i anulowaniem. Komendy buildu i instalacji zadanie ustala w dokumentacji .NET MAUI i Android SDK i podaje z linkami.
- **Kryteria akceptacji:** test na atrapie procesów: kolejność kroków i anulowanie; ręczna weryfikacja na repo MAUI zespołu z wynikiem w opisie zmiany; środowisko procesu bez tokenu (test jak w P2-32).
- **Rozmiar:** ~300 LOC.
- **Przegląd:** security.

#### P4-02 · Tryb nauki

- **Zależy od:** P3-09
- **Zakres:** osobny tryb dla nowych osób: AI tłumaczy, co robi zaznaczony fragment i jak łączy się z resztą. Inny kolor panelu i etykieta „Wyjaśnienie, nie ocena”. Wyjaśnienia nigdy nie stają się draftami ani hipotezami.
- **Kryteria akceptacji:** testy komponentów: w trybie nauki brak przycisku „Akceptuj jako draft”; przełączenie trybu nie miesza historii wyjaśnień z hipotezami.
- **Rozmiar:** ~250 LOC.

#### P4-03 · Pętla uczenia

- **Zależy od:** P3-07
- **Zakres:** raport sprintu:
  - najczęstsze powody odrzuceń hipotez per reguła i prompt;
  - błędy znalezione po merge (issues z etykietą błędu, które wskazują MR) z informacją, czy Lupa coś zgłosiła;
  - propozycje zmian do `.review/` jako tekst do MR w repo MAUI. Lupa nie commituje sama.
- **Kryteria akceptacji:** testy na danych z atrapy: agregacja powodów, powiązanie issue z MR; eksport bez treści kodu.
- **Rozmiar:** ~300 LOC.

#### P4-04 · Metryki sukcesu

- **Zależy od:** P1-30, P4-03
- **Zakres:** ekran metryk z `docs/input/plan.md` wobec bazy z P1-30:
  - mediana czasu do pierwszego review;
  - liczba rund;
  - błędy po merge;
  - odsetek zaakceptowanych hipotez;
  - mediana rozmiaru MR.

  Link do krótkiej ankiety sprintu (adres w ustawieniach). Dane lokalne, eksport do Markdown.
- **Kryteria akceptacji:** testy obliczeń na zestawie z atrapy; wykres trendu per sprint zgodny z danymi testowymi.
- **Rozmiar:** ~350 LOC.

#### P4-05 · Narzędzia dla autorów reguł

- **Zależy od:** P2-06
- **Zakres:** `lupa-rules-check`: walidator `.review/` do uruchamiania w CI repo MAUI (kod wyjścia, komunikaty z linią). Podgląd dopasowań reguły na 50 ostatnich MR („ta reguła zadziałałaby 12 razy”).
- **Kryteria akceptacji:** testy: niepoprawny plik daje kod ≠ 0 i komunikat z linią; podgląd dopasowań na repo testowym zgodny z P2-06.
- **Rozmiar:** ~250 LOC.

#### P4-06 · OAuth z PKCE

- **Zależy od:** P1-14, odpowiedź admina GitLaba (`open-questions.md`)
- **Zakres:** logowanie OAuth z PKCE jako alternatywa dla PAT, jeśli admin zarejestruje aplikację na instancji. Odświeżanie tokenu, zapis w Credential Manager, wylogowanie.
- **Kryteria akceptacji:** testy na atrapie serwera OAuth: pełny przepływ, odświeżenie, odrzucenie złego `state`; token nie trafia do logów ani argumentów procesu.
- **Rozmiar:** ~350 LOC.
- **Przegląd:** security.

#### P4-07 · Audyt dostępności

- **Zależy od:** faza 2
- **Zakres:** przegląd WCAG 2.1 AA dla wszystkich ekranów (klawiatura, fokus, czytnik ekranu dla listy kroków i paneli, kontrast w obu motywach i paletach), poprawki.
- **Kryteria akceptacji:** automatyczny test dostępności w Playwright dla każdego ekranu bez naruszeń poziomu „serious” i „critical”; ręczny przegląd z czytnikiem ekranu opisany w zmianie.
- **Rozmiar:** zależy od wyniku audytu, dzielone na zmiany do ok. 400 LOC.

#### P4-08 · Wydajność na dużych MR

- **Zależy od:** P1-28
- **Zakres:** profilowanie na największych MR z repo MAUI (powyżej 3 000 LOC i 150 plików), poprawki wąskich gardeł, tryb przewijania całego MR w jednym widoku z wirtualizacją między plikami, jeśli pomiary pokażą potrzebę.
- **Kryteria akceptacji:** budżety z `docs/input/plan.md` spełnione dla największego MR zespołu (`npm run bench` z nowym scenariuszem).
- **Rozmiar:** zależy od profilu, dzielone na zmiany do ok. 400 LOC.

#### P4-09 · Przegląd bezpieczeństwa przed 1.0

- **Zależy od:** wszystkie zadania security
- **Zakres:** pełny przegląd `security-reviewer` całego produktu. Zestaw wrogich MR w `lupa-testkit` uruchamiany w CI:
  - prompt injection w opisie, komentarzu i kodzie;
  - path traversal w nazwach plików;
  - symlinki w repo;
  - wrogie `.csproj` i `.claude/`;
  - ogromne pliki i linie;
  - znaki RTL.

  Aktualizacja modelu zagrożeń w `architecture.md`.
- **Kryteria akceptacji:** zestaw wrogich MR przechodzi w CI; brak otwartych znalezisk o wadze średniej i wyższej.
- **Rozmiar:** testy + poprawki w zmianach do ok. 400 LOC.
- **Przegląd:** security.

## Bramka wyjścia z fazy 4 (wersja 1.0)

1. Metryki z P4-04 za ostatnie 2 sprinty: mediana czasu do pierwszego review i liczba błędów po merge lepsze niż baza z P1-30. Docelowe wartości zatwierdza zespół przed startem fazy.
2. Ankieta sprintu: średnia satysfakcja recenzentów co najmniej 4 na 5.
3. P4-07 i P4-09 zamknięte bez otwartych znalezisk.
4. Instalator, auto-update i instrukcja dla nowej osoby w zespole działają na czystym Windows.
