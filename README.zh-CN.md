<a name="top"></a>

<div align="center">

<h1>SparkFox</h1>

<h3>Local-first AI Agent desktop workstation · Data sovereignty first</h3>

<p>
  <b>Don't rent your second brain to others — your thoughts should not become someone else's nourishment.</b>
</p>

<p>
  <img alt="License: AGPL-3.0-only" src="https://img.shields.io/badge/License-AGPL_3.0_only-FF6F91?style=for-the-badge">
  <img alt="Platform" src="https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-7583B2?style=for-the-badge">
  <img alt="Status" src="https://img.shields.io/badge/Status-pre--1.0-FBBF24?style=for-the-badge">
  <img alt="Version" src="https://img.shields.io/badge/Version-v0.2.28%20(v1.1.0%20WIP)-24C8DB?style=for-the-badge">
</p>

<p>
  <img alt="Built with Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white">
  <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-edition_2024-CE412B?style=flat-square&logo=rust&logoColor=white">
  <img alt="Preact" src="https://img.shields.io/badge/Preact-10-673AB8?style=flat-square&logo=preact&logoColor=white">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white">
  <img alt="Arco Design" src="https://img.shields.io/badge/Arco_Design-2-0FC6C2?style=flat-square">
</p>

<p>
  <a href="README.md">简体中文</a>&nbsp;·&nbsp;<b>English</b>
</p>

<p>
  <a href="#-core-features">🎯 Core Features</a>&nbsp;·&nbsp;
  <a href="#-architecture">🏗️ Architecture</a>&nbsp;·&nbsp;
  <a href="#-getting-started">🚀 Getting Started</a>&nbsp;·&nbsp;
  <a href="#-development">🛠️ Development</a>&nbsp;·&nbsp;
  <a href="#-acknowledgements">💛 Acknowledgements</a>&nbsp;·&nbsp;
  <a href="#-license">⚖️ License</a>
</p>

</div>

---

## 🎯 Project Positioning

**SparkFox** is a local-first AI Agent desktop workstation that integrates the essence of multiple open-source projects, focusing on **memory system optimization** and **Agent orchestration innovation**.

- **Data sovereignty first**: All data resides on your machine. No cloud account, no telemetry, no subscription. The only outbound traffic is the LLM calls you explicitly configure.
- **Apple system style desktop design**: Follows macOS design language, native Tauri 2 shell, no Electron, no Node host.
- **AGPL-3.0-only**: Strong copyleft license, ensuring derivative works remain open-source, guarding the data sovereignty promise.

---

## ✨ Core Features

### 🧠 6-Layer Memory Architecture (Pangu Nebula L0-L5)

Based on Pangu Nebula's 6-layer architecture as the memory system foundation:

- **L0 Raw**: Raw data layer (events / chunks)
- **L1 Indexed**: Index layer (HnswIndex + sqlite-vec dual engine)
- **L2 Associative**: Association layer (event_entity_relation graph)
- **L3 Episodic/Semantic**: Episodic memory + semantic memory (GraphNode)
- **L4 Metacognitive**: Metacognitive layer (thinking process visualization)
- **L5 Procedural**: Procedural memory (skill precipitation)

### 🔍 SAG (Semantic Agentic Graph) Multi-hop Retrieval

v1.1.0 core capability, three-strategy parallel retrieval:

- **multi**: BFS multi-hop expansion (max_hop=3)
- **multi1**: Single-hop pruning (performance priority)
- **hopllm**: LLM-guided multi-hop expansion (semantic priority, fallback to multi1 on failure)
- **R-07 three LIMIT valves**: MAX_HOP=3 / MAX_INTERMEDIATE_ENTITIES=100 / MAX_JOIN_ROWS=10000, preventing graph explosion
- **MULTI 8-step pipeline**: query vectorization → entity extraction → entity retrieval → event retrieval → three-strategy merge → chunk association → Rerank → return SearchResult

### 🐝 Swarm Orchestration (OpenAkita + Pangu Nebula)

Dual-master + swarm worker + persona self-evolution design:

