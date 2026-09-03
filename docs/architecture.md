# Power Agent Architecture

## 1. Overview

Power Agent is a **Context-Aware AI Agent for PC Power Management** running on Windows PCs.

Its goal is to select an appropriate power-saving strategy based on:

* Real-time system telemetry
* Current user context
* Natural-language user intent
* Relevant historical context
* Available system capabilities

Power Agent aims to reduce power consumption while preserving the **user experience (UX)** and **Quality of Service (QoS)** required by the user as much as possible.

Power Agent is not merely a system monitor, nor is it simply a UI wrapper around Windows power settings.

Core concept:

```text
Current State
+
User Intent / Context
+
Available Legal Actions
        ↓
Decision
        ↓
Policy / Safety Check
        ↓
Execution
        ↓
Observe Result
```

---

## 2. High-Level Architecture

```text
User Request                  Proactive Trigger
     │                               │
     └──────────────┬────────────────┘
                    ↓
           Telemetry Collection
                    ↓
         Context / Intent Layer
                    ↓
       Relevant History Retrieval
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
                    ↓
        Feedback / History Update
```

The LLM is not required for every decision.

Simple deterministic decisions may be handled directly by the Decision Engine.

---

## 3. Telemetry Layer

The Telemetry Layer is responsible for collecting the actual system state of the PC.

### 3.1 Battery Telemetry

The current implementation uses the Windows API:

`GetSystemPowerStatus()`

Currently available data includes:

* Battery percentage
* AC / battery power source
* Charging state
* Battery remaining time, when available

The current test PC may not provide `BatteryLifeTime`.

This is considered an acceptable system condition and should not be treated as an application error.

In the future, remaining battery time may be estimated based on telemetry history and observed discharge behavior.

### 3.2 Display Telemetry

Windows Display APIs are currently being introduced to retrieve:

* Current refresh rate

The initial implementation primarily targets the primary display.

Next step:

* Enumerate supported refresh rates under the current display configuration

### 3.3 Future Telemetry

Potential future telemetry includes:

* Battery discharge rate
* Estimated remaining time
* CPU utilization
* GPU utilization
* NPU utilization
* Foreground application
* Background application activity
* Camera state
* Audio state
* Network activity
* Display state
* Thermal state
* AC-to-battery transition
* User activity / idle state

---

## 4. Context Layer

Context describes:

> What is the user currently doing?

The initial version is expected to support at least:

* Normal Mode
* Meeting Mode

Future contexts may include:

* Office Work
* Travel
* Gaming
* Media
* Idle

Context and Power Condition should be modeled separately.

For example:

```text
Context:
Meeting Mode

Power Condition:
Low Battery
```

The actual scenario that must be handled is the combination of the two.

For example:

```text
Meeting Mode
+
Battery Insufficient
        ↓
Save power while preserving meeting-critical UX
```

---

## 5. User Intent Layer

In the future, users may describe their requirements through natural language.

For example:

> I still have a two-hour meeting coming up, my battery is running low, and I do not want Teams to be affected.

A Local LLM may convert this natural-language request into structured intent, for example:

```json
{
  "mode": "MEETING",
  "required_duration_minutes": 120,
  "battery_priority": "high",
  "camera_required": true,
  "audio_required": true
}
```

The LLM is primarily responsible for:

* Natural Language Understanding
* Intent Extraction
* Context Understanding

The LLM must not directly control Windows APIs or BEM APIs.

---

## 6. Decision Engine

The Decision Engine is the primary strategy-selection layer of Power Agent.

Inputs may include:

```text
Telemetry
+
Structured User Intent
+
Current Context
+
Relevant History
+
Available Legal Actions
```

The output is a candidate power strategy.

For example:

```text
Battery = 25%

Current Refresh Rate = 120 Hz

Supported Refresh Rates = [60, 120]

Context = Meeting

Required Duration = 120 minutes
```

The Decision Engine may produce:

```text
Candidate Action:

CHANGE_REFRESH_RATE_TO_60HZ
```

However, a candidate action must not be executed directly.

It must first pass through the Policy / Safety Layer.

---

## 7. Available Legal Actions

Power Agent should not allow the LLM to freely generate arbitrary system actions.

The Capability Layer should first provide the actions that are actually supported by the current system.

For example:

```text
Current Refresh Rate:
120 Hz

Supported Refresh Rates:
60 Hz
120 Hz
```

The legal actions may therefore include:

```text
KEEP_120HZ
CHANGE_TO_60HZ
```

The following must not be generated:

```text
CHANGE_TO_47HZ
```

Therefore, an important execution flow is:

```text
Observe Current State
        ↓
Discover Available Actions
        ↓
Decision
```

