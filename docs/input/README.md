# docs/input: materiały wejściowe

Ten katalog zawiera materiały od zespołu, na których opiera się planowanie:

- `plan.md`: opis produktu, zakres i wymagania;
- `mocks/`: makiety interfejsu (obrazy lub pliki eksportowane z narzędzia do makiet).

## Zasady

- **Agent tylko czyta ten katalog.** Zapis blokują hook `.claude/hooks/protect-inputs.mjs` i reguła `Edit(/docs/input/**)` w `.claude/settings.json`. Pliki dodaje i zmienia człowiek.
- Materiały są źródłem prawdy dla `docs/plan/`. Jeśli plan odbiega od materiałów, rozbieżność trafia do ADR w `docs/adr/` albo do pytania do zespołu, a nie do cichej zmiany planu.
- Materiały mogą zawierać tylko dane przykładowe. Nie wklejaj tu tokenów, adresów wewnętrznych serwerów ani prawdziwych danych z merge requestów.
