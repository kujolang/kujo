# Example Projects

This directory contains demonstration scripts that showcase Kujo's language features through practical examples. These are **code demonstrations**, not interactive applications - they run once and show you what Kujo can do.

## Projects

### 1. Todo Manager (`todo_manager.kujo`)
A demonstration script showing:
- **Structs**: `Todo` with typed fields  
- **Arrays**: Managing lists of todos
- **Control flow**: Loops and conditionals
- **Field mutation**: `todos[0].done := true`

**What it does:**
- Creates hardcoded todos in the script
- Displays them with checkbox format
- Mutates a field to mark one complete
- Shows the updated list

*Note: Not interactive - no user input yet. Edit the .kujo file to change todos.*

**Run it:**
```bash
cargo run --quiet -- run examples/projects/todo_manager.kujo
```

### 2. Contact Manager (`contact_manager.kujo`)
A demonstration script showcasing:
- **Dictionaries**: Fast contact lookup by name
- **Structs**: Contact data structure
- **String functions**: Search with `contains()`, case handling
- **Error handling**: Try/except for missing data
- **Dict operations**: `keys()`, `has_key()`

**What it does:**
- Creates hardcoded contacts in the script
- Demonstrates dictionary operations
- Shows filtering and searching
- Displays error handling patterns

*Note: Not interactive - demonstrates language features with preset data.*

**Run it:**
```bash
cargo run --quiet -- run examples/projects/contact_manager.kujo
```

### 3. Data Analyzer (`data_analyzer.kujo`) ✨ NEW
A demonstration script showcasing:
- **Array higher-order functions**: `map()`, `filter()`, `reduce()`, `find()`
- **Anonymous functions**: `func(x) { return x * 2 }`
- **Functional programming**: Data transformation and aggregation
- **Statistics**: Calculate sum, average, min, max
- **Advanced filtering**: Multi-step data processing

**What it does:**
- Analyzes dataset with validation
- Filters valid data points
- Calculates comprehensive statistics
- Performs category-based analysis
- Demonstrates function chaining
- Computes performance scores

*Note: Perfect example of functional programming in Kujo!*

**Run it:**
```bash
cargo run --quiet -- run examples/projects/data_analyzer.kujo
```

### 4. Log Parser (`log_parser.kujo`) ✨ NEW
A demonstration script showcasing:
- **File I/O**: `read_file()`, `write_file()`, `read_lines()`
- **String functions**: `split()`, `join()`, `contains()`, `substring()`
- **Data extraction**: Parse structured log entries
- **Pattern matching**: Find specific log patterns
- **Report generation**: Create summary reports

**What it does:**
- Creates sample log file
- Parses log entries into structured data
- Filters by log level (ERROR, WARNING, INFO)
- Searches for specific patterns
- Extracts email addresses
- Generates summary report file

*Note: Demonstrates real-world text processing and file operations.*

**Run it:**
```bash
cargo run --quiet -- run examples/projects/log_parser.kujo
```

### 5. Inventory System (`inventory_system.kujo`) ✨ NEW
A demonstration script showcasing:
- **Complex structs**: Product data with multiple fields
- **Enum patterns**: Product status tracking
- **Advanced filtering**: Multi-condition product searches
- **Calculations**: Inventory valuation and analytics
- **Business logic**: Stock management rules

**What it does:**
- Manages product inventory with 8 sample products
- Tracks stock levels with status indicators
- Filters by category and stock level
- Identifies low stock and out of stock items
- Calculates total inventory value
- Finds most/least expensive products
- Simulates restocking operations

*Note: Great example of a business application structure.*

**Run it:**
```bash
cargo run --quiet -- run examples/projects/inventory_system.kujo
```

---

## 🌐 HTTP-Based Projects (v0.5.0) ✨

### 6. URL Shortener Service (`url_shortener.kujo`)
A complete HTTP API for creating and resolving shortened URLs with SQLite storage.

**Features:**
- **HTTP Server**: Multi-route API on port 3000
- **POST /shorten**: Create short URLs with unique codes
- **GET /:code**: Redirect to original URL (HTTP 302)
- **POST /stats**: View click statistics (requires code in body)
- **GET /count**: Get total URL count (no data exposed)
- **GET /health**: Health check with storage info
- **Validation**: URL format checking
- **Storage**: SQLite database (urls.db) for secure persistent storage
- **Random code generation**: 6-character alphanumeric codes

**What it demonstrates:**
- HTTP server creation and routing
- Dynamic route parameters (/:code)
- POST/GET request handling
- JSON request/response parsing
- SQLite database operations
- URL validation logic
- Statistics tracking
- RESTful API design
- HTTP 302 redirects

**Run it:**
```bash
cargo run --quiet -- run examples/projects/url_shortener.kujo

# In another terminal, test the API:

# Create a short URL
curl -X POST http://localhost:3000/shorten -d '{"url": "https://github.com/kujolang/kujo"}'

# Visit the short URL (redirects to original)
curl -L http://localhost:3000/CODE_FROM_ABOVE

# Get statistics for a code (POST with code in body)
curl -X POST http://localhost:3000/stats -d '{"code": "CODE_FROM_ABOVE"}'

# Get total URL count
curl http://localhost:3000/count

# Health check
curl http://localhost:3000/health
```

---

### 7. Weather Dashboard API (`weather_dashboard.kujo`)
Aggregates weather data with caching and comparison features.

