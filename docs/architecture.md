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

Windows Display APIs currently retrieve the following information for the primary display:

* Current refresh rate
* Current display mode
* Driver-reported display mode candidates

Each driver-reported candidate currently preserves:

* Width
* Height
* Refresh rate
* Bits per pixel
* Display flags
* Display orientation, when reported
* Fixed-output behavior, when reported
* `DEVMODE` field-validity flags

Only candidates matching the current resolution are retained. These candidates are GDI display modes reported by Windows and the display driver. They are not necessarily exposed by Windows Settings, validated as controllable by Power Agent, or approved as Available Legal Actions.

The current implementation also queries the active Windows Connecting and Configuring Displays (CCD) configuration as a read-only identity and topology mapping foundation. The primary GDI source name is matched to CCD source information returned by `DisplayConfigGetDeviceInfo`, producing the active path identity needed by future validation:

```text
Primary GDI Display Name (for example, \\.\DISPLAY1)
        ↓ exact source-name mapping
CCD Adapter LUID + Source ID
        ↓ current active path
CCD Target ID + Current Path/Mode References
```

The mapping does not use the numeric suffix of `DISPLAYx` or assume that GDI and CCD enumeration indexes correspond. A source may map to more than one active path in a clone topology, so the backend represents matched paths as a collection.

CCD path refresh information is preserved as numerator/denominator rational values. The path refresh rate and target-mode vertical sync rate are represented separately because virtual or dynamic refresh-rate configurations can give them different meanings.

This CCD snapshot describes the current active configuration only. It does not prove that any driver-reported candidate can be applied, and it does not promote a candidate to a Controllable Action or an Available Legal Action.

The backend can also perform an explicit, validation-only sanity check of the latest complete active CCD snapshot. It queries all active paths and their mode table again, then calls `SetDisplayConfig` with:

```text
SDC_VALIDATE
+
SDC_USE_SUPPLIED_DISPLAY_CONFIG
+
matching virtual-mode awareness flags
```

This operation does not include `SDC_APPLY`, `SDC_ALLOW_CHANGES`, or database-saving flags. It tests whether Power Agent can construct a coherent validation payload for the current configuration without applying a display change. Failure is reported separately and does not invalidate the existing GDI telemetry or CCD identity mapping.

Current CCD Configuration Validation is an infrastructure sanity check. Success does not validate any driver-reported refresh-rate candidate and does not derive a Controllable Action or Available Legal Action.

Candidate-validation research found an important boundary between the GDI and CCD representations. A GDI `DEVMODE` candidate contains the logical display-mode fields reported by `EnumDisplaySettingsExW`, including an integer `dmDisplayFrequency`, but it does not contain the complete CCD target signal timing. A `DISPLAYCONFIG_TARGET_MODE` additionally requires values such as pixel rate, rational horizontal and vertical sync frequencies, active size, total size, and scan-line ordering. These missing values must not be guessed or derived from assumed blanking intervals.

The public CCD APIs used here do not expose the complete timing for every arbitrary non-current GDI candidate. `QueryDisplayConfig` returns the current configuration, while `DisplayConfigGetDeviceInfo(DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE)` returns the preferred target mode rather than all alternate target timings. Therefore, the current 40 Hz and 48 Hz entries remain driver-reported candidates; neither has an exact CCD target-mode identity yet.

Next step:

* Validate candidates with the intended execution API without applying them
* Derive Controllable Refresh-Rate Actions
* Apply Policy / Safety constraints before exposing Available Legal Actions

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

Driver-Reported Refresh Rate Candidates = [60, 120]

Validated Controllable Refresh Rates = [60, 120]

Available Legal Actions = [KEEP_120HZ, CHANGE_TO_60HZ]

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

Driver-reported display modes are capability-discovery candidates. They must not be treated directly as Controllable Actions or Available Legal Actions.

For example:

```text
Current Refresh Rate:
120 Hz

Driver-Reported Refresh Rate Candidates:
60 Hz
120 Hz
```

This information alone is not sufficient to produce `CHANGE_TO_60HZ`. The candidate must first be validated with the intended execution API under the current display configuration. A successful validation may produce:

```text
Controllable Refresh-Rate Actions:
KEEP_120HZ
CHANGE_TO_60HZ
```

After Policy / Safety constraints are applied, the resulting legal actions may include:

```text
Available Legal Actions:
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
Enumerate Driver-Reported Candidates
        ↓
Map Current GDI Source to Active CCD Path
        ↓
Validate Candidate With Intended Execution API
        ↓
Derive Controllable Actions
        ↓
Apply Policy / Safety Constraints
        ↓
Expose Available Legal Actions
        ↓
Decision
```

The Policy / Safety step shown here is pre-decision action-admissibility filtering. After the Decision Engine selects a candidate action, the selected action must still pass the final Policy / Safety check before execution.

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
* Enumerate driver-reported display mode candidates
* Validate whether a candidate is controllable
* Change refresh rate after policy approval

The current implementation reads current state, enumerates driver-reported candidates, builds a read-only GDI-to-CCD active-path mapping, and can validate the latest unchanged active configuration with `SetDisplayConfig` in `SDC_VALIDATE` mode. It does not implement candidate controllability validation, `SDC_APPLY`, or any refresh-rate modification.

