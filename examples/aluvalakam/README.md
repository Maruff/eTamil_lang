# examples/aluvalakam — a project office, end to end

```bash
etamil --vm examples/aluvalakam/aluvalakam.qmz
```

`aluvalakam.qmz` (அலுவலகம், *office*) takes one project — **திட்டம் வாசல்**,
a customer portal for ACME — through what a PMO application does to it,
using the project, Azure DevOps and product libraries together. Every rule is
in `nUlakam`; the program holds only the project's facts and the order things
happen in.

| Step | What happens | Libraries |
|---|---|---|
| 1. Scope | WBS rows in, excluded or removed; the SSO feature adds 16 hours to *Build* and a task of its own; PDF, not chosen, adds nothing | `qittam/nOkkam` |
| 2. Plan | estimates become working days; the network is laid onto a Monday–Friday calendar with Gandhi Jayanti off; go-live 11 November | `qittam/attavaNY`, `qittam/maRu_attavaNY`, `nAtkAtti/vElYnAL` |
| 3. Charter | defaults in three layers (organisation, product, project); readiness 100%; rendered with effort as person-days | `qittam/kattam`, `qittam/AvaNac_cUzal`, `vativam/AvaNam` |
| 4. Two weeks in | design is 60% done on the day it was due; rescheduling on actuals moves go-live a day, the milestone goes at risk, the project amber — and the status report is rendered from the data | `qittam/maRu_attavaNY`, `qittam/nalam`, `qittam/nilY_aRikkY` |
| 5. Change request | CR-3 reviewed, acknowledged and approved, each actor stamped; it becomes a sub-task that the signed scope lets in; the float in *Build* absorbs it | `qittam/mARRak_kOrikkY`, `qittam/nilYmARRam` |
| 6. Azure DevOps | a work-item type and state for every task, a validated patch of only the fields the PMO owns, an outbox row each that cannot be queued twice; the first send fails and is rescheduled; an incoming hook is applied with the title edit refused | `oruwkiNYppu/*` |
| 7. Portfolio | three projects rolled up by hours and by client | `qittam/pala_qittam` |
| 8. Product | the release carrying PDF export is 6.5.0; the promise to ACME holds | `qayArippu/*` |

It reaches the network once, at `127.0.0.1:9`, which refuses at once — that
is how step 6 shows a failed send being scheduled again rather than lost.

## How much code

The question this example was written to answer is whether an application
built on these libraries is smaller than one written without them. Counting
code lines only — no blank lines, comments or docstrings — for the same
features:

| | Code lines |
|---|---:|
| BeakPMO's domain logic for these features¹ | 1,668 |
| BeakPMO's Azure DevOps integration² | 1,361 |
| **BeakPMO, total** | **3,029** |
| **`aluvalakam.qmz`** — what an application writes | **223** |
| the `nUlakam` modules it uses, written once and shared³ | 1,293 |

The application's own code is about a thirteenth of BeakPMO's for the same
ground, and the libraries and application together are about half.

**What that comparison does not say.** BeakPMO's files also read and write a
database, and its integration runs a worker loop and a reconciliation sweep
against a live Azure DevOps; this example keeps its data in memory and sends
one request. A real application on these libraries still needs its storage,
its HTTP API, its screens and sign-in — none of which is here, and none of
which is counted on either side. What *is* like for like is the domain logic:
the schedule, the health rules, change control, scope, the document context and
the sync rules, which BeakPMO writes inline and an eTamil application calls.

No performance comparison is claimed. On the machine this was written on, the
whole program — twenty-five modules loaded, every step above — runs in about
2.5 seconds, most of it start-up.

## The same office, as a running service

`aluvalakam.qmz` keeps its project in memory. The service beside it keeps it in
SQLite, answers HTTP, and drains the Azure DevOps outbox on a timer:

```bash
etamil --vm examples/aluvalakam/aluvalakam_amYppu.qmz            # the database, seeded
ETAMIL_JWT_SECRET=… ALUVALAKAM_HOOK_TOKEN=hook-token \
  etamil --server --port 8095 examples/aluvalakam/aluvalakam_cEvY.qmz
```

Sign in with `POST /uLnuzYvu` and `{"பயனர்_பெயர்": "meena", "கடவுச்சொல்":
"demo-password"}`; the reply's `சீட்டு` goes on every other request as
`Authorization: Bearer <token>`, and is good for eight hours. The seed makes
three users, one per role — `meena` manages, `arun` is on the team, `ravi` is a
stakeholder — all with the password `demo-password`, which is for a laptop and
nowhere else. The manager adds everyone after that, through `/payaZarkaL`.