- **Prime Star · Orchestrator**: Task decomposition + DAG scheduling
- **Avatar · Soul Split**: Persona self-evolution
- **Stardust Swarm**: Swarm worker parallel execution
- **Star Soul**: Long-term memory precipitation

### 🎨 Scene Protocol + Thinking Process Visualization (BaiLongma)

- **Scene Protocol**: Scenario-based conversation display
- **ReasoningChainPanel**: 7-step reasoning chain visualization (hop color mapping + via_entities highlighting)
- **CitationDetailDrawer**: Three-level citation (Entity → Event → Chunk)
- **KnowledgeGraphView**: Knowledge graph visualization (@xyflow/react v12 + 11-class coloring + EntityEditDrawer editing)

### 🔒 End-to-End Encryption (E2EE)

- **X25519** key agreement + **HKDF** key derivation + **AES-256-GCM** symmetric encryption
- **Double Ratchet** algorithm forward secrecy (direct implementation, not ratchetx2)

### 🔄 CRDT Multi-device Sync

- **automerge-rs** implementation (not self-developed)
- `AutoCommit::save()` + `load()` + `merge()` full snapshot CRDT merge

---

## 🏗️ Architecture

One Preact frontend + one Rust backend, **two host modes**, same backend running in-process.

| | `sparkfox-desktop` | `sparkfox-web` |
|---|---|---|
| **Shell** | Tauri 2 desktop app | Standalone axum server |
| **Backend** | Embedded in-process, private loopback port | Same backend, in-process |
| **Auth** | Local-trust token injected into webview | Login required by default |
| **Serves** | Native desktop UI + tray + companion windows | API + `/ws` + built-in SPA |

<details>
<summary><b>Repository map</b></summary>

```text
apps/
  desktop/      Tauri 2 shell + desktop-only commands
  web/          Standalone web host (API + SPA)
crates/
  sparkfox/     14 core crates:
                sparkfox-knowledge (SAG retrieval)
                sparkfox-memory (6-layer memory)
                sparkfox-orchestrator (swarm orchestration)
                sparkfox-graph (graph traversal)
                sparkfox-e2ee (end-to-end encryption)
                sparkfox-crdt (multi-device sync)
                sparkfox-embedding (vectorization)
                sparkfox-llm (LLM abstraction)
                sparkfox-chat / sparkfox-agent / sparkfox-hotspot
                sparkfox-monitor / sparkfox-parser / sparkfox-core
  backend/      29 nomifun-* backend crates (naming preserved for compatibility)
  agent/        15 nomi-* / sparkfox-ag-* Agent crates
ui/             Preact + Vite SPA (shared by desktop and web)
docs/           Technical docs, user guides, architecture notes
```

</details>

---

## 🚀 Getting Started

**Prerequisites**

