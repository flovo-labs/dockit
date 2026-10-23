# Credits & License

## Original Author & Upstream Foundation

Flovo Dockit is built upon the solid foundation of [**tug**](https://github.com/raeffs/tug), originally authored and architected by [**Raphael Fleischlin**](https://github.com/raeffs).

We express our sincere appreciation to Raphael for:
- The elegant **process seam** architecture (`Invocation` and `Runner`), allowing docker-free test suites and dependable `--dry-run` executions.
- The clean dependency graph algorithms and deterministic topological ordering.
- The principled approach of keeping Compose files standard and runnable with plain `docker compose`.

---

## Evolution to Flovo Dockit

Flovo Dockit builds on this rock-solid base to introduce:
- Comprehensive backup and restore workflows with incremental content-addressed storage (Restic) and database-native dumps.
- Safety and preflight diagnostics (`dockit doctor`, `dockit preflight`, `dockit plan`).
- Automated health-check monitoring, update assistance, and rollback protection.
- Enhanced shell completions, `.dockitignore` configuration with legacy compatibility, and expanded CI/CD integrations.

---

## License

Flovo Dockit is licensed under the permissive [MIT License](https://opensource.org/licenses/MIT).

```text
MIT License

Copyright (c) 2026 Raphael Fleischlin
Copyright (c) 2026 Flovo Authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
