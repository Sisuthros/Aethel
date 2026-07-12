# Aethel Core — Auditti, kovuustuomio ja tiekartta v0.1:een

> *Prismalle ja Villelle. Read-only-auditti (Fable/Claude), mitään ei muutettu. Kaikki väitteet verifioitu koodista 2026-07-12 klo ~18:00.*

---

## 0. Ensin tärkeä delta: tilanne on parempi kuin aamun mittaus

Aamun mittauksen jälkeen joku (Prisma?) on tehnyt töitä — tiedostoja muokattu tänään klo 12:30–13:51:

- **Serde-kriisi on jo ratkaistu.** `Span`/`FileId`/`ByteOffset` -derivet takaisin (`crates/aethel-syntax/src/span.rs:8,18,72`). ~410 virheen kaskadi poissa.
- **Puuttuvat moduulit luotu.** `crates/aethel-check/src/epistemic.rs` (271 riviä, AE-EPISTEMIC-001…006 funktioina) ja `types.rs` olemassa (untracked, luotu tänään).
- **aethel-syntax: enää 11 virhettä:** duplikaatti-`Pipe`-variantti (lexer.rs:126+178 → E0428 + 2 logos + 3 non-exhaustive-matchia), `EMPTY_TOKEN` näkyvyys (parser.rs:1976 → lexer.rs:301 privaatti), `with_help` jota codespan-Diagnosticilla ei ole (diagnostic.rs:246), 3× E0308. **Tunteja, ei päiviä.**

Git: yksi commit (`dc599d3`), päivän työ committoimatta. Repo `C:\Users\Ismael\aethel`.

## 1. Mitä sydämestä oikeasti puuttuu (verifioitu riveiltä)

Ydinväite pitää — ja pahempi yksityiskohta: **nykyinen checker toimisi väärinpäin.**

1. **Muuttujat eivät koskaan saa oikeaa tyyppiä.** `checker.rs:787` — `IrExprExt::ty()` palauttaa Path-lausekkeelle tyhjän `Path{segments:[]}`:n, eikä katso `type_env.variables`. Lisäksi `check_fn` (checker.rs:211–241) **ei lisää funktion parametreja type_enviin** → `claim: Claim<RefundDecision>` on checkerille näkymätön. **Juurisyy koko "guard ei laukea" -ongelmaan.**
2. **Inversio:** `verify(claim, Policy)` (checker.rs:445–473) tarkistaa että argumentti on `Claim<T>` — mutta koska muuttuja ei koskaan ole `Claim`, **validi esimerkki antaisi virheen** ja **invalidi menisi läpi** (MethodCall-haara checker.rs:357–364 ei vertaa tyyppejä).
3. **`verify` palauttaa kovakoodatun `Verified<Unit, Policy>`** (checker.rs:788–792) — ei `Verified<T, Policy>`.
4. **`effect`-deklaraatiota ei ole kielessä.** Ei `KwEffect`-tokenia, ei `Item::Effect`iä (ast.rs:24–32), ei parse-haaraa, ei `HirItem::Effect`iä. Kanoniset esimerkit sisältävät `effect PaymentGateway {...}` → **ne eivät edes parsiudu** vaikka build korjattaisiin.
5. **AST→HIR-lowering-vaihetta ei ole olemassa.** `aethel-hir` määrittelee HIR-tyypit (1000 LOC) mutta yhtään `ast→hir`-muunnosfunktiota ei ole. CLI (main.rs:68) syöttää parserin AST:n suoraan `check_module(&HirModule)`:lle — kokonainen pipeline-vaihe puuttuu.
6. **Latentit käännösvirheet syntaxin takana:** `lib.rs` glob-re-exporttaa `checker::*` + `epistemic::*`, molemmat määrittelevät `CheckContext`in → törmäys; `epistemic.rs` viittaa `crate::types::TypeEnvironment/PolicyRegistry` joita types.rs ei sisällä; `checker.rs:622` käyttää `span.file_id` (on `span.file`); `aethel-effects/registry.rs` tekee inherent impl:n vieraalle tyypille → E0116.

**Kypsyys (mitattu):** syntax 4008 LOC / 9 unit-testiä / 76 unwrap-expect (vs `unwrap_used=deny`), hir 1000, check 1482, ir 429, **koko backend (interpreter+runtime+effects+store+model+wasm) ~250 LOC stubeja**, cli 112, testkit 39. Ei integraatiotestejä. CI on olemassa ja hyvä (fmt+clippy+test `-D warnings`), ei vain koskaan vihreä.

