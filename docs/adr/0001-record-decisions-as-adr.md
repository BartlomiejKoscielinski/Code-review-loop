# 0001. Decyzje zapisujemy jako ADR

- Status: Zaakceptowany
- Data: 2026-10-10
- Decydenci: zespół Lupy

## Kontekst

Lupę buduje kilka osób z pomocą Claude Code. Agent zaczyna każdą sesję bez pamięci poprzednich rozmów, a decyzje z czatu i review merge requestów giną. Bez trwałego zapisu agent i ludzie wracają do rozstrzygniętych kwestii albo łamią je nieświadomie.

## Rozważane opcje

1. ADR w repo, jeden plik Markdown na decyzję, format Michaela Nygarda.
2. Decyzje w `CLAUDE.md`. Odrzucone: plik ładuje się w każdej sesji i musi mieć mniej niż 200 linii.
3. Wiki w GitLabie. Odrzucone: agent nie widzi jej bez dodatkowego serwera MCP, a zmiany nie przechodzą review razem z kodem.

## Decyzja

Każdą decyzję architektoniczną lub technologiczną zapisujemy jako ADR w `docs/adr/`:

- plik `NNNN-krotki-tytul-po-angielsku.md` z czterocyfrowym numerem, treść po polsku;
- nowy ADR tworzy skill `/adr` z szablonu `0000-template.md`;
- statusy: Proponowany, Zaakceptowany, Odrzucony, Zastąpiony przez NNNN;
- zaakceptowanego ADR nie edytujemy, poza zmianą statusu i odnośnikiem do następcy. Zmiana decyzji oznacza nowy ADR.

## Konsekwencje

- Agent i nowi członkowie zespołu znajdują uzasadnienie decyzji w repo, obok kodu.
- Decyzja przechodzi review w tym samym merge requeście co zmiana, która jej wymaga.
- Każda decyzja kosztuje kilka minut pisania. Drobne wybory implementacyjne nie wymagają ADR.

## Źródła

- Michael Nygard, „Documenting Architecture Decisions”, 2011: https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions
- Martin Fowler, „Architecture Decision Record”: https://martinfowler.com/bliki/ArchitectureDecisionRecord.html