- [Rust](https://rustup.rs) — stable toolchain, edition 2024
- [Bun](https://bun.sh) ≥ 1.3.13
- Recommended on PATH: `node` / `npm` / `npx`, `git`, `ripgrep`

**Desktop app (from source)**

```bash
git clone git@github.com:AQWlala/SparkFox.git
cd SparkFox
bun install

bun run dev      # develop with hot reload
bun run build    # build a desktop bundle for your OS
```

**Web server (self-host)**

```bash
bun run build:ui && bun run serve:web
# serves API + SPA on http://127.0.0.1:8787 (login required)
```

See [`docs/getting-started/installation.md`](docs/getting-started/installation.md) for details.

---

## 🛠️ Development

```bash
bun install        # install dependencies (one-time)
bun run dev        # desktop app development (hot reload)
bun run dev:web    # web host + Vite development
bun run build:ui   # build the SPA
bun run check      # frontend typecheck + i18n + theme + script registry gate
bun run test       # Rust tests (including doctest)
bun run test:fast  # nextest fast Rust tests (daily)
```

### 📦 Desktop Packaging

Each OS has its own command, **a package can only be built on its matching OS**:

| OS | Command | Output |
|---|---|---|
| macOS | `bun run build:mac` | `.dmg` (universal / arm / intel) |
| Windows | `bun run build:win` | `.exe` (NSIS, x64 / arm64) |
| Linux | `bun run build:linux` | `.deb` / `.AppImage` / `.rpm` |

Signing and notarization: `--signed` flag, see [`apps/desktop/signing/README.md`](apps/desktop/signing/README.md).

---

## 📖 Documentation

- [`docs/SparkFox-v1.1.0-规划.md`](docs/SparkFox-v1.1.0-规划.md) — v1.1.0 implementation plan and progress matrix
- [`docs/SparkFox-最终融合蓝图-1.0.md`](docs/SparkFox-最终融合蓝图-1.0.md) — Four-project fusion blueprint
- [`docs/SparkFox-重组优化方案-1.0.md`](docs/SparkFox-重组优化方案-1.0.md) — Reorganization optimization plan
- [`docs/SAG-深度评估与重构方案-1.0.md`](docs/SAG-深度评估与重构方案-1.0.md) — SAG refactoring plan
- [`docs/architecture/`](docs/architecture/) — Technical architecture
- [`docs/getting-started/`](docs/getting-started/) — Installation and first run
- [`docs/guides/`](docs/guides/) — User and operator guides
- [`docs/rfc/`](docs/rfc/) — RFC design documents

---

## 🗺️ Current Version and Roadmap

**Current version**: v0.2.28 (v1.1.0 WIP)

**v1.1.0 progress** (Task 11.x SAG multi-hop retrieval):

- ✅ W4 milestone: 32/32 sub-step completed (5 frontend components integrated into existing pages)
- ✅ Task 11.x: 12/18 sub-step completed (2/3 progress)
  - 11.1.x MULTI 8-step pipeline skeleton + Step1-Step8 real implementation + E2E integration (Recall@5=0.80)
  - 11.2.x multi / multi1 / hopllm three strategies + R-07 three LIMIT valves
  - 11.3.x KnowledgeGraphView entry + 11-class coloring + EntityEditDrawer
  - 11.4.x data contract + @xyflow/react v12 rendering

**Next direction**:

- v1.1.0 wrap-up: 11.4.2 EntityEditDrawer IPC / 11.5.x multi-hop path rendering / 11.6.x hnswlib-rs integration
- v1.2.0+: complete MULTI strategy + dynamic hypergraph
- v2.0.0: downgraded to maintenance release

---

## 💛 Acknowledgements

SparkFox stands on the shoulders of giants, drawing deeply from the following open-source projects (alphabetical order):

| Project | License | Borrowed Content |
|---|---|---|
| **BaiLongma** | MIT | Conversation display method, thinking process visualization, information hotspot tracking; Scene Protocol via clean-room rewrite |
| **NomiFun** | Apache-2.0 | Arco Design interface foundation, functional module design; crate naming via `nomi-*` / `nomifun-*` → `sparkfox-*` rename |
| **OpenAkita** | MIT | Agent menu design, monitoring panel design, organization orchestration model |

**Compliance notes**:

- AGPL-3.0-only is compatible with MIT / Apache-2.0, derivative works remain open-source
- BaiLongma MIT components undergo clean-room rewrite (schema borrowing and field renaming) to maintain AGPL compliance
- API contract fields (`agent_type === 'nomi'`, `nomi_delegate`, `NOMI_SKILL_DIR`, etc.) are preserved to avoid functionality breakage
- See [`NOTICE`](NOTICE) and [`docs/SparkFox-重组优化方案-1.0.md`](docs/SparkFox-重组优化方案-1.0.md)

---

## 🤝 Contributing

- Read [`CONTRIBUTING.md`](CONTRIBUTING.md) to get set up and learn the check ladder
- Follow [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- Report vulnerabilities per [`SECURITY.md`](SECURITY.md)
- Browse [open issues](https://github.com/AQWlala/SparkFox/issues) for a place to start

---

## ⚖️ License

[AGPL-3.0-only](LICENSE) © 2025–2026 SparkFox Contributors.

When the LICENSE file is not present in this repository, AGPL-3.0-only applies (see [`package.json`](package.json) declaration).

Third-party attributions: see [`NOTICE`](NOTICE).

<div align="center">
<br/>
<sub>Local-first · Data sovereignty · AGPL-guarded</sub>
<br/><br/>
<a href="#top">⬆ Back to top</a>
</div>