Display capability discovery currently uses two related but distinct Windows API views:

* GDI APIs identify the primary display view, read its current `DEVMODE`, and enumerate driver-reported mode candidates for the current resolution.
* CCD APIs query the current active configuration and map the GDI view name to adapter, source, target, path, and mode-table identity.

Conceptually:

```text
GDI Current Mode / Driver-Reported Candidates
        +
GDI Display Name → CCD Source → Active CCD Path / Target
        ↓
Identity foundation for future candidate validation
```

The CCD query is read-only and is supplementary to existing GDI telemetry. A CCD query or mapping failure is reported without turning the existing GDI current-mode and candidate data into a failure. Mode-table indexes are snapshot-local and must not be treated as persistent identities across later CCD queries.

Current configuration validation always performs a new `QueryDisplayConfig` call and passes the complete native active path and mode arrays from that same snapshot to `SetDisplayConfig`. It preserves path priority and the full multi-display topology; it does not validate only the primary display path. Query awareness flags are paired with the corresponding `SDC_VIRTUAL_MODE_AWARE` and `SDC_VIRTUAL_REFRESH_RATE_AWARE` modifiers when applicable.

#### Candidate-validation evidence levels

Candidate validation must preserve the distinction between the following evidence levels:

1. **Driver-reported candidate:** `EnumDisplaySettingsExW` reports a `DEVMODE` for the current resolution. This is discovery evidence only.
2. **GDI driver preflight:** `ChangeDisplaySettingsExW` with `CDS_TEST` can test the exact enumerated `DEVMODE` without applying it. Success is evidence that the GDI/driver path accepts that graphics mode, but it does not prove compatibility with the complete current CCD topology or identify an exact CCD target timing.
3. **OS-resolved CCD request validation:** `SetDisplayConfig` permits a target mode to be omitted and can use best-mode logic to supply missing mode information. A validation-only request may therefore test a nominal path refresh-rate request without constructing target timing. Success means Windows found a compatible configuration for that request; it does not prove that the resolved mode is a particular GDI candidate or a caller-known exact target mode.
4. **Exact execution validation:** A candidate is exact only when the intended execution mechanism supplies or identifies the complete requested mode without guessed timing, and validates that same execution representation against a fresh full-topology snapshot.

`SDC_ALLOW_CHANGES` must not be used to claim exact candidate controllability. That flag permits Windows to modify supplied source and target mode information to create a functional path set, so success provides weaker evidence than validation of the unchanged request.

There is no documented public API contract exposing the exact refresh-rate option model used by the Windows Settings dropdown together with complete non-current CCD target timings. Settings visibility can be supporting UX evidence, but it is not itself a machine-readable validation result. For example, a driver-reported 48 Hz candidate that is absent from Settings must remain a candidate rather than being silently removed or promoted.

The recommended validation pipeline is:

```text
Driver-Reported GDI Candidate
        -> GDI CDS_TEST Preflight
        -> Fresh Full CCD Topology Snapshot
        -> Validation With the Intended Execution Contract
             - OS-resolved nominal refresh request, or
             - execution-specific exact mode identity/timing
        -> Controllable Action With an Explicit Guarantee Level
        -> Policy / Safety
        -> Available Legal Action
```

An OS-resolved action, if adopted later, must be represented according to its actual contract, for example `REQUEST_NOMINAL_40HZ_WITH_OS_MODE_RESOLUTION`. It must not be mislabeled as applying a known exact 40 Hz CCD target mode. Exact mode control may require an execution-specific or vendor/OEM API that can enumerate and validate the same mode representation it will later apply.

The separation is:

```text
Current Active CCD Configuration
        ↓ validation-only infrastructure sanity check
Current CCD Configuration Validation

Driver-Reported Candidate
        ↓ not implemented yet
Candidate Validation
        ↓
Controllable Action
        ↓
Policy / Safety
        ↓
Available Legal Action
```

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

Driver-Reported Refresh Rate
Candidates                        Implemented

GDI-to-CCD Active Path Mapping    Implemented

Current CCD Configuration
Validation                        Implemented

Candidate Validation Research     Complete

GDI Candidate Preflight           Not Started

Exact CCD Candidate Validation    Not Implemented; public API timing gap

Controllable Action Derivation    Not Started

Available Legal Action Derivation Not Started

Natural Language Input            Not Started

Local LLM                         Not Started

Intent Extraction                 Not Started

Decision Engine                   Not Started

Policy / Safety Layer             Not Started

Refresh Rate Action               Not Started

History / Personalization         Not Started
```

Battery telemetry currently uses the Windows `GetSystemPowerStatus()` API.

Display telemetry retrieves the current refresh rate and preserves the necessary metadata for driver-reported display mode candidates at the current resolution.

These candidates have not passed candidate validation, have not produced Controllable Actions, and have not been converted into Available Legal Actions.

The actual repository implementation remains the source of truth for current development status.

---

## 14. Development Roadmap

The current development sequence is:

```text
Observe
   ↓
Enumerate Driver-Reported Candidates
   ↓
Map Current GDI Source to Active CCD Path
   ↓
Validate Current CCD Configuration
   ↓
Validate Candidates
   ↓
Derive Controllable Actions
   ↓
Apply Policy Constraints
   ↓
Expose Available Legal Actions
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
