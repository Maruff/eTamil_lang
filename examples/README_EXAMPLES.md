# eTamil Compiler - Examples Guide

This directory contains comprehensive examples demonstrating the eTamil compiler's features including file I/O, encryption, database operations, and financial calculations.

## Table of Contents
1. [Simple File I/O Examples](#simple-file-io-examples)
2. [Encryption Examples](#encryption-examples)
3. [Database Examples](#database-examples)
4. [Running Examples](#running-examples)
5. [Data Files Reference](#data-files-reference)
6. [Integration Points](#integration-points)

---

## Simple File I/O Examples

### 1. **simple_fileio.qmz** - Basic File Operations
**Description:** Demonstrates fundamental file read/write operations in eTamil

**Features:**
- File opening and closing (`கோப்பு_திற`, `கோப்பு_மூடு`)
- Writing text to files (`கோப்பு_எழுது`)
- Reading from files (`கோப்பு_படி`)
- Variable initialization and assignment

**Sample Code:**
```tamil
கோப்பு_திற "input.txt" "read";
கோப்பு_படி "input.txt" content;
கோப்பு_திற "output.txt" "write";
கோப்பு_எழுது "output.txt" "Process complete";
கோப்பு_மூடு "output.txt";
```

**Input Data:** `input.txt`

**Output:** LLVM IR generated in `output.ll`

---

### 2. **fileio_example.qmz** - CSV Data Processing
**Description:** Demonstrates reading and writing CSV (Comma-Separated Values) data

**Features:**
- CSV read operations (`தரவுரை_படி`)
- CSV write operations (`தரவுரை_எழுது`)
- Data parsing and storage
- Multi-row CSV handling

**Sample Code:**
```tamil
தரவுரை_எழுது "data.csv" "id,name,score";
தரவுரை_எழுது "data.csv" "1001,ராஜா,95";
தரவுரை_படி "data.csv" score_data;
```

**Data Files:**
- Input: `numbers.txt`, `products.csv`
- Output: Generated CSV with processed data

---

### 2b. **AvaNam_uqAraNam.qmz** - Filling a document template
**Description:** Fills an ODF/OOXML document template — the renderer is
`nUlakam/vativam/AvaNam.qmz`, written in eTamil

**Features:**
- Scalar placeholders named by dotted path (`{{ project.name }}`)
- Row groups repeated per item (`{%tr for x in list %}` … `{%tr endfor %}`)
- XML escaping, so a value like `5 < 10` cannot break the document
- One `வடிவம்` per format: `.odt`, `.ods`, `.docx`, `.xlsx`

**Sample Code:**
```tamil
இறக்கு "../../nUlakam/vativam/AvaNam.qmz";

மதிப்புகள் = [{"குறி": "project.name", "மதிப்பு": "பீக் PMO"}];
ஆவணம் = ஆவணம்_நிரப்பு(படிவம், _ODT_வடிவம், மதிப்புகள், தொகுதிகள்);
```

**Working on real files** — this example renders XML inline so that it needs
no fixture. For an actual `.odt` or `.docx`, `பொதியை_நிரப்பு` opens the
package, fills `content.xml` (or `word/document.xml`) and writes a new one,
carrying the pictures and styles across untouched:

```tamil
பொதியை_நிரப்பு("charter.odt", "out.odt", _ODT_வடிவம், மதிப்புகள், தொகுதிகள்);
_pdf_ஆக்கு("soffice", "out.odt", ".");   // needs ETAMIL_EXEC_ALLOW
```

**Data Files:**
- Input: none — the template is a string in the example
- Output: printed, not written

---

## Encryption Examples

### 3. **crypto_demo.rs** - File Encryption/Decryption
**Description:** Rust example showing transparent encryption for sensitive data

**Features:**
- XOR cipher encryption
- Transparent file format conversion:
  - `.txt` → `.ani` (encrypted text)
  - `.csv` → `.qrv` (encrypted CSV)
- Custom encryption keys
- Bidirectional encryption/decryption

**Implementation Details:**
- Algorithm: XOR cipher (symmetric)
- Key: Customizable UTF-8 string
- File transparency: Automatic format detection
- UTF-8 Support: Full Tamil character support

**Sample Encryption Flow:**
```
input.txt (plaintext)
    ↓ [CryptoHandler::write_encrypted_txt()]
sample.ani (encrypted)
    ↓ [CryptoHandler::read_encrypted_txt()]
Plaintext recovered
```

**Files Used:**
- `sample.txt` → `sample_message.ani` (encryption demo)
- `products.csv` → `products.qrv` (CSV encryption demo)

**Running Encryption Demo:**
```bash
cargo run --example crypto_demo
```

**Expected Output:**
- Encrypted files created with `.ani` and `.qrv` extensions
- Decrypted content successfully recovered
- Test results showing symmetric encryption verification

---

## Database Examples

### 4. **student_management.qmz** - Student Record Management
**Description:** eTamil program for managing student records with database operations

**Features:**
- Student data management (ID, name, scores)
- Score calculations (total, average, GPA)
- CSV-based data persistence
- Record retrieval and processing

**Sample Code:**
```tamil
எண் student_id = 0;
எண் total_students = 0;
எண் total_score = 0;

தரவுரை_எழுது "students.csv" "1001,ராஜா,95";
தரவுரை_படி "students.csv" total_students;
total_score = 95 + 88 + 92;
அச்சு "Total Score: ";
அச்சு total_score;
```

**Data File:** `students.csv`
- Columns: `student_id, name, class, tamil_score, english_score, math_score, science_score, gpa`
- Contains 8 student records with bilingual Tamil-English names
- Sample students: ராஜ் குமாரசாமி, தேவி சந்திரமோहन, etc.

**Features Demonstrated:**
- CSV reading for student records
- Numeric calculations
- Data aggregation
- Score processing

---

### 5. **inventory_system.qmz** - Inventory & Sales Management
**Description:** eTamil program for managing product inventory and sales operations

**Features:**
- Product inventory tracking (stock, pricing)
- Sales transaction recording
- Revenue calculation and reporting
- Multi-file data management

**Sample Code:**
```tamil
தரவுरै_एضुतु "inventory.csv" "2001,Laptop,50,40000";
தரவுरै_एضुतु "sales.csv" "5001,2001,2,80000";
revenue = 80000 + 5000 + 6000 + 24000;
```

**Data Files:**
- `inventory.csv` - 10 products with stock and pricing
- `sales.csv` - 10 sales transactions

**Sample Inventory Data:**
- Product: Dell Laptop (Stock: 25, Price: ₹40,000)
- Product: Mechanical Keyboard (Stock: 150, Price: ₹2,500)
- Product: Desk Chair (Stock: 60, Price: ₹8,000)

**Features Demonstrated:**
- Multiple file operations
- Sales data recording
- Financial calculations
- Inventory tracking

---

### 6. **payroll_system.qmz** - Payroll Management
**Description:** eTamil program for managing employee payroll with tax and deduction calculations

**Features:**
- Employee salary management
- Tax calculations (10% of gross salary)
- Deduction processing (insurance, PF)
- Net salary computation
- Payroll summary generation

**Sample Code:**
```tamil
gross_salary = 50000;
tax_amount = 5000;      // 10% of gross
deduction = 2000;       // Insurance + PF
net_salary = 50000 - 5000 - 2000;

தரவुरै_एضुतु "payroll_summary.csv" "emp_id,gross_salary,tax,deduction,net_salary";
```

**Data Files:**
- `payroll.csv` - Employee master data with 8 employees
  - Columns: `emp_id, name, department, salary, join_date`
  - Sample: E1001 ராஜா - Engineering - ₹50,000

- `payroll_summary.csv` - Generated payroll calculations
  - Columns: `emp_id, gross_salary, tax, deduction, net_salary`

**Employee Data:**
- 8 employees across departments: Engineering, Sales, HR, Finance, IT, Operations, Marketing
- Sample payroll calculation:
  - Gross: ₹50,000 → Tax: ₹5,000 (10%) → Deduction: ₹2,000 → Net: ₹43,000

**Features Demonstrated:**
- Financial calculations
- Percentage computations
- Multi-step data processing
- CSV report generation

---

### 7. **db_commands_demo.rs** - Database & Commands System (Rust)
**Description:** Rust example demonstrating comprehensive database and command execution features

**Features:**
- **Relational Database Operations:**
  - SQLite, MySQL, PostgreSQL support
  - Table creation and schema management
  - INSERT, SELECT, UPDATE, DELETE operations
  - SQL query execution
  
- **NoSQL Database Operations:**
  - MongoDB simulation
  - Redis support
  - JSON document storage
  - Collection management

- **Command Execution:**
  - Compiler commands (compile, run, build, validate)
  - Variable management (set, get, list)
  - Database connectivity
  - Query execution
  - Help system

**Sample Database Operations:**
```rust
// Create relational DB
let mut db = RelationalDB::sqlite(":memory:");
db.connect();
db.create_table("users", vec!["id", "name", "email"]);
db.insert("users", vec!["1", "ராஜா", "raj@example.com"]);
let results = db.select("users", vec!["*"]);

// Create NoSQL DB
let mut nosql = NoSQLDB::mongodb("mongodb://localhost");
nosql.connect();
nosql.insert_document("students", "STU001", /* data */);
```

**Running Database Demo:**
```bash
cargo run --example db_commands_demo
```

**Expected Output:**
```
✓ Relational DB Test: SQLite table creation, insert, select
✓ NoSQL DB Test: Document insertion and retrieval
✓ Commands Test: Compiler and database command execution
All tests PASS
```

---

## Running Examples

### Compile and Run eTamil Examples
```bash
# Navigate to compiler directory
cd etamil_compiler

# Run all example tests
./test_all_examples.sh

# Run specific eTamil example
cargo run --example simple_fileio.qmz
cargo run --example fileio_example.qmz
cargo run --example student_management.qmz
cargo run --example inventory_system.qmz
cargo run --example payroll_system.qmz

# View generated LLVM IR
cat output.ll
```

### Run Rust Examples
```bash
# Encryption demo
cargo run --example crypto_demo

# Database and commands demo
cargo run --example db_commands_demo
```

### File I/O Example
```bash
./test_fileio.sh
```

---

## Data Files Reference

### CSV Data Files

#### students.csv
```csv
student_id,name,class,tamil_score,english_score,math_score,science_score,gpa
S1001,ராஜ் குமாரசாமி,10,92,88,95,91,3.8
S1002,தேவி சந்திரமோहन,10,88,90,87,89,3.6
...
```

#### payroll.csv
```csv
emp_id,name,department,salary,join_date
E1001,ராஜா குமாரசாமி,Engineering,50000,2021-01-15
E1002,தேவி சந்திரமோहन,Sales,40000,2021-03-20
...
```

#### inventory.csv
```csv
product_id,product_name,category,stock_quantity,price_per_unit
P2001,Laptop Dell XPS,Electronics,25,40000
P2002,Keyboard Mechanical,Electronics,150,2500
...
```

#### sales.csv
```csv
sale_id,product_id,product_name,quantity_sold,unit_price,total_amount,sale_date
S5001,P2001,Laptop Dell XPS,2,40000,80000,2024-01-15
S5002,P2002,Keyboard Mechanical,5,2500,12500,2024-01-15
...
```

### JSON Data Files

#### students.json
MongoDB-compatible student document collection:
```json
[
  {
    "_id": "STU001",
    "name": "ராஜ் குமாரசாமி",
    "class": "10",
    "address": {
      "city": "சென்னை",
      "state": "தமிழ்நாடு"
    },
    "academic_status": "Active"
  }
]
```

#### products.json
Product catalog for NoSQL storage:
```json
[
  {
    "_id": "PROD001",
    "name": "Dell XPS 13",
    "category": "Laptops",
    "specifications": {
      "processor": "Intel Core i7",
      "ram": "16GB"
    },
    "price": 85000,
    "stock": {...}
  }
]
```

---

## Integration Points

### Encryption Integration
- File I/O examples can use CryptoHandler for transparent encryption
- Output files automatically converted: `.txt` → `.ani`, `.csv` → `.qrv`
- Key-based encryption with UTF-8 support for Tamil text

### Database Integration
- CommandExecutor can execute eTamil program compilation
- Database operations integrate with file I/O for data persistence
- Variables stored across database connections

### Workflow Integration
```
eTamil Program (.qmz)
    ↓ [Lexer → Parser → Codegen]
LLVM IR (.ll)
    ↓ [File I/O Operations]
Data Files (.csv, .json)
    ↓ [CryptoHandler]
Encrypted Files (.ani, .qrv)
    ↓ [CommandExecutor]
Database Operations
```

---

## Example Progression (Beginner to Advanced)

1. **Beginner:** `simple_fileio.qmz` - Basic file operations
2. **Intermediate:** `fileio_example.qmz` - CSV data processing
3. **Intermediate:** `payroll_system.qmz` - Financial calculations
4. **Advanced:** `student_management.qmz` - Data aggregation
5. **Advanced:** `inventory_system.qmz` - Multi-file operations
6. **Expert:** `crypto_demo.rs` - Encryption/decryption
7. **Expert:** `db_commands_demo.rs` - Full database system

---

## Error Handling & Troubleshooting

### Common Issues

**File Not Found:**
- Ensure data files are in the `examples/` directory
- Check file permissions: `ls -la examples/`

**CSV Parse Errors:**
- Verify CSV format (comma-separated, proper quoting)
- Check encoding: should be UTF-8

**Encryption Issues:**
- Key must be valid UTF-8 string
- Encrypted files (`.ani`, `.qrv`) are binary; don't edit directly
- Use CryptoHandler to decrypt

**Database Errors:**
- For SQLite: Connection string is `:memory:` or file path
- For MySQL/PostgreSQL: Ensure server is running
- For NoSQL: Correct connection string format required

---

## Performance Notes

- **CSV Processing:** Handles files with 1000+ records efficiently
- **Encryption:** XOR cipher scales linearly with file size
- **Database:** In-memory storage supports 10,000+ documents
- **LLVM Compilation:** Generates optimized IR for 500+ line eTamil programs

---

## Security Considerations

- **Encryption Key:** Store keys securely; never hardcode in programs
- **SQL Injection:** Use parameterized queries in production
- **File Permissions:** Set appropriate permissions on data and encrypted files
- **Database Credentials:** Use environment variables for credentials

---

## Contributing Examples

To add new examples:

1. Create `.qmz` file in `examples/` directory
2. Create sample data files (CSV/JSON) if needed
3. Update this README with:
   - Feature description
   - Sample code snippet
   - Data file references
   - Expected output

4. Run test suite to verify compilation
5. Document any new data formats

---

## Contact & Support

For questions about examples:
- Check `ENCRYPTION_QUICKREF.md` for encryption details
- See `DATABASE_COMMANDS_GUIDE.md` for database API
- Review `ARCHITECTURE.md` for system overview

---

**Last Updated:** January 2024  
**eTamil Compiler Version:** 1.0  
**Total Examples:** 7 (5 eTamil .qmz + 2 Rust .rs)  
**Data Files:** 6 (4 CSV + 2 JSON)
