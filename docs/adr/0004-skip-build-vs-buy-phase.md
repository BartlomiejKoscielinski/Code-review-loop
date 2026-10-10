# 0004. Rezygnujemy z fazy 0 (testy gotowych rozwiązań)

- Status: Zaakceptowany (@Bartek, 2026-10-10, zgoda na start implementacji w sesji planowania)
- Data: 2026-10-10
- Decydenci: @Bartek

## Kontekst

`docs/input/plan.md` zakłada fazę 0 z tygodniem testów istniejących opcji (GitLab Duo, CodeRabbit, Claude Code w CI, Rider) i pomiarem metryk bazowych. Plan wspomina pięć faz, ale ich zakres był tylko na diagramie, którego nie ma w repo.

10.10.2026 @Bartek zdecydował w sesji planowania, że nikt nie będzie testował innych rozwiązań, i zlecił plan czterech faz. To odstępstwo od `docs/input/plan.md`, więc zapisujemy je jako decyzję.

## Rozważane opcje

1. Zostawić fazę 0 zgodnie z `docs/input/plan.md`.
2. Pominąć ocenę gotowych rozwiązań. Techniczne niewiadome rozstrzygać spike'ami na starcie faz, a pomiar bazy przenieść do fazy 1.

## Decyzja

Wybieramy opcję 2.

- Plan ma cztery fazy: 1 czytnik, 2 całe review, 3 AI i semantyka, 4 dopracowanie (`docs/plan/README.md`).
- Spike'i techniczne (S1, S2, P2-30, P3-01, S4) wykonujemy przed zadaniami, które od nich zależą. To eksperymenty na naszym stosie, nie ocena innych produktów.
- Metryki bazowe liczy zadanie P1-30 z API GitLaba, więc porównanie „przed i po” zostaje.

## Konsekwencje

- Implementacja zaczyna się od razu od fazy 1.
- Nie wiemy, czy gotowe narzędzie nie pokryłoby części potrzeb. Akceptujemy to ryzyko.
- `docs/input/plan.md` nadal wspomina fazę 0 i pięć faz. Aktualizacja materiału wejściowego jest opcjonalna (`docs/plan/decisions.md`).

## Źródła

- `docs/input/plan.md`, sekcje „Build vs buy” i „Plan faz”.
- Decyzja @Bartka w sesji planowania z 2026-10-10.
