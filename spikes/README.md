# spikes: eksperymenty przed decyzją

Spike to krótki, ograniczony czasowo eksperyment, który odpowiada na jedno pytanie techniczne przed decyzją w ADR. Przykład: czy `claude -p` uruchomiony z Tauri da się odizolować do trybu tylko do odczytu na Windows.

## Zasady

- Każdy spike ma katalog `spikes/<nazwa-po-angielsku>/` i `README.md` z hipotezą, metodą, kryterium sukcesu i limitem czasu. Zakłada go skill `/spike`.
- Kod spike'a nie trafia do aplikacji. Jeśli się sprawdzi, implementację pisze się od nowa według planu.
- Po zakończeniu `README.md` zawiera wynik (potwierdzona, obalona, nierozstrzygnięta), dowody (logi, wyniki komend) i rekomendację. Rekomendacja prowadzi do ADR.
- Po przekroczeniu limitu czasu spike się zamyka z wynikiem „nierozstrzygnięta” i opisem, czego zabrakło.
- Spike nie zawiera sekretów: tokeny tylko ze zmiennych środowiskowych, nigdy w plikach.