**Features:**
- **HTTP Server**: Weather API on port 4000
- **GET /weather**: Current weather for a city
- **GET /forecast**: 5-day weather forecast
- **POST /compare**: Compare weather across multiple cities
- **GET /cache**: View cached data
- **POST /cache/clear**: Clear cache
- **Caching**: 5-minute cache duration with freshness checking
- **Mock data**: Simulates external weather API calls

**What it demonstrates:**
- HTTP client integration patterns
- Data caching with timestamps
- Data aggregation from multiple sources
- Array filtering and transformations
- Finding max/min values in datasets
- Cache management
- API rate limiting concepts

**Run it:**
```bash
cargo run --quiet -- run examples/projects/weather_dashboard.kujo

# In another terminal, test the API:
curl http://localhost:4000/weather -H "Content-Type: application/json" -d '{"city": "London"}'
curl http://localhost:4000/forecast -H "Content-Type: application/json" -d '{"city": "Paris"}'
curl -X POST http://localhost:4000/compare -H "Content-Type: application/json" -d '{"cities": ["London", "Paris", "Tokyo"]}'
curl http://localhost:4000/cache
```

---

### 8. Simple Blog API (`blog_api.kujo`)
A RESTful API for managing blog posts and comments.

**Features:**
- **HTTP Server**: Blog API on port 5000
- **POST /posts**: Create new blog post
- **GET /posts**: List all posts (with author filtering)
- **GET /post**: Get specific post with comments
- **PUT /posts**: Update existing post
- **DELETE /posts**: Delete post and associated comments
- **POST /comments**: Add comment to post
- **GET /stats**: Get API statistics
- **Validation**: Title, content, and author validation
- **Nested resources**: Comments belong to posts

**What it demonstrates:**
- Full CRUD operations (Create, Read, Update, Delete)
- RESTful API design
- Input validation with error messages
- Nested resource management
- Array filtering and searching
- Data relationships (posts → comments)
- Auto-incrementing IDs
- Timestamp tracking

**Run it:**
```bash
cargo run --quiet -- run examples/projects/blog_api.kujo

# In another terminal, test the API:
curl -X POST http://localhost:5000/posts -H "Content-Type: application/json" -d '{"title": "Hello World", "content": "My first post!", "author": "Alice"}'
curl http://localhost:5000/posts
curl http://localhost:5000/post -H "Content-Type: application/json" -d '{"id": 1}'
curl -X POST http://localhost:5000/comments -H "Content-Type: application/json" -d '{"post_id": 1, "author": "Bob", "content": "Great post!"}'
curl http://localhost:5000/stats
```

---

## Learning Path

**Basic Projects:**
1. **Start with Todo Manager** - Simplest introduction to structs and arrays
2. **Try Contact Manager** - Learn dictionary operations and string functions
3. **Explore Data Analyzer** - Master functional programming with higher-order functions

**Intermediate Projects:**
4. **Study Log Parser** - Learn file I/O and text processing techniques
5. **Examine Inventory System** - See complex business logic and calculations

**HTTP/API Projects (v0.5.0):**
6. **URL Shortener** - Learn HTTP basics, routing, and JSON handling
7. **Weather Dashboard** - Explore data aggregation, caching, and external APIs
8. **Blog API** - Master RESTful design, CRUD operations, and data relationships

## What These Projects Demonstrate

All projects use real-world patterns and demonstrate:
- ✅ Struct design with typed fields
- ✅ Collection manipulation (arrays and dicts)
- ✅ Function organization and modularity
- ✅ Control flow (loops, conditionals)
- ✅ String operations (`contains`, `split`, `join`, `substring`, etc.)
- ✅ Error handling with try/except
- ✅ Dictionary operations (`keys`, `has_key`)
- ✅ Array iteration
- ✅ Array higher-order functions (`map`, `filter`, `reduce`, `find`)
- ✅ Anonymous function expressions
- ✅ File I/O operations
- ✅ Complex data processing and analysis
- ✅ Text parsing and pattern matching
- ✅ **NEW (v0.5.0):** HTTP server creation and routing
- ✅ **NEW (v0.5.0):** RESTful API design
- ✅ **NEW (v0.5.0):** JSON request/response handling
- ✅ **NEW (v0.5.0):** HTTP client usage patterns
- ✅ **NEW (v0.5.0):** Data caching strategies
- ✅ **NEW (v0.5.0):** Input validation
- ✅ **NEW (v0.5.0):** Multi-route applications

## Notes

**HTTP Projects are Live Servers:** The HTTP-based projects (6-8) create actual web servers that stay running until you press Ctrl+C. Test them using `curl` commands from another terminal window.

**Other Projects are Demonstrations:** The non-HTTP scripts (1-5) run once to show language features. They use hardcoded data and don't accept user input. User input functions like `input()` are available but these demos don't use them yet.

These projects use only the standard Kujo features available in v0.5.0+. They demonstrate practical programming patterns that work reliably with the current implementation.

## Tips for Your Own Projects

- Use `[]` for array literals: `[1, 2, 3]`
- Use `{}` for dict literals: `{"key": "value"}`
- Use `#` for comments (not `//`)
- Print supports multiple args: `print("Value:", x)`
- Structs work great for organizing data
- Dicts provide fast lookups by key
- Arrays are perfect for ordered collections
- Field assignment works: `obj.field := value` and `arr[0].field := value`
- Booleans (`true`/`false`) work in if conditions
- If you see compiler warnings about unused code, those are harmless internal warnings
