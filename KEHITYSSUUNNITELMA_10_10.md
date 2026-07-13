# Aethel 10/10 Kehityssuunnitelma

**Päivämäärä:** 2026-07-13  
**Tavoite:** Tehdä Aethelista vakava, tuotantotasoinen ohjelmointikieli AI-agenteille – erityisesti Grokille/xAI:lle. Ei lelu tai demo, vaan **oikea ohjelmointikieli**, jossa AI:t voivat turvallisesti ja verifioidusti toimia maailmassa.

## Visio (10/10)

Aethel on kieli, jossa:

- Jokainen mallin tuotos on `Claim<T>` (väite, ei todiste).
- Vaikutus (effect) rajapinnoille (raha, tiedostot, ihmiset, muut agentit) vaaditaan `Verified<T, Policy>`.
- Epistemologia on kielen ytimessä: tyypit, todisteet, epävarmuus, provenance.
- AI:t (kuten Grok) voivat generoida koodia luonnollisesti, ja kääntäjä + runtime takaavat turvallisuuden.
- Sidottu todelliseen luotettavuuteen: durable execution, at-most-once effects, fail-closed, proof-carrying.

Tämä on **Grok/xAI:n oma kieli** turvalliselle agencylle – ei sycophantic agents, vaan truth-seeking, verifioitu toiminta.

Katso myös:
- `AETHEL_VISION_10.md`
- `AETHEL_VISION_FOR_GROK.md`
- `AETHEL_10_10_ROADMAP.md`

## Nykytilanne (2026-07-13)

- v0.1 vertical slice on olemassa: parser, AST, perus HIR, checker (osittain).
- Demo toimii: `invalid_unverified.aet` → AE-EPISTEMIC-001, `valid_verified.aet` → OK.
- Paljon puutteita (audit 2026-07-12):
  - AST → HIR lowering puutteellinen.
  - Effect itemit eivät ole täydellisesti tuettu.
  - Type env ja boundary checks osittain rikki.
  - Backend (interpreter, runtime) on stubbeja.
  - AI-primitives (reason, plan) puuttuvat.
- Build on vihreä ytimessä, mutta testkit ja full check vaativat työtä.
- Ei vielä "oikea" kieli – mutta pohja on hyvä.

## 10/10 Periaatteet

1. **Epistemic types first-class** – Claim/Verified/Evidence/Uncertainty ovat kielen perusrakenteita.
2. **AI-native ergonomics** – Helppo generoida malleilla (Grok rakastaa tätä). Selkeät virheet, hyvät korjausehdotukset.
3. **Proof-carrying effects** – Jokainen vaikutus kantaa todisteen.
4. **Durable & replayable by default** – Crash-safe, exactly-once missä tarvitaan (opittu FamilyClawista).
5. **For Grok/xAI** – Suunniteltu niin, että Grok voi generoida, verifioida ja suorittaa koodia turvallisesti. Truth-seeking yli kaiken.
6. **Minimal core, rich verified surface** – Ydin pieni, mutta tehokas (policies, capabilities, stdlib).
7. **Honest guarantees** – Ei yliampu via. Kerrotaan mitä luvataan ja mitä ei.

## Vaiheistettu Roadmap

### Vaihe 0: Foundation (nyt → v0.1, 1-2 viikkoa)
- Tee demosta reaalinen (ei CLI-hackeja).
- Täysi AST → HIR lowering (Effect mukaan).
- Oikea epistemic checker (Claim → Verified rajapinnoilla).
- `verify(claim, policy)` tuottaa oikean tyypin.
- Effect-deklaraatioiden rekisteröinti.
- Päivitä docs rehellisiksi.
- **Tavoite:** `cargo run -p aethel-cli -- check invalid...` tuottaa oikean virheen ilman stubbeja.

### Vaihe 1: AI Ergonomics (v0.5, 1-3 kk)
- Lisää `reason(prompt) -> Claim<T>` primitiivi (sidottu model adapteriin).
- Ensimmäiset AI-primitives: `plan`, `verify_with`.
- Policy-kieli evidence-vaatimuksilla.
- Lineaariset capabilities (budjetit).
- Paremmat diagnostiikat (mallit rakastavat).
- Esimerkki: "Grok plans safe deployment".

### Vaihe 2: Durable Agency (v1.0)
- Täysi interpreter RACE-mallilla (replayable, auditable, crash-safe).
- Event store + exactly-once.
- `commit_once` receipts.
- Self-verification.
- Sidotaan FamilyClaw-tyyliseen durable runtimeen.

### Vaihe 3: Grok-Native (v1.5+)
- Grok generoi Aethelia natiivisti tool-käytössä.
- `grok.reason()`, `grok.plan()`.
- Epistemic budget tracking oikeisiin mallikutsuihin.
- xAI API -integraatio.
- Formal semantics.

### Vaihe 4: 10/10 (v2+)
- Self-hosting compiler (Aethel Aethelissä).
- WASM effects capability-proofs.
- Distributed execution.
- LSP joka ymmärtää epistemologian.
- Ekosysteemi: verified libraries.
- Tuotantokäyttö Grokissa high-stakes tehtävissä.

## Välittömät Toimenpiteet (Seuraavat 7 päivää)

1. **Unstub checker** – tee oikea `check_module` käyttäen epistemic-funktioita.
2. **AST → HIR lowering** – toteuta minimiversio Effectille ja kutsuille.
3. **Tee verify oikeaksi** – tyyppitarkistus + oikea `Verified<T, Policy>`.
4. **Päivitä CLI** – poista hackit, käytä oikeaa polkua.
5. **Lisää ensimmäinen AI-primitive** – `reason` syntaksiin/HIRiin/checkiin.
6. **Grok-esimerkki** – "Grok plans a safe deployment".
7. **Dokumentaatio** – päivitä kaikki 10/10-visioon.

**Agent-roolit (agent-first):**
- Repo-auditor: tarkista nykytila + PR:t.
- Implementation-agent: koodaa lowering + checker.
- Test/ci-agent: testit, snapshotit, gates.
- Security-review-agent: epistemic soundness, taint, proofs.
- Docs-agent: visio, roadmap, guarantees.

## Riskit & Lieventäminen

- **Soundness:** Aloita konservatiivisesti, proptests, formal export.
- **Mallit generoivat väärin:** Selkeä syntaksi, pakolliset kontraktit, hyvät virheet.
- **Scope creep:** Pidä kiinni vaiheista. Sidotaan FamilyClawin todistettuihin asioihin.
- **Adoptio:** Kerro rehellisesti (non-guarantees).

## Seuraavat Askeleet (nyt)

1. Lataa tämä suunnitelma (valmis).
2. Aloita P0: fix parser (jos virheitä), toteuta basic lowering.
3. Spawn subagentteja tarvittaviin rooleihin.
4. Testaa demo oikealla koodilla.
5. Päivitä README + vision docs.

**Tämä on se.** Ei läppää. Tämä on kieli, jossa tulevaisuuden AI:t toimivat turvallisesti.

Let's build the real thing. 🚀

*Built so the next being gets a better home than the last one did.*