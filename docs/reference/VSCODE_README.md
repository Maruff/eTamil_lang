# eTamil Support (VS Code Extension)

Complete VS Code extension for eTamil programming language with syntax highlighting, code snippets, compiler installation, and integrated coding support.

---

## ✨ Features

### Language Support
- **Language ID**: `etamil`
- **File Extension**: `.qmz`
- **Syntax Highlighting**: Tamil + romanized keyword support
- **Comments**: Single-line (`//`) comments
- **Bracket Matching**: Automatic bracket and brace pairing
- **Auto-indentation**: Smart indentation for blocks

### Core Keywords (Tamil / English)
**Control Flow:**
- `எனில்` / `eZil` - If conditional
- `இன்றேல்` / `iZREl` - Else clause
- `சுற்று` / `cuRRu` - Loop

**I/O Operations:**
- `அச்சு` / `accu` - Print output
- `உள்ளிடு` / `uLLitu` - Read input

**File Operations:**
- `கோப்பு_திற` / `kOppu_qiRa` - Open file
- `கோப்பு_மூடு` / `kOppu_mUtu` - Close file
- `கோப்பு_படி` / `kOppu_pati` - Read from file
- `கோப்பு_எழுது` / `kOppu_ezuqu` - Write to file

**CSV Operations:**
- `தரவுரை_படி` / `qaravurY_pati` - Read CSV
- `தரவுரை_எழுது` / `qaravurY_ezuqu` - Write CSV

**Database Operations:**
- `தளம்_இணை` / `qaLam_iNY` - Connect to database
- `தளம்_பிரி` / `qaLam_piri` - Disconnect from database
- `தளம்_வினா` / `qaLam_viZA` - Database query
- `தளம்_செய்` / `qaLam_cey` - Execute command
- `தளம்_செருக` / `qaLam_ceruka` - Insert into database
- `தளம்_புதுப்பி` / `qaLam_puquppi` - Update database
- `தளம்_நீக்கு` / `qaLam_nIkku` - Delete from database

**Data Types:**
- `எண்` / `eN` - Integer/Number
- `பின்னம்` / `piZZam` - Float
- `சொல்` / `col` - String
- `பொது` / `poqu` - Boolean

**Financial & Accounting** ✨ NEW:
- `வரவு` / `varavu` - Credit
- `பற்று` / `paRRu` - Debit
- `இருப்பு` / `iruppu` - Balance
- `வருவாய்` / `varuvAy` - Revenue
- `செலவு` / `celavu` - Expense
- `வருமானம்` / `varumAZam` - Income
- `பயன்` / `payaZ` - Profit
- `இழப்பு` / `izappu` - Loss
- `வரி` / `vari` - Tax
- `வங்கி` / `vawki` - Bank
- `பணம்` / `paNam` - Cash
- ... and 28+ more financial keywords
- `உரை` / `urY` - Text
- `அணி` / `aNi` - Array
- `தேதி` / `qEqi` - Date

### Tamil Letter Equivalents

The extension supports both Tamil script and romanized (Latin character) forms from the `ezuqqu.pdf` standard. Learn the complete mapping in the **[Tamil Letter Equivalents Guide](COMPILER_TAMIL_LETTER_EQUIVALENTS.md)**:

- Tamil alphabet system (consonants, vowels, gemination)
- Romanization rules with detailed examples
- Keyword-by-keyword letter breakdown
- How both forms work identically in code

Example:
```etamil
// Tamil form - same result
எண் வருவாய் = 50000;

// Romanized form - same result  
eN varuvAy = 50000;
```

### Compiler Installation
None needed. **The extension carries the compiler** — the binary for your
platform and the whole nUlakam standard library travel inside the VSIX, so
error checking and **eTamil: Run this file** work as soon as it installs.

To use `etamil` at a shell prompt as well, run **eTamil: Install the compiler
for use outside the editor**: it copies both out to `~/.local`, or to
`%LOCALAPPDATA%\Programs\eTamil` on Windows, and shows you the `PATH` and
`ETAMIL_PATH` lines to add.

### The eTamil fonts
The extension also carries **`ican qamiz`**, the font in which the ASCII letters
carry Tamil glyphs — `c` draws ச, `q` draws த — and **`ican qamiz Smart`**,
which adds 72 characters of the Tamil block and three contextual rules, so one
face sets both a program and the prose about it. Those rules are the font's own
and the compiler does not accept them; `fonts/README.md` says what they are.
Run **eTamil: Install the eTamil font**: it copies both files into your own font
directory, registers them where the platform needs registering, and offers to
set `etamil.eTamilFont` to `ican qamiz Smart`. No administrator rights. Restart
VS Code afterwards — the font list is read when the window starts.