| Route | Who | What it does |
|---|---|---|
| `POST /uLnuzYvu` | anyone | sign in; the reply carries a signed token |
| `POST /qittam` | manager | open a project — `{திட்டக்_குறி, பெயர், தொடக்க_நாள்}`, and optionally the customer, sponsor, objective, holidays and a target end |
| `GET /qittam/:kuRi` | any role | the project's record, and how many tasks it has |
| `POST /qittam/:kuRi` | manager | change the name, customer, sponsor or objective; the start, target end and holidays only before the baseline |
| `POST /qittam/:kuRi/atippatY` | manager | baseline: schedule the scope on the project's calendar, record where it ends, and lock the scope |
| `POST /qittam/:kuRi/mUtu` | manager | close the project; it takes no changes after that |
| `GET /qittam/:kuRi/attavaNY` | any role | the schedule as of today, rescheduled on actuals |
| `GET /qittam/:kuRi/nalam` | any role | RAG health, its reasons, and the go-live forecast |
| `GET /qittam/:kuRi/aRikkY` | any role | this week's status report — achievements, blockers, next steps — and its rendered document body |
| `GET /qittam/:kuRi/cAcaZam` | any role | the charter: how ready it is, what it still lacks, and its rendered body |
| `GET /qittam/:kuRi/aRikkY/kOppu` | any role | the status report as a Word file, filled from `vArppukaL/aRikkY.docx` |
| `GET /qittam/:kuRi/cAcaZam/kOppu` | any role | the charter as a Word file, filled from `vArppukaL/cAcaZam.docx` |
| `POST /qittam/:kuRi/paNi/:paNi` | manager, team | record progress; on a baselined project only progress fields move, anything else is a 409 |
| `POST /qittam/:kuRi/nOkkam` | manager | set the scope and features before baseline; tasks that stay keep their progress, and one that leaves is deleted — or cancelled, if it is already a work item |
| `POST /mARRam/:kuRi` | manager | move a change request; approval turns its items into tasks and queues them |
| `GET /palaqittam` | any role | the portfolio, overall and by client |
| `GET /vAkkuRuqi` | any role | customer commitments: overdue, notice due, notice late |
| `POST /vAkkuRuqi/:kuRi` | manager | record that the customer has been told |
| `GET /varicY` | manager | the outbox |
| `GET /payaZarkaL` | manager | the users, with their roles and never their password hashes |
| `POST /payaZarkaL` | manager | add a user — `{பயனர்_பெயர், கடவுச்சொல், பயனர்_பங்கு}`; a name already taken is a 409, and a password needs twelve characters |
| `POST /payaZarkaL/:peyar/pawku` | manager | change a user's role, from their next request on; the last manager stays a manager |
| `POST /payaZarkaL/:peyar/katavuccol` | manager | reset a user's password, which signs them out everywhere |
| `POST /ajUr/kokki/:cIttu` | the hook's token | an Azure DevOps service hook: token, echo brakes, field ownership |

No token, a forged one or an expired one is a 401; a role the route does not
admit is a 403.

Every 15 seconds the worker sends what is due, parents first, and backs off on
failure. Set `AZURE_DEVOPS_ORG`, `AZURE_DEVOPS_PROJECT` and `AZURE_DEVOPS_PAT`
to sync for real; without them every send fails with status 0 and the rows wait
and, after eight attempts, park.

| File | |
|---|---|
| `aluvalakam_kaLam.qmz` | storage — one SQLite table per kind of record, each row a key and the record as JSON |
| `aluvalakam_kYyALi.qmz` | the handlers — what each route does, as functions that take today's date and Azure DevOps as arguments |
| `aluvalakam_cEvY.qmz` | the server — routes, the outbox worker, the real Azure DevOps calls |
| `aluvalakam_amYppu.qmz` | creates and seeds the database |
| `vArppukaL/` | the two Word templates the documents are filled from — `aRikkY.docx`, `cAcaZam.docx` |
| `aluvalakam_kYyALi_cOqaZY.qmz` | 120 assertions driving every handler against a real SQLite file, with Azure DevOps faked |

The handlers are functions rather than code inside the routes so that the whole
service is tested under `--vm` — including a failed send rescheduled, a child
sent after its parent in the same round, an echo refused, a developer's rename
of a PMO-owned title rejected while their progress is applied, a forged token
and every role refused where it should be, and a rescope that keeps a task's
progress, deletes one task and cancels another. The
server file itself stops at its first route under `--vm`, as route examples do,
and is listed as such in `scripts/run_examples.sh`.

