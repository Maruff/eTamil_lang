# Database Samples Directory

This folder contains all database-related example programs and sample data files for the eTamil compiler.

## Contents Overview

### eTamil Example Programs
- `qaLam_ceyal_paqivu.qmz` - A function of the program called by a SQLite query, once per row
- `student_management.qmz` - Student academic record management with CSV operations
- `inventory_system.qmz` - Product inventory and sales tracking system
- `payroll_system.qmz` - Employee payroll processing with tax calculations

### Rust Demo Programs
- `db_commands_demo.rs` - Comprehensive database and command system demonstration

### Sample Data Files (CSV - Relational)
- `students.csv` - 8 student records with academic scores and GPA
- `payroll.csv` - 8 employee records with salary information
- `inventory.csv` - 10 product records with stock and pricing
- `sales.csv` - 10 sales transaction records with revenue

### Sample Data Files (JSON - NoSQL)
- `students.json` - 4 student documents (MongoDB-compatible) with nested address/guardian data
- `products.json` - 4 product documents with detailed specifications and stock info

## Quick Start

### Run eTamil Examples
```bash
cd /home/esan/ஆவணங்கள்/eTamil/etamil_compiler
cargo run --bin etamil_compiler examples/db_samples/student_management.qmz
cargo run --bin etamil_compiler examples/db_samples/inventory_system.qmz
cargo run --bin etamil_compiler examples/db_samples/payroll_system.qmz
```

### Run Database Demo
```bash
cargo run --example db_commands_demo
```

## Data Relationships

### CSV Data Structure
```
students.csv
└─ Student ID, Name, Class, Scores, GPA

payroll.csv
└─ Employee ID, Name, Department, Salary, Date

inventory.csv
└─ Product ID, Name, Category, Stock, Price, Supplier

sales.csv
└─ Sale ID, Product ID, Quantity, Amount, Date, Salesperson
```

### JSON Data Structure
```
students.json
└─ Document: _id, name, email, phone, class
   └─ Nested: address (street, city, state, zip)
   └─ Nested: guardian (name, relation, phone)

products.json
└─ Document: _id, name, category, price, rating
   └─ Nested: specifications (processor, ram, storage, etc)
   └─ Nested: stock (total, warehouse_a, warehouse_b)
```

## Integration with eTamil Compiler

These samples demonstrate:
1. **File I/O Operations** - Reading and writing CSV files
2. **Database Simulation** - Using CSV data like database tables
3. **Encryption** - Ability to encrypt `.csv` files to `.qrv` format
4. **Data Processing** - Aggregations, calculations, transformations
5. **Financial Operations** - Tax calculations, revenue computation

## See Also

- `../README_EXAMPLES.md` - Complete guide to all examples
- `../../DATA_FILES_GUIDE.md` - Detailed data schema definitions
- `../../DATABASE_COMMANDS_GUIDE.md` - Database API reference

---

**Total Records:** 44 (36 CSV + 8 JSON)  
**Total Size:** ~44 KB  
**Format:** UTF-8 with full Tamil character support
