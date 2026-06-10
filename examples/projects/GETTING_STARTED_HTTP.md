# Getting Started with Kujo HTTP Projects

This guide will help you run and explore the HTTP-based sample projects in Kujo.

## Prerequisites

1. Install Rust and Cargo (see [INSTALLATION.md](../../INSTALLATION.md))
2. Clone the Kujo repository
3. Build the project: `cargo build --release`

## Quick Start

### 1. URL Shortener Service

Create and manage shortened URLs with statistics tracking.

**Start the server:**
```bash
cargo run --quiet -- run examples/projects/url_shortener.kujo
```

**Test it** (in another terminal):
```bash
# Create a short URL
curl -X POST http://localhost:3000/shorten \
  -d '{"url": "https://github.com/kujolang/kujo"}'

# Response:
# {"code":"abc123","short_url":"http://localhost:3000/abc123","original_url":"https://github.com/kujolang/kujo"}

# Get redirect info
curl http://localhost:3000/redirect?code=abc123

# List all URLs
curl http://localhost:3000/list

# Get statistics
curl http://localhost:3000/stats -d '{"code": "abc123"}'

# Health check
curl http://localhost:3000/health
```

---

### 2. Weather Dashboard API

Aggregate weather data with caching and multi-city comparison.

**Start the server:**
```bash
cargo run --quiet -- run examples/projects/weather_dashboard.kujo
```

**Test it** (in another terminal):
```bash
# Get current weather
curl http://localhost:4000/weather \
  -d '{"city": "London"}'

# Response:
# {"city":"London","temperature":18.0,"condition":"Cloudy","humidity":65,"wind_speed":15,"summary":"☀️ Pleasant - Cloudy","cached":false}

# Get 5-day forecast
curl http://localhost:4000/forecast \
  -d '{"city": "Paris"}'

# Compare multiple cities
curl -X POST http://localhost:4000/compare \
  -d '{"cities": ["London", "Paris", "Tokyo"]}'

# View cache
curl http://localhost:4000/cache

# Clear cache
curl -X POST http://localhost:4000/cache/clear

# API info
curl http://localhost:4000/
```

---

### 3. Simple Blog API

Full-featured blog API with posts and comments.

**Start the server:**
```bash
cargo run --quiet -- run examples/projects/blog_api.kujo
```

**Test it** (in another terminal):
```bash
# Create a post
curl -X POST http://localhost:5000/posts \
  -d '{"title": "My First Post", "content": "Hello, World! This is my first blog post.", "author": "Alice"}'

# Response:
# {"id":1,"title":"My First Post","content":"Hello, World!...","author":"Alice","created_at":...}

# List all posts
curl http://localhost:5000/posts

# Get specific post
curl http://localhost:5000/post -d '{"id": 1}'

# Update a post
curl -X PUT http://localhost:5000/posts \
  -d '{"id": 1, "title": "Updated Title"}'

# Add a comment
curl -X POST http://localhost:5000/comments \
  -d '{"post_id": 1, "author": "Bob", "content": "Great post!"}'

# Filter posts by author
curl http://localhost:5000/posts \
  -d '{"author": "Alice"}'

# Get stats
curl http://localhost:5000/stats

# Delete a post
curl -X DELETE http://localhost:5000/posts -d '{"id": 1}'
```

---

## Tips & Tricks

### Using HTTPie (Alternative to curl)

If you have [HTTPie](https://httpie.io/) installed, the commands are even simpler:

```bash
# URL Shortener
http POST :3000/shorten url=https://example.com

# Weather Dashboard
http :4000/weather city=London

# Blog API
http POST :5000/posts title="Hello" content="World" author="Alice"
```

### Pretty-Print JSON Responses

Add `| python -m json.tool` to any curl command:

```bash
curl http://localhost:3000/list | python -m json.tool
```

Or use `jq` if installed:

```bash
curl http://localhost:3000/list | jq
```

### Running Multiple Servers

You can run all three projects simultaneously in different terminals since they use different ports:
- URL Shortener: `localhost:3000`
- Weather Dashboard: `localhost:4000`
- Blog API: `localhost:5000`

### Stopping Servers

Press `Ctrl+C` in the terminal where the server is running.

---

## Exploring the Code

Each project is a single `.kujo` file with extensive comments. Open them to see:

- **URL Shortener** (`url_shortener.kujo`): 
  - Code generation algorithms
  - URL validation
  - Dictionary-based storage
  - Statistics tracking

- **Weather Dashboard** (`weather_dashboard.kujo`):
  - Data caching with timestamps
  - Cache freshness checks
  - Multi-source data aggregation
  - Array operations for comparisons

- **Blog API** (`blog_api.kujo`):
  - Full CRUD operations
  - Input validation
  - Nested resources (posts → comments)
  - Auto-incrementing IDs
  - Array filtering and searching

---

## Common Issues

### Port Already in Use

If you see "Address already in use" error:
1. Check if another instance is running: `lsof -i :3000`
2. Kill the process: `kill <PID>`
3. Or use a different port (edit the `.kujo` file)

### Type Checking Warnings

You may see warnings like:
```
Type checking warnings:
  Undefined Function: Undefined function 'http_server'
```

These are safe to ignore - the functions exist and work correctly.

### Connection Refused

If `curl` says "Connection refused":
1. Make sure the server is running
2. Check the correct port number
3. Try `http://127.0.0.1:PORT` instead of `localhost`

---

## Next Steps

1. **Modify the projects**: Change ports, add new routes, implement features
2. **Combine features**: Use HTTP client in one project to call another
3. **Build your own**: Use these as templates for your own HTTP services
4. **Explore examples**: Check `/examples/http_*.kujo` for more patterns

## Resources

- [Kujo Documentation](../../README.md)
- [HTTP Examples](../)
- [CHANGELOG](../../CHANGELOG.md)
- [ROADMAP](../../ROADMAP.md)

---

**Happy coding with Kujo! 🐾**
