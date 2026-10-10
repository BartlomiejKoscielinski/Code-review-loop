# 0007. Izolacja procesów Gita przy migawce lokalnych zmian

- Status: Zaakceptowany (decyzja techniczna delegowana przez @Bartka w sesji 2026-10-10, `docs/plan/decisions.md`)
- Data: 2026-10-10
- Decydenci: @Bartek

## Kontekst

ADR 0006 zakłada, że migawka lokalnych zmian nie zapisuje nic do repo dewelopera, a jedyny kod, jaki uruchamia, to filtry i fsmonitor z jego własnej konfiguracji. Przegląd `security-reviewer` po P1-07 pokazał, że tak nie jest:

1. `git add -A` uruchamia hook `post-index-change` także dla indeksu z `GIT_INDEX_FILE`. Przy względnym `core.hooksPath` (np. `.githooks`, husky) hook pochodzi z wybranej gałęzi, a odświeżanie po powrocie fokusu uruchamiałoby go bez żadnej akcji dewelopera.
2. Przy split index Git zapisuje i usuwa pliki `sharedindex.*` w `.git` dewelopera.
3. Fsmonitor może uruchomić hook albo demona z gniazdem w `.git`.
4. W partial clone wykrywanie zmian nazw i `cat-file` dociągają brakujące obiekty przez sieć.
5. Odziedziczone zmienne `GIT_*` (np. `GIT_ATTR_SOURCE`, `GIT_TRACE*`, `GIT_REDIRECT_*`) zmieniają zachowanie i wyjście Gita.
6. Na Windows `canonicalize` nie działa dla nieistniejącej ścieżki, więc sprawdzenie „cache poza repo” przy pierwszym uruchomieniu nic nie sprawdzało.
7. Wspólny plik indeksu w cache pozwalał dwóm równoległym migawkom nadpisać sobie dane.

Do tego test z CI ujawnił efekt „racy git”: kopia indeksu z nowszym czasem modyfikacji sprawia, że Git ufa nieaktualnym danym `stat` i nie zauważa zmiany pliku o tym samym rozmiarze, zapisanej w tej samej sekundzie co commit.

## Rozważane opcje

1. Opisać te efekty w dokumentacji i ich nie blokować.
2. Wymusić bezpieczne ustawienia dla każdego wywołania Gita w migawce.

## Decyzja

Wybieramy opcję 2. Uzupełnia ona ADR 0006 i nie zmienia jego mechanizmu.

- Każde wywołanie Gita w `WorkTree` dostaje:
  - `-c core.hooksPath=<pusty katalog w cache>`;
  - `-c core.fsmonitor=false`;
  - `-c core.splitIndex=false`.
- Usuwamy wszystkie odziedziczone zmienne `GIT_*` poza `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM` i `GIT_CONFIG_NOSYSTEM`, które tylko wybierają pliki konfiguracji. Ustawiamy `GIT_TERMINAL_PROMPT=0` i `GIT_NO_LAZY_FETCH=1`.
- Katalog cache jest najpierw tworzony, a potem porównywany po kanonikalizacji z korzeniem working tree i z `--git-common-dir`. Katalog założony w złym miejscu jest od razu usuwany.
- Każda migawka używa własnego pliku indeksu, a migawki jednego `WorkTree` idą po kolei (mutex).
- Kopia indeksu dostaje czas modyfikacji oryginału, żeby wykrywanie „racy” działało jak w Gicie dewelopera.
- `WorkTree` służy wyłącznie do repo dewelopera. Zmiany w MR liczy `Repo::changed_files` z samych obiektów, bez `git add` (zadanie P2-31).

## Konsekwencje

- Testy w `crates/lupa-git/tests/worktree_isolation.rs` sprawdzają:
  - hooki się nie uruchamiają, także przy względnym `core.hooksPath`;
  - `.git` zostaje bajt w bajt bez zmian, także przy split index;
  - cache w `.git` jest odrzucany;
  - równoległe migawki dają ten sam wynik.

  Wyłączenie ochrony hooków albo split index sprawia, że testy failują. Test „racy” (`worktree_racy.rs`) failował w 1 na 3 przebiegi przed poprawką, a po niej przeszedł 6 razy po 40 rund.
- Bez fsmonitora `git add -A` skanuje całe drzewo. Pomiar na Windows to zadanie P1-13 (budżet 300 ms dla 5000 plików).
- W partial clone migawka może zakończyć się błędem brakującego obiektu zamiast dociągnąć go z sieci. To świadoma decyzja.
- Zostają, jak w ADR 0006, zapis obiektów LFS do `.git/lfs/objects` i ewentualne odświeżenie czasu modyfikacji obiektów w repo.
- Limity czasu i anulowanie procesów Gita dochodzą razem ze wzorcem zadań w tle (P1-15).

## Źródła

- Hooki, w tym `post-index-change`: https://git-scm.com/docs/githooks
- `core.hooksPath`, `core.fsmonitor`, `core.splitIndex`: https://git-scm.com/docs/git-config
- Zmienne środowiskowe Gita, w tym `GIT_NO_LAZY_FETCH`: https://git-scm.com/docs/git#_environment_variables
- Racy git: https://git-scm.com/docs/racy-git
- Przegląd `security-reviewer` P1-07 (2026-10-10).
