# Use Cases: TraceCraft

## 1. Fast Onboarding for New Developers

- Scenario: A new engineer joins a large Rust project with multiple crates.
- How TraceCraft helps: Instantly generate a high-level call graph and module dependency map. Developers can visualize and understand the codebase in minutes, not days.

## 2. Architecture Review & Refactoring

- Scenario: Architects need to audit the execution paths, dependencies, and unsafe/macro usage across crates before making refactoring decisions.
- How TraceCraft helps: Full AST and execution path analysis, with focused reports on specific Rust constructs (e.g., unsafe blocks, macros, trait objects).

## 3. Security Audit & Code Quality Checks

- Scenario: Teams performing internal audits must trace all sensitive flows, unsafe code, or third-party dependency entry points, without uploading any code to the cloud.
- How TraceCraft helps: Full offline analysis with outputs suitable for compliance reporting and further scripting.

## 4. Teaching, Documentation, and Developer Training

- Scenario: Senior engineers or technical writers want to produce readable flowcharts or summaries for documentation or training.
- How TraceCraft helps: Export call graphs and execution flows in DOT format for easy embedding and explanation. Support for other formats (text, Mermaid, etc.) is planned for future versions.

## 5. Resource-Constrained/Privacy-Sensitive Development

- Scenario: Companies and teams cannot share source code externally or upload to SaaS/online tools.
- How TraceCraft helps: 100% local, memory-efficient analysis, optimized for laptops and developer workstations with strict resource limits.

