# Power Agent - Agent Development Instructions

## Project Context

This project is a **Context-Aware AI Agent for PC Power Management** running on Windows PCs.

The goal of Power Agent is to select an appropriate power-saving strategy based on:

* Real-time system telemetry
* User context
* Natural-language user intent
* Relevant historical context
* Available system capabilities

The agent should reduce power consumption while preserving the **user experience (UX)** and **Quality of Service (QoS)** required by the user as much as possible.

For detailed system design, refer to:

`docs/architecture.md`

---

## Core Architecture Rule

Development should preserve the following architectural separation:

```text
Telemetry Collector
        ↓
Context / Intent Layer
        ↓
Decision Engine
        ↓
Policy / Safety Layer
        ↓
Capability / API Layer
        ↓
Hardware
        ↓
Observe Result
```

Do not unnecessarily mix responsibilities across layers.

---

## AI Responsibility

The Local LLM will primarily be responsible for:

* Natural Language Understanding
* User Intent Extraction
* Context Understanding

The LLM must **not directly invoke**:

* Windows APIs
* BEM APIs
* Hardware control APIs
* Other low-level execution interfaces

Simple deterministic logic should not be delegated to the LLM merely for the sake of using AI.

---

## Decision Responsibility

The Decision Engine should select a candidate power strategy based on:

* Telemetry
* Structured user intent
* Current context
* Relevant history
* Available legal actions

Conceptually:

```text
Telemetry
+
Intent
+
Context
+
History
+
Available Actions
        ↓
Decision Engine
        ↓
Candidate Strategy
```

---

## Action Safety

Before making a decision, Power Agent must first determine the **Available Legal Actions** that are actually supported by the system.

For example, if the display supports:

* 60 Hz
* 120 Hz

then valid actions may include:

```text
KEEP_CURRENT_REFRESH_RATE
CHANGE_REFRESH_RATE_TO_60HZ
```

The AI must not generate settings that have not been confirmed as supported by the system, such as:

```text
CHANGE_REFRESH_RATE_TO_47HZ
```

Before execution, all actual actions should pass through the **Policy / Safety Layer**.

---

## Policy / Safety Principle

The Policy / Safety Layer should consider:

* Whether the action is supported and valid on the current system
* Whether the action is reversible
* Whether the action may interrupt the user's current activity
* UX impact
* Current user context
* User preferences
* Whether user approval is required

Action risk should be evaluated **contextually rather than assigned as a fixed value**.

For example:

```text
Camera Off + Normal Mode
→ Potentially low risk

Camera Off + Meeting Mode
→ High risk
```

---

## Capability Layer

The Capability / API Layer is responsible for actual interaction with Windows and hardware.

It may include:

* Windows Native APIs
* BEM-related capabilities
* Intel Graphics Control Library
* Display APIs
* Camera APIs
* Audio APIs

The Decision Engine must not directly invoke low-level APIs.

Conceptually:

```text
Power Agent
= When / Why / What to change

Capability / BEM APIs
= How to change it
```

---

## Technology Stack

The current primary technology stack includes:

* Windows
* Tauri
* React
* TypeScript
* Rust
* Windows Native APIs

Future components may include:

* Local LLM
* LM Studio or another local inference runtime
* BEM-related component APIs
* Intel Graphics Control Library
* Camera APIs
* Audio APIs

Do not introduce future components prematurely unless explicitly required by the current task.

---

## Development Rules

Before modifying code:

1. Inspect the repository and relevant files first.
2. Understand the current implementation before making changes.
3. Treat the actual repository code as the source of truth.
4. If documentation and implementation differ, explicitly identify the discrepancy.
5. Do not reimplement functionality that is already working correctly.

When modifying code:

1. Prefer the smallest necessary change.
2. Preserve existing functionality.
3. Follow the existing coding style and architecture.
4. Do not expand the scope of the current task without explicit instruction.
5. Avoid overengineering for future requirements.
6. Handle Windows API failures safely; failures must not cause the application to crash.
7. Maintain appropriate separation between Telemetry, Decision, Policy, and Execution.
8. Do not guess when Windows API behavior is uncertain.
9. Prefer official APIs or implementations whose behavior can be verified.
10. Do not allow the LLM to directly control low-level APIs.

After modifying code:

1. Run appropriate Rust compile/check commands.
2. Run TypeScript/frontend checks.
3. When appropriate, perform Tauri runtime/build validation.
4. Report the actual validation results.
5. Clearly state which files were modified and why.

---

## Development Strategy

The project currently follows an **incremental implementation** strategy.

Development should follow this loop:

```text
Observe
   ↓
Discover Available Actions
   ↓
Decide
   ↓
Safety Check
   ↓
Act
   ↓
Observe Again
```

Validate each layer independently before integrating them incrementally.

Avoid introducing and debugging all of the following simultaneously:

* AI
* Telemetry
* Windows APIs
* Decision Engine
* Hardware Actions

unless explicitly required by the current task.

---

## Scope Rule

At the beginning of each new task:

1. Read this document.
2. Read `docs/architecture.md`.
3. Inspect the current repository.
4. Implement only the scope explicitly requested by the user.

If you discover other issues or potential improvements outside the current scope:

* You may point them out.
* Do not implement them without explicit instruction.