It was also run as a server and driven over HTTP, signed in: a stakeholder
could read but not record progress, a team member could record progress but
not move a change request, and the manager's approval produced the change's
task; closing a task on its actual dates moved go-live back onto the baseline;
the status report and charter came back rendered; and with Azure DevOps not
configured the worker tried the queued rows, recorded why they failed, and
scheduled them again at wall-clock seconds. A manager added a user, who could
record progress until demoted, and the token they already held lost the right
on the next request; demoting the last manager was refused; a password reset
refused the old token and let the new password in; and the report and charter
came down as Word files that Word, LibreOffice and python-docx open.

**What it does not do.** It holds one database connection at a time, which is
what the host provides. It serves plain HTTP; put TLS in front of it. And the parent link it sends to Azure DevOps carries `?api-version=7.1`,
which has not yet been checked against a real organisation.

**Users.** A role is read from the user's record on every request rather than
from the token, so a change holds at once. A password reset moves the record's
`சீட்டுப்_பதிப்பு` on, and a token that carries an older one is refused — which
is how a reset signs someone out without a list of revoked tokens.

**Documents in Word.** The `.docx` routes fill the templates in `vArppukaL/`
with `பொதியை_நிரப்பு`: only `word/document.xml` changes, so a template's styles,
fonts and pictures come through as they were made. Edit them in Word. A
placeholder is `{{ திட்டம்.பெயர் }}`, and a repeating row is a table row between
a `{%tr for சாதனை in சாதனைகள் %}` row and a `{%tr endfor %}` row, as the
templates show. Word often stores a placeholder typed in two sittings in two
runs; `பிளந்த_குறிகளை_இணை` joins it back before filling, and the rejoined text
takes the formatting of the run it starts in. Keep each list's heading row,
so a list that comes out empty leaves a table Word still opens. The service
looks for the templates in `ALUVALAKAM_TEMPLATES` (by default
`examples/aluvalakam/vArppukaL`, from the repository root), writes the filled
file to `ALUVALAKAM_OUT` (by default the system's temporary directory), and
answers 503 when a template is missing rather than send an empty document. The
JSON routes render from the same templates, so an edit in Word shows there too,
and fall back to the templates in the code when none is installed; their reply's
`வார்ப்பு_மூலம்` says which was used.

**Projects.** A manager opens a project, fills in what the charter asks for,
sets the scope, and baselines it: the tasks are scheduled on the project's
calendar from its start, where they end becomes the baseline every forecast is
measured against, and the scope locks. After that the dates move only by
rescheduling on actuals and the scope only by change request, so editing a date
by hand is a 409. Closing is final. There is no delete: a project with history
is closed, not erased.

The outbox's backoff is timed with `இப்போதைய_நொடி()`, the seconds clock added to
the language for this: before it, the only clock finer than a day counted from
the program's start, so a row's next attempt meant nothing after a restart.

### The whole application

| | Code lines |
|---|---:|
| BeakPMO for the same ground — domain logic, Azure DevOps integration, and the sign-in, API routes, repositories, sync worker and sync SQL behind it⁴ | 5,599 |
| **the service** — `kaLam`, `kYyALi`, `cEvY`, `amYppu` | **1,118** |
| the `nUlakam` modules it uses | 1,293 |

BeakPMO's API and repository files serve more endpoints than these twenty-four —
full create, read, update and delete, with authentication in front — so the
ratio flatters the service, and should be read as the size of the same *kind*
of application rather than the same application. The domain-logic comparison
above is the like-for-like one.

¹ `project/services/` `scheduling`, `dependencies`, `tasks`, `change_control`,
`status_reports`, `scope_wbs`, `project_features`, `lifecycle`,
`field_defaults`; `docgen/context`; `product/services/product_management`; and
the SQL views `db/phase6/19_kpi_views` and `23_status_report_items`.
² `integrations/azure_devops/` `mapper`, `applier`, `client`, `publisher`,
`reconcile`, and `integrations/common/` `fingerprint`, `policy`, `retry`.
³ Eleven `qittam` modules, nine `oruwkiNYppu` and two `qayArippu`.
⁴ The files in ¹ and ², and `project/api/` `tasks`, `change_control`,
`integrations`, `dashboards`, `scope_wbs`, `status_reports`, `lifecycle`;
`project/repositories/` `tasks`, `change_control`, `integrations`, `projects`,
`scope_wbs`, `status_reports`; `product/api` and `product/repositories`
`product_management`; `core/security` and `common/api/auth`; `workers/sync`;
and `db/phase4/01_schema` and `02_functions_triggers`.
