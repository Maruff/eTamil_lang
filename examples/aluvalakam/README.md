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

¹ `project/services/` `scheduling`, `dependencies`, `tasks`, `change_control`,
`status_reports`, `scope_wbs`, `project_features`, `lifecycle`,
`field_defaults`; `docgen/context`; `product/services/product_management`; and
the SQL views `db/phase6/19_kpi_views` and `23_status_report_items`.
² `integrations/azure_devops/` `mapper`, `applier`, `client`, `publisher`,
`reconcile`, and `integrations/common/` `fingerprint`, `policy`, `retry`.
³ Eleven `qittam` modules, nine `oruwkiNYppu` and two `qayArippu`.