This is a key design principle of the current MVP.

---

## 8. Policy / Safety Layer

After the Decision Engine generates a candidate action, the Policy / Safety Layer determines whether the action may be executed.

Evaluation factors may include:

* Whether the action is valid and supported
* Whether the action is reversible
* Current user context
* Task interruption risk
* UX degradation
* User preferences
* Decision confidence
* Whether user approval is required

Action risk should vary depending on context.

For example, disabling the camera:

```text
Normal Mode
→ Potentially low impact

Meeting Mode
→ Potentially severe impact
```

Conceptually:

```text
Action Risk =
f(
    Action,
    Current Context,
    User Preference
)
```

---

## 9. Capability / API Layer

The Capability Layer is responsible for actual communication with Windows and hardware.

The Decision Engine must not directly operate low-level APIs.

### 9.1 Battery

Currently uses:

`GetSystemPowerStatus()`

### 9.2 Display

Windows Display APIs may currently or eventually support:

* Read current refresh rate
* Enumerate supported refresh rates
* Change refresh rate

### 9.3 Future BEM / Component Capabilities

Future capabilities may include:

* LCD Panel EPSM
* OLED Panel ELP
* Refresh rate control
* Camera MEP
* Audio power-related capabilities
* Intel Graphics Control Library
* Other BEM/component interfaces

Conceptually:

```text
Power Agent
= When / Why / What to change

BEM / Component APIs
= How to change it
```

---

## 10. History and Personalization

In the future, Power Agent may store:

* Telemetry summaries
* Context
* Previous recommendations
* User acceptance / rejection
* Observed results

History should not rely on a single fixed time window.

Multiple time scales are expected:

```text
Recent Telemetry
→ current session / minutes

Session Summaries
→ repeated daily patterns

Long-Term Summaries
→ longer-term preferences
```

Relevant history should consider both:

* Context similarity
* Recency

Conceptually:

```text
History Weight =
α × Context Similarity
+
β × Recency
```

For example:

A user may normally work in the office while connected to AC power, but use the PC on battery while traveling on a particular day.

In this case, a large amount of historical Office + AC data should not outweigh the current battery-powered session.

---

## 11. Power Agent Self-Optimization

A future research direction may investigate which execution device should run Power Agent's own AI workload:

* CPU
* GPU
* NPU

A possible objective is:

```text
Minimize Agent Energy Cost

subject to:

Latency <= Required Latency
```

This applies only to the AI workload generated by Power Agent itself.

Power Agent should not control or migrate the CPU / GPU / NPU placement of arbitrary third-party applications.

---

## 12. Initial MVP Scenario

The initial MVP considers two Context states:

* Normal Mode
* Meeting Mode

and two Power Conditions:

* Battery Sufficient
* Battery Insufficient

This produces the following scenarios:

| Context | Power State  | Expected Behavior                                      |
| ------- | ------------ | ------------------------------------------------------ |
| Normal  | Sufficient   | Keep current settings                                  |
| Normal  | Insufficient | Apply more aggressive power-saving measures            |
| Meeting | Sufficient   | Preserve meeting UX                                    |
| Meeting | Insufficient | Save power while preserving meeting-critical functions |

The key concept that this MVP should validate is:

> The same battery state may result in different power strategies depending on the current user context.

---

## 13. Current Development Status

Current development status:

```text
Tauri Project                     Done

Rust Backend                      Done

Windows Native API                Done

Battery Percentage                Done

AC / Battery Detection            Done

Charging State                    Done

Current Refresh Rate              Implemented / Validating

Supported Refresh Rates           Current Task

Natural Language Input            Not Started

Local LLM                         Not Started

Intent Extraction                 Not Started

Decision Engine                   Not Started

Policy / Safety Layer             Not Started

Refresh Rate Action               Not Started

History / Personalization         Not Started
```

Battery telemetry currently uses the Windows `GetSystemPowerStatus()` API.

Display telemetry has started using Windows Display APIs to retrieve the current refresh rate.

The actual repository implementation remains the source of truth for current development status.

---

## 14. Development Roadmap

The current development sequence is:

```text
Observe
   ↓
Discover Available Actions
   ↓
Add Context / Intent
   ↓
Decision
   ↓
Safety Check
   ↓
User Approval
   ↓
Act
   ↓
Observe Again
   ↓
Personalization
```

The current priority is to establish a minimal, verifiable end-to-end pipeline before progressively expanding system capabilities.

In the short term, avoid integrating all of the following at once:

```text
AI
+
Telemetry
+
Windows APIs
+
Decision Engine
+
Hardware Actions
```

Each layer should first be validated independently and then integrated incrementally.