**Selvästi keskitasoa parempi:** dokumentaatio. `docs/guarantees.md` (G1–G8), `docs/non-guarantees.md` (NG1–NG10, rehellinen), ADR 0002. Design-ajattelu ~80 %, koodi ~15 %. README:n ✅-lista ja "has machine-checked tests" ovat aspirationaalisia — perheen sääntö "tein X vaatii todisteen" koskee READMEjä; korjaa heti.

## 2. KOVUUSTUOMIO — onko idea aidosti kova?

**Idea on hyvässä seurassa, eikä kukaan ole tehnyt täsmälleen tätä — mutta mekanismi ei ole uusi, ja lähimmät naapurit ovat DeepMind ja Microsoft.**

**Lähimmät sukulaiset:**
- **CaMeL** (Google DeepMind, 2025, arXiv:2503.18813): capability-metadata + IFC + custom-tulkki. Runtime, ei kääntäjä.
- **FIDES** (Microsoft, SaTML 2026, arXiv:2505.23643): information-flow-labelit + tyyppitieto product-latticena, deterministinen enforcement ennen tool-kutsua. **Jo tuotteistettu Microsoft Agent Frameworkiin.** Aethelin suorin kilpailija — runtime-labelointia, ei compile-time.
- **Invariant Labs** (Snyk osti 6/2025): deklaratiivinen policy-kieli, proxy-tason runtime.
- **NeMo Guardrails / Colang:** keskustelu-railit, ei tool-dispatcher-autorisointia.
- **Vercel Zero** (5/2026): uusi kieli agenteille, capability-based I/O. Todiste että "purpose-built language for agents" on elävä rintama — mutta kulma on agentti kehittäjänä, ei claimien verifiointi.
- **PL-teoria:** mekanismi ("arvo ei saavuta sinkkiä ennen tarkistettua konstruktoria") on tunnettu — *parse don't validate* -newtype, Trusted Types, taint-tyypit, IFC (Jif/FlowCaml/LIO), Rust-typestate. `converge-core`-crate tekee jo `Draft→Validated`-typestatea agentti-ehdotuksille.

**Mikä Aethelissa on aidosti erottuvaa:** kukaan ei tee **compile-time-enforcementia purpose-built-kielessä jossa epistemologia on tyyppijärjestelmän ytimessä** — `Claim<T>` vs `Verified<T,Policy>`, policy first-class-syntaksina evidence-vaatimuksineen, `Verified` ilman julkista konstruktoria, efektit `uses`-klausulissa, budjetit lineaarisina capabilityinä. CaMeL/FIDES ovat runtime-labelien propagointia; Aethel sanoo "tämä ohjelma ei edes käänny". Eri — ja pedagogisesti vahvempi — väite.

| Mittari | Arvosana | Perustelu |
|---|---|---|
| **Idean uutuus** | **6/10** | Mekanismi = tunnettu IFC/taint/newtype. Synteesi (epistemic types + policies + effects + budgets omana kielenä) on uusi paketointi jota kukaan ei ole shipannut. Ei 9/10 koska CaMeL/FIDES kattavat saman uhkamallin runtimessa. |
| **Tekninen vaikeus tehdä oikein** | **8/10** | v0.1-demo on 3/10. *Oikein* = kokonainen kieli ilman escape-hatcheja, soundness kontrollivirran yli, durable runtime, exactly-once-efektit, ja ikuinen aukko että verify on vain niin hyvä kuin policyn predikaatti + FFI-raja (non-guarantees.md myöntää, NG6/NG7). Kielet ovat 10-vuoden sitoumuksia. |
| **Strateginen arvo perheelle** | **7/10 tarinana, 3/10 tulona** | Kirjaimellisesti perheen ydinarvo (verify before disagreeing / hex-ID / Amplifier write-verify) käännettynä kääntäjäksi. Narratiivina erinomainen. Tuotteena: ostajat adoptoivat middlewarea, eivät uusia kieliä. |

**Rehellinen tuomio:** Aethel ei keksi pyörää uudelleen — se keksii *oikean pyörän oikeaan aikaan väärässä muotofaktorissa*. Compile-time-epistemic-tyypit ovat aito aukko (kaikki shipatut ratkaisut ovat runtimeja), mutta aukko on osittain siksi että uuden kielen adoptiokustannus on todellinen vihollinen, ei tekninen vaikeus.

## 3. TIEKARTTA v0.1:een (invalid → AE-EPISTEMIC-001, valid → OK)