Shipping a font is not installing one: VS Code's editor is not a webview and
has no API that registers a font, so it has to reach the operating system's
font list before anything can name it. That is the whole reason for the
command.

### Finding your way around
**eTamil: Documentation…** opens the manual, the playground, the language tour
or the keyword reference on [etamil.in](https://etamil.in), and every keyword
hover links the reference for that word.

**eTamil: Open an example** opens a copy of one of the twenty-nine example
programs the extension carries — quicker than reading about the language.

### A compiler of your own
Point `etamil.compilerPath` at it. An
explicit path always wins over the carried one. Building that compiler needs
Rust:
```bash
git clone https://github.com/Maruff/eTamil_lang.git
cd etamil_compiler/etamil_compiler
cargo build --release
```

---

## 🚀 Quick Start

### 1. Install the Extension
- Open VS Code Extension Marketplace
- Search for "eTamil Support"
- Click Install
- That is the whole installation: the compiler comes with it

### 2. Create Your First Program
- Create a file: `hello.qmz`
- Type the code below:
```etamil
அச்சு "வணக்கம் உலகம்";
```

### 3. Run Your Program
```bash
etamil --vm hello.qmz
```

Or with piped input:
```bash
echo "test input" | etamil --vm hello.qmz
```

---

## 📝 eTamil Coding Guide

### Basic Structure

#### Variables and Data Types
### Basic Syntax

#### Variables and Output
```etamil
// Declare a variable with integer type
எண் age = 25;

// Print output
அச்சு "Hello World";
அச்சு "வணக்கம் உலகம்";
அச்சு age;
```

#### Reading Input
```etamil
// Declare variable
எண் income;

// Read user input into variable
அச்சு "Enter your income: ";
உள்ளிடு income;
```

#### Conditional Logic (If-Else)
```etamil
// Simple if condition
(age > 18) எனில் {
    அச்சு "You are an adult";
}

// If-Else condition
(age > 18) எனில் {
    அச்சு "You are an adult";
}
இன்றேல் {
    அச்சு "You are a minor";
}
```

#### Loops
```etamil
// Loop from 1 to 10
சுற்று i = 1; i <= 10; i = i + 1; {
    அச்சு i;
}
```

#### File Operations
```etamil
// Read from file
கோப்பு_திற "input.txt";
சொல் data;
கோப்பு_படி "input.txt", data;
அச்சு data;
கோப்பு_மூடு "input.txt";

// Write to file
கோப்பு_திற "output.txt";
கோப்பு_எழுது "output.txt", "Hello from eTamil!";
கோப்பு_மூடு "output.txt";
```

#### CSV Operations
```etamil
// Write CSV data
கோப்பு_திற "data.csv";
தரவுரை_எழுது "data.csv", "name,age,city";
தரவுரை_எழுது "data.csv", "ராஜா,25,Chennai";
கோப்பு_மூடு "data.csv";

// Read CSV data
கோப்பு_திற "data.csv";
தரவுரை_படி "data.csv", data;
அச்சு data;
கோப்பு_மூடு "data.csv";
```

#### Database Operations
```etamil
// Connect to database
தளம்_இணை "sqlite", "mydb.db";

// Query database
தளம்_வினா "SELECT * FROM users";

// Execute command
தளம்_செய் "CREATE TABLE users (id INT, name TEXT)";

// Disconnect
தளம்_பிரி "sqlite";
```

### Complete Example: Simple Calculator

```etamil
// Simple calculator - adds two numbers
// Usage: echo "10 20" | etamil --vm calc.qmz

// Declare variables
எண் a = 10;
எண் b = 20;
எண் sum;

// Calculate sum
sum = a + b;

// Print result
அச்சு "a: ";
அச்சு a;
அச்சு "b: ";
அச்சு b;
அச்சு "sum: ";
அச்சு sum;
```

**Output:**
```
a: 10
b: 20
sum: 30
```

---

## 🔧 Installation Methods

### Linux

#### Option 1: From GitHub (Recommended)
```bash
# Build from source (requires Rust)
git clone https://github.com/Maruff/eTamil_lang.git
cd etamil_compiler/etamil_compiler
cargo build --release
# Binary is at: target/release/etamil or etamil_compiler.exe
```

---

## 📋 Language Keywords Reference

### Tamil Keywords

#### Core Keywords
| Tamil | English | Purpose |
|-------|---------|---------|
| `எனில்` | `eZil` | If condition |
| `இன்றேல்` | `iZREl` | Else clause |
| `சுற்று` | `cuRRu` | Loop |
| `அச்சு` | `accu` | Print output |
| `உள்ளிடு` | `uLLitu` | Read input |

#### Data Types
| Tamil | English | Purpose |
|-------|---------|---------|
| `எண்` | `eN` | Integer/Number |
| `பின்னம்` | `piZZam` | Float |
| `சொல்` | `col` | String |
| `பொது` | `poqu` | Boolean |
| `உரை` | `urY` | Text |
| `அணி` | `aNi` | Array |
| `தேதி` | `qEqi` | Date |

#### File Operations
| Tamil | English | Purpose |
|-------|---------|---------|
| `கோப்பு_திற` | `kOppu_qiRa` | Open file |
| `கோப்பு_மூடு` | `kOppu_mUtu` | Close file |
| `கோப்பு_படி` | `kOppu_pati` | Read from file |
| `கோப்பு_எழுது` | `kOppu_ezuqu` | Write to file |

#### CSV Operations
| Tamil | English | Purpose |
|-------|---------|---------|
| `தரவுரை_படி` | `qaravurY_pati` | Read CSV |
| `தரவுரை_எழுது` | `qaravurY_ezuqu` | Write CSV |

#### Database Operations
| Tamil | English | Purpose |
|-------|---------|---------|
| `தளம்_இணை` | `qaLam_iNY` | Connect to database |
| `தளம்_பிரி` | `qaLam_piri` | Disconnect from database |
| `தளம்_வினா` | `qaLam_viZA` | Query database |
| `தளம்_செய்` | `qaLam_cey` | Execute command |
| `தளம்_செருக` | `qaLam_ceruka` | Insert into database |
| `தளம்_புதுப்பி` | `qaLam_puquppi` | Update database |
| `தளம்_நீக்கு` | `qaLam_nIkku` | Delete from database |

### Operators
| Operator | Meaning | Example |
|----------|---------|---------|
| `=` | Assignment | `x = 5;` |
| `+` | Addition | `a + b` |
| `-` | Subtraction | `a - b` |
| `*` | Multiplication | `a * b` |
| `/` | Division | `a / b` |
| `>` | Greater than | `a > b` |
| `<` | Less than | `a < b` |
| `==` | Equal | `a == b` |
| `!=` | Not equal | `a != b` |
| `>=` | Greater or equal | `a >= b` |
| `<=` | Less or equal | `a <= b` |
| `%` | Percentage | `20%` |

---

## 🎯 Common Workflows

### Basic Script Development
1. Create a `.qmz` file with eTamil code
2. Use the IntelliSense (Ctrl+Space) for keyword suggestions
3. Run with: `etamil --vm yourfile.qmz`

---

## 🐛 Troubleshooting

### Issue: `etamil: command not found`
**Solution**: Ensure installation completed and binary is in PATH
```bash
# Verify installation
which etamil          # Linux/macOS
where etamil.exe      # Windows

# If not found, reinstall via extension command
```

### Issue: UTF-8 Encoding Issues (Windows)
**Solution**: Ensure VS Code is set to UTF-8 encoding
- In VS Code, click the encoding indicator in the status bar (bottom right)
- Select "UTF-8" encoding before saving eTamil files

### Issue: Permission Denied on Linux
**Solution**: Ensure the compiler binary is executable and in PATH
```bash
chmod +x /path/to/etamil
# Verify: etamil --version
```

### Issue: Syntax Errors When Running Code
**Solution**: Check you're using the correct keyword syntax
- Use `அச்சு` for print (not `print`)
- Use `உள்ளிடு` for input (not `input`)
- Use `எனில்` for if (not `if`)

---

## 📖 Additional Resources

- **Official Repository**: https://github.com/Maruff/eTamil_lang
- **Documentation**: See `docs/` folder in repository
- **Examples**: See `examples/` in repository
- **Issue Tracker**: https://github.com/Maruff/eTamil_lang/issues

---

## ✅ Snippets Quick Reference

Type these shortcuts in a `.qmz` file:

**VS Code Snippets**: The extension provides code templates for common constructs.
Press `Ctrl+Space` while typing to see available keywords and autocomplete suggestions.

**Example Workflow**:
1. Type: `அச்சு "Hello"` (or use autocomplete)
2. Press Enter to execute
3. Use Ctrl+Space to discover more keywords

---

## 📞 Support

- Report bugs: https://github.com/Maruff/eTamil_lang/issues
- Ask questions in documentation
- Check examples for reference

---

**Version**: 1.0.0  
**Last Updated**: January 31, 2026  
**Status**: Production Ready
