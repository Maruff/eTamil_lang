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
ALUVALAKAM_HOOK_TOKEN=hook-token \
  etamil --server --port 8095 examples/aluvalakam/aluvalakam_cEvY.qmz
```

| Route | What it does |
|---|---|
| `GET /qittam/:kuRi/attavaNY` | the schedule as of today, rescheduled on actuals |
| `GET /qittam/:kuRi/nalam` | RAG health, its reasons, and the go-live forecast |
| `POST /qittam/:kuRi/paNi/:paNi` | record progress; on a baselined project only progress fields move, anything else is a 409 |
| `POST /mARRam/:kuRi` | move a change request; approval turns its items into tasks and queues them |
| `GET /palaqittam` | the portfolio, overall and by client |
| `GET /varicY` | the outbox |
| `POST /ajUr/kokki/:cIttu` | an Azure DevOps service hook: token, echo brakes, field ownership |

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
| `aluvalakam_kYyALi_cOqaZY.qmz` | 40 assertions driving every handler against a real SQLite file, with Azure DevOps faked |

The handlers are functions rather than code inside the routes so that the whole
service is tested under `--vm` — including a failed send rescheduled, a child
sent after its parent in the same round, an echo refused, and a developer's
rename of a PMO-owned title rejected while their progress is applied. The
server file itself stops at its first route under `--vm`, as route examples do,
and is listed as such in `scripts/run_examples.sh`.

It was also run as a server and driven over HTTP: closing a task on its actual
dates moved go-live back onto the baseline and turned the project green; a
locked field came back 409 naming the field; a change request that tried to
skip review came back 409, and on approval produced its task; and with Azure
DevOps not configured the worker tried the queued rows, recorded why they
failed, and kept them waiting.

**What it does not do.** There is no sign-in: anyone who can reach the port can
record progress, and a real deployment puts it behind one. It holds one
database connection at a time, which is what the host provides. The outbox
times its backoff against the process's own clock — the language has no wall
clock in seconds — so a restart makes every waiting row due at once. And the
parent link it sends to Azure DevOps carries `?api-version=7.1`, which has not
yet been checked against a real organisation.

### The whole application

| | Code lines |
|---|---:|
| BeakPMO for the same ground — domain logic, Azure DevOps integration, and the API routes, repositories, sync worker and sync SQL behind it⁴ | 4,653 |
| **the service** — `kaLam`, `kYyALi`, `cEvY`, `amYppu` | **495** |
| the `nUlakam` modules it uses | 1,293 |

BeakPMO's API and repository files serve more endpoints than these seven —
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
`integrations`, `dashboards`; `project/repositories/` `tasks`,
`change_control`, `integrations`, `projects`; `workers/sync`; and
`db/phase4/01_schema` and `02_functions_triggers`.