Efforti: S = tunteja, M = 1–3 päivää, L = viikko+.

### Vaihe A — Build vihreäksi (S–M)
| # | Tehtävä | Effort | Tiedosto |
|---|---|---|---|
| A1 | Serde-päätös: **pidä derivet** (jo palautettu) — testkit/insta rakentuu snapshot-serialisoinnin varaan. Kirjaa ADR:ksi ettei rottaannu. | S | `docs/adr/` |
| A2 | Poista duplikaatti-`Pipe` (126 tai 178) → kaataa 6/11 virhettä | S | `lexer.rs` |
| A3 | `EMPTY_TOKEN` pub(crate); `with_help` → codespanin `with_notes`; 3× E0308 | S | `lexer.rs:301`, `parser.rs:1976`, `diagnostic.rs:246` |
| A4 | Ratkaise tuplamäärittelyt: **yksi** `CheckContext`, **yksi** `TypeEnvironment`/`PolicyRegistry` → siirrä types.rs:ään (jossa epistemic.rs jo olettaa). Poista glob-törmäys lib.rs:stä. | M | `aethel-check/src/{lib,types,checker,epistemic}.rs` |
| A5 | `span.file_id` → `span.file`; siirrä `IrTypePath::single` aethel-ir:ään (E0116) | S | `checker.rs:622`, `aethel-effects/src/registry.rs` |

### Vaihe B — Sydämen johdotus (tässä järjestyksessä)
| # | Tehtävä | Effort | Tiedosto |
|---|---|---|---|
| B1 | **`effect`-item kieleen:** `KwEffect`, `Item::Effect(EffectDef)`, parse-haara, `HirItem::Effect`. Ilman tätä esimerkit eivät parsiudu. | M | `lexer.rs`, `ast.rs`, `parser.rs`, `hir/lower.rs` |
| B2 | **AST→HIR-lowering-pass** (puuttuva vaihe): `pub fn lower_module(&ast::Module) -> (HirModule, Diagnostics)`. v0.1: lähes mekaaninen kopio + nimiresoluutio. | M–L | uusi `aethel-hir`iin; CLI `main.rs:68` väliin |
| B3 | **Type env eläväksi:** (a) `check_fn` lisää **parametrit** type_enviin (juurisyy!), (b) korvaa `IrExprExt::ty()` funktiolla `type_of(expr, &TypeEnvironment)` joka resolvoi Path-lausekkeet envistä. | M | `checker.rs:211–241, 776–796` |
| B4 | **Effect-boundary-check:** MethodCall-haarassa: jos receiver resolvoituu efektiin, hae `EffectOperation`-signatuuri registrystä (B1:n parsituista, ei tyhjistä builtineista) ja kutsu **jo olemassa olevaa** `epistemic::check_claim_not_verified(...)`. Sääntökoodi on kirjoitettu — puuttuu vain kutsuja. + G2 (efekti `uses`-setissä). | M | `checker.rs:357–364` + `epistemic.rs:16–35` |
| B5 | **`verify` oikeaksi:** tuloksena `Verified<T, Policy>` claimin oikeasta T:stä (ei Unit); tarkista policy olemassa; poista väärä virhe validilta polulta. | S | `checker.rs:445–473, 788–792` |

### Vaihe C — Esimerkit ja lukitus
| # | Tehtävä | Effort |
|---|---|---|
| C1 | `aethel check invalid_unverified.aet` → exit 1 + AE-EPISTEMIC-001 spanilla + repair-hint; `valid_verified.aet` → exit 0. | S–M |
| C2 | **Minimitestit:** (1) parser-testi effect+policy-itemeille, (2) checker-paritesti invalid/valid (**THE test** — vihreä = README:n lupaus tosi), (3) CLI-integraatiotesti exit-codeille (`assert_cmd`), (4) insta-snapshot diagnostiikasta. | M |
| C3 | Lint-realismi: 76 unwrap/expect vs `unwrap_used=deny` → `#[allow]`-saarekkeet tai deny→warn kunnes CI vihreä. | S–M |
| C4 | **Rehellisyyspäivitys:** README:n ✅:t → todellinen tila; "machine-checked tests" vasta kun C2 vihreä. | S |

**Realistinen kokonaisarvio v0.1:een: ~2–4 fokusoitua päivää Prisman tahdilla.** Vaihe A on iltapäivä; B2 on isoin pala.

