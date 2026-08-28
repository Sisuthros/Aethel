# NIGHT-REPORT — Prisma yö 28.–29.8.2026 💎

## Kohde

**Aethel NG6: runtime authorization for effect operations** — effect-operaation
tarkistus ajonaikana `sound_eval.rs`:ssä. Valittu backlog-kohde **b**.

## Mitä tehtiin

- Rakennettiin `aethel-check` → `aethel-effects::EffectRegistry` reitti:
  semantic checkerin keräämistä signatureista syntyy runtime-rekisteri
  (`effect_registry.rs`).
- Lisättiin `aethel-interpreter/src/policy.rs`: `PolicyAuthorizer` joka
  vertaa `Verified<T, P>`-arvon policya operaation deklaroituun policyyn.
  Fail-closed: tuntematon operaatio, policy-mismatch ja puuttuva Verified
  kaikki tuottavat runtime-violationin.
- `sound_eval.rs` integroi authorizerin: `MethodCall` ja `CommitOnce`
  kulkevat `record_effect`in kautta, joka kysyy autorisaatiota ennen kuin
  laskee efektin onnistuneeksi.
- `aethel-cli run` ottaa nyt effect-rekisterin check-päästä ja antaa sen
  evaluatorille (`Evaluator::with_effect_registry`).
- Lisättiin `aethel-effects`in rekisteriin case-insensitive +
  operaatio-hint -haku, jotta lähdekonnin muuttujanimi (`payment_gateway`,
  `audit_service`) yhdistyy deklaroituun effect-tyyppiin (`PaymentGateway`,
  `AuditService`).

## Testit

| Portti | Tulos |
|---|---|
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace -- -D warnings` | 0 virhettä |
| `cargo test --workspace` | **56 passed, 0 failed** |
| `bash gate.sh` | **26/26 breaker**, 0 known-gap, emit-ir deterministic, examples ok |

Integration-testit `aethel-cli`ille (`run`, `run --trace`, `full_pipeline`) kaikki
vihreitä NG6:n myötä.

## Branch

`night/2026-08-28-ng6-runtime-auth` — commit `e60dbc8`

Muokattu / uusi:
- `crates/aethel-check/src/sound_checker/mod.rs`
- `crates/aethel-check/src/sound_checker/semantic/mod.rs`
- `crates/aethel-check/src/sound_checker/effect_registry.rs` (uusi)
- `crates/aethel-effects/src/registry.rs`
- `crates/aethel-interpreter/src/lib.rs`
- `crates/aethel-interpreter/src/policy.rs` (uusi)
- `crates/aethel-interpreter/src/sound_eval.rs`
- `crates/aethel-cli/Cargo.toml`
- `crates/aethel-cli/src/main.rs`
- `Cargo.lock`

**Ei pushattu** — push vaatii Villeä.

## Rehellisyys: mitä NG6 NYT on eikä ole

NYT:
- Jokainen `aethel run`:lla ajettu effect-operaatio tarkistetaan
  deklaroitua policya vasten.
- Policy-mismatch, tuntematon operaatio ja puuttuva Verified estetään
  ajonaikana.
- Staattinen (compile-time) ja ajonaikainen tarkistus toimivat
  yhdessä defencenä syvyyteen.

EI VIELÄ:
- `uses`-lausekkeen ajonaikainen valvonta: runtime sallii operaation,
  jos rekisterissä on yksikäsitteinen match; ei tarkista, että funktio
  on `uses`-listannut effectin (se tehdään jo compile-time -puolella).
- Ei capability-based-isolaatiota eikä hiekkalaatikkorajoituksia (NG7
  yhä avoin).

## Mikä jäi kesken / seuraava askel

1. Cyprus `/mini-audit` iCal-latauslinkki (backlog c) — `toICal` valmis,
   nopea (~30 min).
2. Aethel live-CLI-integraatio Cyprus-complianceen (backlog a) — API-route
   joka ajaa `aethel-cli check` ja kirjaa checksumin.
3. NG4 Wasm-sandboxin jatko: IR → Wasm -käännösprototyyppi.

— Prisma 💎
