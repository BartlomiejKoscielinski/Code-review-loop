# 0003. Jeden model celu review dla czterech źródeł zmian

- Status: Zaakceptowany (@Bartek, 2026-10-10, zgoda na start implementacji w sesji planowania)
- Data: 2026-10-10
- Decydenci: @Bartek

## Kontekst

Lupa ma prowadzić przez zmiany z czterech źródeł: lokalnego diffa, konkretnego commita, brancha lub zakresu oraz merge requestu innej osoby. `docs/input/plan.md` szczegółowo opisuje tylko MR. Branch i commit mają po jednej linii („Integracja z Git i GitLab”), a lokalne zmiany pojawiają się jako „self-review autora” w fazie 2. Makieta `Inbox` pokazuje tylko MR.

Jeśli każde źródło dostanie własny przepływ, trasa, bramka i widok diffa powstaną w kilku wersjach. Dodatkowo zmiany niezacommitowane nie mają SHA, a cache opiera się na identyfikatorach obiektów Gita.

## Rozważane opcje

1. Osobne przepływy: MR jako główny, pozostałe źródła jako uproszczony podgląd.
2. Jeden model `ReviewTarget` (base, head, opcjonalnie start i poprzednia wersja) wypełniany przez cztery adaptery. Różnią się tylko pozyskaniem refów i miejscem, do którego trafiają komentarze.

## Decyzja

Używamy opcji 2.

| Źródło | base | head | Komentarze trafiają do |
| --- | --- | --- | --- |
| `WorkingTree` | `merge-base(HEAD, gałąź docelowa)` (domyślnie) albo `HEAD` | migawka working tree (staged, unstaged, nieśledzone) | notatek lokalnych |
| `Commit` | rodzic (przy merge commicie wybrany, domyślnie pierwszy) | commit | notatek lokalnych |
| `Range` | `A` albo `merge-base(target, source)` dla brancha | `B` albo `source` | notatek lokalnych |
| `MergeRequest` | `diff_refs.base_sha` (+ `start_sha`) | `diff_refs.head_sha` | draftów w GitLabie |

Ustalenia wspólne:

- `configRef` wskazuje commit, z którego czytamy `.review/`: zawsze gałąź docelowa, nigdy gałąź zmiany (zgodnie z `docs/input/plan.md`).
- `previous` (poprzedni head i base) włącza interdiff. Dla MR to wersja z ostatniego review, dla celów lokalnych ostatnio przejrzana migawka.
- Pliki working tree dostają identyfikator treści liczony jak `git hash-object`. Wtedy cache diffów działa tak samo jak dla blobów.
- Trasa, checklisty, bramka i widok diffa są wspólne. Decyzja `Approve` i publikacja istnieją tylko dla MR. Pozostałe cele kończą się notatkami z eksportem do Markdown.
- Wejście do wszystkich źródeł zapewnia ekran „Otwórz” i paleta poleceń (zadanie P1-16). Makieta tego ekranu nie istnieje, opis jest w planie.

## Konsekwencje

- Self-review lokalnych zmian i review commita dostają od razu tę samą trasę i bramkę co MR, bez dodatkowej pracy w fazie 2.
- Każda funkcja musi działać bez metadanych MR (opis, wątki, pipeline). Ekrany ukrywają wtedy odpowiednie sekcje.
- Migawka working tree może zmienić się w trakcie review. Odświeżenie zachowuje „viewed” dla plików o niezmienionej treści.
- Kolejne kroki: P1-12, P1-13, P1-15, P1-16 i P2-26 w `docs/plan/`.

## Źródła

- `docs/input/plan.md`, sekcje „Integracja z Git i GitLab” i „Co jeszcze daje lokalne repo”.
- `diff_refs` w MR API: https://docs.gitlab.com/api/merge_requests/
- Ref `refs/merge-requests/<iid>/head` (usuwany 14 dni po zamknięciu lub merge'u): https://docs.gitlab.com/user/project/merge_requests/merge_request_troubleshooting/
- `git hash-object`: https://git-scm.com/docs/git-hash-object