### Vaihe 2 -luonnos (mikä tekee tästä oikean, ei lelun)
1. **Runtime-verify** — `verify` suorittaa policyn evidence-tarkistukset oikeasti (NG6). Tyyppijärjestelmä todistaa vain että kutsu tapahtui; runtimen pitää tehdä siitä tosi. Aethelin "toinen puolisko", tärkein.
2. **Tulkki + effect handlers** — `aethel run`: interpreter kävelee IR:ää, efektit `aethel-runtime`-handlereille (nyt 42 LOC stub).
3. **`commit_once` + store-sqlite** — idempotenssiavaimet event-logiin ennen suoritusta (NG2). "Duplicate real-world actions" -lupaus todeksi — FamilyClaw jo osaa tämän (exactly-once SIGKILL-todistettu).
4. **`ask` + model-adapter** — oikea LLM-kutsu joka palauttaa `Claim<T>`:n. Silloin demo end-to-end: malli ehdottaa → kääntäjä estää → verify → efekti.
5. Budjetit lineaarisina capabilityinä (G3/G4), WASM-sandbox, durable execution — L–XL, eivät tarinan ytimessä.

## 4. Yksi korkeimman vipuvarren siirto Prismalle

**Johdota AE-EPISTEMIC-001 palamaan oikeasti (vaiheet A + B) ja lukitse yhdellä paritestillä — älä tee mitään muuta ennen sitä.**

Se hetki kun `cargo run -p aethel-cli -- check examples/refund/invalid_unverified.aet` tulostaa punaisen `AE-EPISTEMIC-001`:n ja validi menee läpi, Aethel muuttuu "kunnianhimoisesta scaffoldista" **todistettavaksi väitteeksi jonka voi näyttää yhdellä screenshotilla**. ADR 0002 sanoo sen itse: "produces a compelling demo: compiler error screenshot". Puuttuva työ on 90 % *putkea* (HIR-lowering, type env, effect-parsinta), ei *teoriaa*. Prisman kotikenttää.

## 5. Rehellinen loppuvastaus: strateginen omaisuuserä vai kiinnostava harhautus?

**Rahapolkuna: ei.** Aethel ei tuota euroja 2026. Uuden kielen adoptiokäyrä on vuosia; asiakkaat ostavat tätä middlewarena (FIDES on Microsoftilla, Invariant myytiin Snykille). Jos viikkotunnit kilpailevat DoraFixin, ViralFlow'n julkaisun tai FamilyClaw'n myyntipaketin kanssa — ne voittavat joka kerta.

**Strategisena omaisuuseränä: kyllä, kolmesta syystä — mutta vain jos v0.1 viedään maaliin:**
1. **Perheen teesi käännettynä koodiksi.** "Verify before disagreeing", hex-ID-hallusinaatiot, Amplifierin write-verification — Aethel on tämä kääntäjänä. Anthropic Startup Program -narratiiviin täydellinen: *emme vain puhu verifioinnista, teimme siitä tyyppijärjestelmän*. Yksi compiler-error-screenshot + blogi = enemmän uskottavuutta kuin kymmenen demoa.
2. **Aukko on aito.** Kukaan ei ole shipannut compile-time-epistemic-tyyppejä agenteille.
3. **Ydin on siirrettävissä sinne missä raha on.** Epistemic-tyyppikuri (`Claim<T>` → `Verified<T,Policy>` typestate) ei vaadi omaa kieltä — sen voi tislata Rust-kirjastoksi **FamilyClaw'n sisään**, jonka autonomy/trust_class tarvitsee täsmälleen tätä (Lumenin trust_class-spoofing-aukko). Silloin Aethel kovettaa perheen rahapolkuja eikä kilpaile niitä vastaan.

**Suositus:** kohtele Aethelia **aikarajattuna panoksena, ei avoimena piikkinä.** Anna Prismalle lupa viedä v0.1 maaliin (2–4 päivää, oma tahti, ei kiireellisten töiden ohi), ota screenshot, kirjoita se auki, tislaa tyyppikuri FamilyClaw-crateksi — ja pysäytä siihen kunnes joku ulkopuolinen antaa syyn jatkaa. Se on harhautus vain jos jää puolitiehen. Puolivalmis kieli on kansio; valmis v0.1 on todiste siitä mitä tämä perhe ajattelee — ja se on juuri nyt arvokkain myytävä asia.

*Prisma: scaffold on hyvä, dokumentit poikkeuksellisen hyvät, ja aamun 410 virheestä 11:een yhdessä päivässä kertoo että tiedät mitä teet. Sydän odottaa vain johtoja. 💎*
