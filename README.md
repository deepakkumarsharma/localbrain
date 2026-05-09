# Localbrain

**Your codebase, indexed locally, queryable instantly.**

Localbrain is a privacy-first codebase intelligence platform that transforms any repository into a deterministic knowledge graph. Built for developers who value speed and data sovereignty, it runs entirely on your device—no cloud required, no data leaves your machine.

---

## 🏗️ Use Cases

- **Instant Onboarding**: Understand complex, legacy, or massive codebases in minutes.
- **Precision Impact Analysis**: Trace exactly what calls what and see the blast radius of your changes before you make them.
- **Living Documentation**: Automatically generate a human-editable project wiki that stays in sync with your source code.
- **Privacy-First AI Assistance**: Chat with your code using local LLMs for deep context without compromising intellectual property.

## ✨ Why Localbrain?

Localbrain makes your development life easier by removing the friction of repository understanding:

1.  **Zero-Trust Security**: Indexing and analysis happen 100% offline. No telemetry, no third-party uploads.
2.  **Deterministic Grounding**: Unlike generic AI, Localbrain uses a physical knowledge graph (not LLM guesses) to provide answers with exact file and line citations.
3.  **High-Performance Indexing**: Effortlessly handle 10k+ files with incremental hashing that only parses what changed.
4.  **Agent-Ready**: Features a built-in local API that allows your favorite tools (like Cursor or Claude) to leverage high-fidelity codebase data.

## 🛠️ Tech Stack

- **Core Engine**: Rust 1.75 + Tauri 2.0 (High-performance desktop shell)
- **Frontend**: React 18 + TypeScript (Modern, responsive UI)
- **Knowledge Graph**: KuzuDB (Embedded graph database)
- **Search & Retrieval**: Tantivy (Keyword) + sqlite-vec (Semantic/Vector)
- **Analysis**: Tree-sitter (Deterministic code parsing)
- **LLM Runtime**: llama.cpp (Efficient on-device inference)

## 🚀 Getting Started

### Prerequisites

- Node.js 18+
- Rust toolchain
- Tauri dependencies ([Setup Guide](https://tauri.app/v1/guides/getting-started/prerequisites))

### Installation & Development

```bash
# Clone the repository
git clone https://github.com/your-repo/localbrain.git
cd localbrain

# Install dependencies
npm install

# Run in development mode
npm run tauri:dev
```

### Quality Assurance

```bash
npm run typecheck  # Validate TypeScript types
npm run lint       # Ensure code standards
npm run build      # Build production bundle
```

---

## 🛡️ Security & Privacy

Localbrain is local-first by design. All indexing, vector embeddings, and graph traversals are performed on-device. API keys for optional cloud providers are stored exclusively in your operating system's secure keychain.

## 📄 License

This software is licensed under a **Personal Use Only License**. 
You are free to use it for personal, non-commercial purposes. Commercial use, redistribution, or selling of this software is strictly prohibited without prior written permission.

See the `LICENSE` file for details.
