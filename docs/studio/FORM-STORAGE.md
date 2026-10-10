# Storing a form's data: the design

Status: **a proposal for the owner to approve**, written 2026-10-11. Nothing here is built. It is the next
slice of the Studio form builder (`FIRST-SLICE.md`, decision 4: storage and a generated backend over
PostgreSQL are "the destination, after the slice works"). The slice that exists keeps nothing: a form's values
are worked out and forgotten.

## What this slice adds

A generated app that can **save** what was submitted and **list** what was saved, in a database, with the same
exact decimals the preview shows. Not a second product: the same `kaNakkitu` function, a table, and three more
routes in the server the form builder already downloads (`pativam_cEvY.qmz`).

## Findings that shape it

These are from the code, not from memory.

1. **The standard release has SQLite and no PostgreSQL.** `etamil_compiler/Cargo.toml` has
   `default = ["sqlite", "http-client"]`; `postgres` is a separate feature, and the release workflow builds the
   default. So a generated app that needs PostgreSQL does **not run on a downloaded `etamil`**: it needs a build
   made with `--features postgres`, as the droplet's is. A generated app that uses SQLite runs on any release.
2. **The two drivers treat a decimal differently.** SQLite has no exact decimal type, so the driver stores a
   decimal as text (`250.50` goes in as `"250.5"`) and reads text that parses as a number back as a number.
   PostgreSQL stores `NUMERIC` natively. Both are exact. The traps are SQLite's: text orders after every number,
   and `SUM` over text is a float (see `examples/db_samples/qaLam_ceyal_paqivu.qmz`). A list that only reads rows
   in order does not meet either.
3. **The placeholder differs.** `?` for SQLite and MySQL, `$1, $2` for PostgreSQL. Queries are always
   parameterised; there is no way to splice a value into the SQL text from eTamil.
4. **`etamil --server` listens on `127.0.0.1` by default** (`--host` changes it). A generated app is local to the
   machine unless someone asks otherwise.
5. **A name in the database follows the project's naming rule**: tables, columns and fields are written in the
   ezuqqu romanization because they are seen by systems with no Tamil fonts. The form builder lets a field be
   named in Tamil letters; a stored form cannot.

## The design

### One table per form, typed columns

```sql
CREATE TABLE IF NOT EXISTS pativu (          -- the form's table name, chosen in Studio, ASCII
    ilakkam  INTEGER PRIMARY KEY,            -- the record's number
    nEram    TEXT NOT NULL,                  -- when it was saved, UTC, ISO 8601
    paqippu  INTEGER NOT NULL,               -- the version of the form's design that saved it
    qokY     TEXT,                           -- a field          (SQLite: exact text; PostgreSQL: NUMERIC)
    vikiqam  TEXT,                           -- a percent field, as typed: 18, not 0.18
    peyar    TEXT,                           -- a text field
    vari     TEXT,                           -- a calculation, as it was worked out when it was saved
    moqqam   TEXT
);
```

- The record's own columns (`ilakkam` number, `nEram` time, `paqippu` version) are named in the project's
  romanization, so they follow the rule they enforce on the author's fields.
- A **number or percent** is `NUMERIC` in PostgreSQL and exact text in SQLite. A **percent** is stored as typed
  (`18`), so the record says what the person entered, and is divided by 100 only when it is worked out.
- A **text** field is `TEXT`.
- **Both inputs and results are stored.** A saved record has to say what the person saw: formulas change, and a
  record that recalculates when the form is edited is not a record.

### Three routes beside the one that exists

| Route | Does |
|---|---|
| `POST /kaNakku` | exists: works the form out, stores nothing |
| `POST /paqivu` | validates, works out with `kaNakkitu`, **inserts**, answers `{ilakkam, values, messages, problems}`. A form with a problem or a failed check is **not saved** and says why. |
| `GET /paqivukaL` | the saved records, newest first, at most the last 100, as JSON |
| `GET /paqivukaL.csv` | the same as CSV, for a spreadsheet |

No update and no delete in this slice: a record is appended and never changed.

### When the form changes

- **Adding a field or a calculation** adds a column when the app starts (`ALTER TABLE ... ADD COLUMN` for each
  name not in the table), and old rows read as empty there. The version in `paqippu` goes up.
- **Removing or renaming** never drops a column. A retired field stays in the table with its old rows.
- **Changing a field's type** is refused at start, with a message that says to give the form a new table name.
  Quietly converting stored values is not something a form builder should do.

### The same generator for both databases

The generator takes the database as a parameter and writes the right placeholder and column types. **SQLite
first**, because it runs on any release with nothing installed; PostgreSQL as the second target from the same
code, for anyone with a build that has the driver.

## Decisions for the owner

1. **Which database first.** (a) SQLite first, PostgreSQL second from the same generator. (b) PostgreSQL only,
   which needs `--features postgres` and so does not run on the standard release. **Recommendation: a.** A
   second, separate question follows: whether the release archives should carry the PostgreSQL driver
   (about seventy more crates, a longer build, a bigger binary). **Recommendation: not yet.**
2. **Store the results, or only the inputs.** (a) Both, as a snapshot. (b) Inputs only, and recalculate when
   read. **Recommendation: a.** It is what makes a saved record trustworthy.
3. **ASCII names when storage is on.** The form builder would refuse a Tamil-letter field name on a form that is
   saved, and say to use the romanization (it already offers the sample's `qokY`, `vikiqam`).
   **Recommendation: yes.** It is the project's naming rule, applied where it matters.
4. **Append-only.** No edit and no delete in this slice. **Recommendation: yes.** Both need a story about who may
   do them, and that needs accounts (decision 5 in `FIRST-SLICE.md`: none).
5. **Access.** None: no sign-in, so anyone who can reach the app can read and add records. The app listens on
   `127.0.0.1` unless told otherwise, and the form builder would say so beside the download. **Recommendation:
   keep it local and say so.** Authentication comes with the accounts decision; the language already has token
   builtins for it.

A note for the owner, not a recommendation: a form that stores **personal data** (a PAN, an Aadhaar number, an
account number) puts the person who runs the app under data-protection duties. The first slice does not block
such fields, and does not encrypt anything. Whether it should warn, or refuse, is a policy for you, and for
whoever advises you on it.

## How it will be tested

As the slice before it was, with a real `etamil`, not a stand-in:

- a real server is started with a database file in a temporary folder, and asked over HTTP;
- **`250.50` is saved and read back as `295.59`**, exactly, from a file on disk;
- a form with a failed check or a bad field is **not stored**, and the response says why;
- the list is newest first, and the CSV has the same rows;
- the server is stopped and started again, and the rows are still there;
- a text field containing a quote, a semicolon and `'; DROP TABLE` is stored as typed and nothing else happens;
- a design with a Tamil-letter name is refused when storage is on;
- adding a field to a form that already has rows works and the old rows read as empty; changing a type is refused.

For PostgreSQL, the same cases in a CI job with a PostgreSQL service, on a build with the feature.

## Not in this slice

Edit and delete; sign-in and roles; encryption at rest; migration of a type change; a generated React frontend;
filtering, sorting or totals in the database (SQLite's text ordering and its float `SUM` are the reason to do
those in eTamil, not SQL); multiple forms in one app; deployment (the AWS and Azure templates exist and take an
image, not this app).
