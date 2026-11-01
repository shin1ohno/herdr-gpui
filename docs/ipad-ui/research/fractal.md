# fractal + Codex (gpt-6.1-sol) for the herdr-gpui iPad UI work — findings and runbook

Scope: execution tooling only (no screens). Source read: fractal 1.3.0 in the plugin cache
`/Users/shin1ohno/.claude/plugins/cache/plasma/fractal/1.3.0` (abbreviated `F/` below), the operator
notes `/Users/shin1ohno/.claude/docs/fractal-nodes.md`, and the herdr-gpui repo at
`/Users/shin1ohno/ManagedProjects/herdr-gpui` (abbreviated `R/`). Probes were read-only; nothing was
installed or written outside this file.

## 0. Bottom line

1. fractal can drive Codex on this repo, but five defaults must be overridden or every node will
   misbehave: push target (`origin` = penso upstream), Codex model (node `CODEX_HOME` does not
   inherit `~/.codex/config.toml`), `CARGO_TARGET_DIR` (does not reach a warm tmux server), the
   root-branch slash ban, and the launch sandbox (a tmux server started from Claude's sandboxed
   Bash would confine every node).
2. Codex nodes run `danger-full-access` (seeded), so the earlier `workspace-write` failures
   (Unix-socket tests, `git add` in worktrees, simctl) do not occur under fractal — and there is no
   filesystem/network boundary either. Guard pushes with `--local` plus a git-side guard.
3. Disk is the binding constraint: 5.0 GiB free at the last probe (5.4 GiB 30 minutes earlier),
   `target/shared` already 6.4 GiB. Nodes must not run `just ci`/full `cargo test --workspace`;
   at most one node building at a time.
4. `just` is not installed on this Mac (`command -v just` empty; not in mise installs). Every gate
   in HANDOFF §0/§4 must be invoked through the underlying commands.

## 1. Install

- Not installed: `command -v fractal wiki` empty, `uv tool list` = "No tools installed",
  `python3 -c "import fractal"` fails, `~/.fractal` absent.
- Available: uv 0.6.10 (`~/.local/bin/uv`), Python 3.12.2 (pyenv), tmux 3.7c, codex-cli 0.166.0;
  pipx absent.
- Package: `plasma-fractal` (alias `fractal`), requires Python >=3.12,<3.15, depends on
  `plasma-wiki>=1,<2` (`F/pyproject.toml:1-23`). PyPI latest = 1.4.0 (fractal), 1.9.0 (plasma-wiki)
  (PyPI JSON probe 2026-11-01). The installed skill and every line cited here are 1.3.0
  (`F/.claude-plugin/plugin.json`), so pin 1.3.0 to keep source and runtime identical.
- The README one-liner `uv tool install plasma-fractal --with-executables-from plasma-wiki`
  (`F/README.md:67`) does not work on uv 0.6.10: `uv tool install --help` shows only `--with`.
  fractal itself finds `wiki` beside its interpreter (`F/fractal/util/system.py:19-48`), but node
  agents call bare `wiki` from PATH (`F/fractal/_node/scripts/lint.sh:37`, memory steps), so
  install it as its own tool too:

  ```zsh
  uv tool install 'plasma-fractal==1.3.0'
  uv tool install 'plasma-wiki>=1,<2'
  fractal --version && wiki --version
  ```

## 2. What `fractal init` writes, and keeping it out of feature PRs

`fractal init <repo> --agent=codex` (`F/fractal/core/node.py:1950-2137`) writes:

| Artifact | Where | Tracked? | Evidence |
|---|---|---|---|
| User-node data (config.json, `.db`, radio) | `R/.fractal/<root>/` | No — self-ignored with a `*` `.gitignore` | `node.py:2085-2106`, `worktree.py:340-359` |
| Project-path cache | `R/.worktrees/.project/<root>` | No (`/.worktrees/` excluded) | `node.py:2086`, `F/fractal/_assets/git/exclude:6` |
| Exclude block | `R/.git/info/exclude` (shared by every worktree) | n/a | `node.py:2114`, `worktree.py:277-318` |
| Project wiki `wiki/` (via `wiki init`, ASCII-identifier naming) | `R/wiki/` (absent today) | Yes | `worktree.py:520-592`, `node.py:2118-2124` |
| `.gitattributes` line `**/_index.md merge=wiki` | `R/.gitattributes` | Yes | `worktree.py:539-550`, `commit.py:464-470` |
| Wiki merge driver | `R/.git/config` | n/a | `F/fractal/skills/fractal/SKILL.md:280-288` |

`fractal commit "<msg>" --init` then commits `wiki/` and the `.gitattributes` edit on the root
branch as `<root>: init (<msg>)`, pathspec-limited, no push (`commit.py:430-522`).

Side effect on the user's own checkouts: the exclude block adds unanchored patterns `.claude/`,
`.codex/`, `.agents/`, `tmp/`, `.venv`, `.db`, `.session`, `setup.log`, … (`_assets/git/exclude:8-77`)
to `R/.git/info/exclude`. Already-tracked files stay tracked (4 files under `R/.claude/` today), but
any NEW file under `.claude/` (e.g. a new skill) becomes invisible to `git status`/`git add` in
every worktree until `fractal destroy`/`exclude_strip` (`worktree.py:439-456`) removes the block.

Node seeds (`.fractal/<root>.<node>/`: NODE.md, steps, scripts, memory, plans, config.json) are
tracked on node branches but stripped when a node squash-merges into its parent
(`F/fractal/_scripts/merge.sh:120-125`, `fractal-nodes.md:99-105`).

Keeping fractal out of feature PRs: build each PR branch from `feat/ipados-foundation` (or the
previous PR branch) and transplant only project paths from the root branch:

```zsh
git -C /Users/shin1ohno/ManagedProjects/herdr-gpui diff --binary <pr-base> ipad_ui -- . \
  ':(exclude)wiki' ':(exclude).gitattributes' ':(exclude).fractal' \
  | git -C <pr-worktree> apply --index
```

Also tell nodes the shared wiki is off-limits (NODE.md Memory rule allows "a brief that bars the
shared wiki", `F/fractal/_node/NODE.md:73-83`), so the only wiki content is init's.

## 3. Branch naming

- The user (root) node is the branch checked out in the MAIN checkout; `/` is refused
  (`node.py:1964-1973`), detached HEAD refused (`node.py:1959-1963`), dot-nesting with another
  root refused (`node.py:1974-1993`). Init from a linked worktree is refused or misread as a
  sub-project (`F/fractal/cli/utils.py:382-428`, refusal text at :410-413), so the
  `.claude/worktrees/*` pattern cannot host the root.
- Node names: `[A-Za-z0-9_]+`, no `-`, `.`, `/`, max 64 chars (`worktree.py:91-143`, limits at
  :36/:41). Node branch = `<parent>.<name>`; worktree = `R/.worktrees/<branch>/`
  (`SKILL.md:525-527`).
- `node merge` needs the parent branch checked out in some worktree
  (`merge.sh:98-105`).
- Scheme compatible with `feat/ipados-foundation`:
  - root branch `ipad_ui`, created at `feat/ipados-foundation` (76a86890) in the main checkout,
    kept checked out there for init / node init / merges;
  - one top-level node per HANDOFF §4 PR unit: `ipad_ui.foundation`, `ipad_ui.sidebar` (01),
    `ipad_ui.rail` (03), `ipad_ui.softkeys` (02+06), `ipad_ui.keyboard` (07+08),
    `ipad_ui.sheets` (04+05);
  - PR branches keep the repo convention: `feat/ipad-ui-<unit>` cut from `feat/ipados-foundation`
    (or the previous PR branch when stacked), pushed to `fork` only (`git remote -v`: `fork` =
    shin1ohno, `origin` = penso).
- `feat/ipados-foundation` itself is never touched.

## 4. How a Codex node is launched

- Argv: `codex exec -C <worktree> --json [-m <model>] [-c model_reasoning_effort="<e>"] -- <prompt>`
  for a fresh thread, `codex exec resume <thread> --json …` for later steps
  (`F/fractal/impl/codex.py:252-308`). No `-s` and no approval flag are passed: the sandbox comes
  from the node's config.
- `CODEX_HOME=<node_dir>/.codex` is the only env key fractal adds (`codex.py:297-301`), composed over
  the loop's `os.environ` (`F/fractal/core/agent.py:530-556`).
- Seeded config (`F/fractal/_node/agents/codex/config.toml:1-11`): `model_reasoning_effort =
  "high"`, `sandbox_mode = "danger-full-access"`, analytics off. No `model`, no `approval_policy`,
  no `[mcp_servers]` — so the user's `~/.codex/config.toml` (`model = "gpt-6.1-sol"`,
  `model_reasoning_effort = "ultra"`, MCP servers `ai-memory`, `roon`) is NOT inherited, and nodes
  run without MCP.
- `auth.json` is a symlink to the resolved `~/.codex/auth.json` (`codex.py:480-495`). That file is
  `auth_mode = chatgpt` with no API key (key-name probe), so fractal's dollar figures are
  API-equivalent estimates; the real limit is the ChatGPT plan quota.
- Children copy the parent's live `.codex/config.toml`, else the package seed
  (`agent.py:1216-1231`); `.codex/skills` symlinks to the node's `skills/` (`agent.py:1232-1235`).
  `.codex/` is git-excluded (`_assets/git/exclude:43`), so edits to it are local-only.
- An explicit model triggers one preflight probe (`reply with: ok`, 60 s timeout) before the run
  (`codex.py:343-454`, `F/fractal/core/loop.py:697-715`).
- Codex reads the worktree's `AGENTS.md`, so project conventions apply; NODE.md's "Sole operator"
  rule overrides the commit/staging restrictions (`NODE.md:154-158`).
- The README warns nodes run with no permission prompts (`F/README.md:23-32`); the skill repeats it
  (`SKILL.md:180-184`).

Sandbox consequences:

- Under `danger-full-access` the memory-recorded `workspace-write` failures (361/4923 tests on
  Unix-socket binds, `git add` in a worktree whose index lives in the parent `.git`, simctl)
  should not occur. Nodes also need writes outside the worktree: the central DB at
  `R/.fractal/ipad_ui/.db` (`node.py:159-166`), `~/.fractal/pricing.json`
  (`F/fractal/core/pricing.py:21`), the gpg-agent socket (`fractal-nodes.md:39-41`). Switching a
  node to `workspace-write` would break `fractal commit`/radio/cost, not only tests.
- [推測] A tmux server started by a process inside Claude Code's command sandbox inherits that
  seatbelt profile, so every node in it would be confined like the earlier `workspace-write` runs.
  Run `fractal node start` with the sandbox disabled, or have the user run it in their own
  terminal. Probe: no tmux server is running now (`tmux ls` → "error connecting to
  /private/tmp/tmux-501/default (No such file or directory)", both inside and outside the
  sandbox), so the first start creates the server.

## 5. Environment propagation and the shared target dir

- tmux path: `start.sh` passes only `_NODE`, `PATH` (and `VIRTUAL_ENV` when set) explicitly
  (`F/fractal/_scripts/start.sh:82-93`). Everything else comes from the tmux server's global env.
  A cold `new-session` becomes the server and inherits the launching shell's exported env; a warm
  one does not (`start.sh:125-147`). So `export CARGO_TARGET_DIR=…` reaches nodes only if it was
  exported in the shell that cold-started the server (or set later with
  `tmux set-environment -g`).
- Headless path (`fractal node start --headless`): the loop inherits the full launching env
  (`start.sh:97-107`), and delegated children follow the parent's backend (`SKILL.md:126-130`).
- `CARGO_TARGET_DIR` is unset in this shell; no `.cargo/config.toml` exists in `R/` or `~/.cargo`.
- Repo scripts honour the variable but fall back to the checkout-local `target/` when it is unset:
  `R/scripts/ios/lint.sh:22-24` (`--target-dir "${CARGO_TARGET_DIR:-$root/target}"`),
  `R/justfile:453` (`test-ios`). `R/justfile:173-174` (`bundle`) hard-codes `--target-dir target`
  (desktop bundle only; not needed for iPad PRs). A `.cargo/config.toml` `build.target-dir` alone
  therefore loses to these explicit flags.
- Codex 0.166 supports `[shell_environment_policy]` with `set`, `inherit`, `exclude`,
  `include_only` (strings in the codex binary under
  `~/.local/share/mise/installs/npm-openai-codex/latest/.../bin/codex`).

Force every node onto `R/target/shared` with three layers:

1. In the top-level node's `<node_dir>/.codex/config.toml` (children copy it, `agent.py:1220-1231`):

   ```toml
   [shell_environment_policy]
   set = { CARGO_TARGET_DIR = "/Users/shin1ohno/ManagedProjects/herdr-gpui/target/shared" }
   ```

   This reaches every shell Codex spawns, independent of tmux.
   [推測] exact merge semantics with `inherit` were not exercised; verify with an `echo` step.
2. Export it in the shell that runs the first `fractal node start` (cold tmux server) so the loop
   and `setup.sh` see it too.
3. Assert it in `<node_dir>/scripts/test.sh` (which the agent runs, `02-EXECUTE.md:14-16`):
   `[[ "${CARGO_TARGET_DIR:-}" == /Users/shin1ohno/ManagedProjects/herdr-gpui/target/shared ]] || exit 3`,
   and add a NODE.md rule to run `echo $CARGO_TARGET_DIR` before the first cargo call.

[推測] Sharing one target dir across worktrees does not duplicate third-party artifacts, and
Cargo's workspace-relative stable hash for path packages should map each workspace crate to the
same file names in every worktree. Concurrent builds then serialize on Cargo's build-directory
lock and invalidate each other's incremental state (rebuild time, not disk). Not verified in
Cargo source here.

Relative-path trap: `just test-ios` and `just ios-sim` use `../herdr-uikit/Cargo.toml` relative to
the justfile directory (`R/justfile:420,453`). From `R/.worktrees/<branch>/` that resolves to
`R/.worktrees/herdr-uikit`, which does not exist. `just lint-ios` is safe because it resolves
herdr-uikit through `cargo metadata` (`R/scripts/ios/lint.sh:14-19`). Either run test-ios from the
main checkout (operator) or add an untracked symlink `R/.worktrees/herdr-uikit ->
/Users/shin1ohno/ManagedProjects/herdr-uikit` (needs authorization).

mise trust covers nested worktrees: `mise -C R/.claude/worktrees/ipad-menus current rust` →
1.96.1, and the trust entry points at the repo dir
(`~/.local/state/mise/trusted-configs/ManagedProjects-herdr-gpui-be1144f6313d8631` →
`/Users/shin1ohno/ManagedProjects/herdr-gpui`).

## 6. Commits, signing, merges

- Node commit: `fractal commit "<summary>"` checks scope, refreshes the wiki index, runs
  `scripts/lint.sh`, stages, commits, and pushes unless `local` (`F/fractal/core/commit.py:71-427`).
  Subject is forced to `<branch>: iteration <run>.<iter> (<summary>)` (`commit.py:312-320`;
  `04-COMMIT.md:30-32`) — not a conventional commit, so node history can never be the PR
  history (CI `commits` job, AGENTS.md "Changelog").
- Signing: the commit is a plain `git commit` (`commit.py:348-353`); global
  `commit.gpgsign = true` and `user.signingkey` come from `~/.gitconfig`. gpg-agent uses
  `pinentry-tty` with `default-cache-ttl`/`max-cache-ttl` 2592000 s (`~/.gnupg/gpg-agent.conf`).
  With a warm cache, node commits sign unattended. With a cold cache, pinentry-tty has no usable
  TTY inside `codex exec` [推測], so commits fail. The loop's backstop `--force` adds
  `--no-verify` but still signs (`commit.py:345-350`). Prime the cache before launch
  (`gpg-connect-agent reloadagent /bye && echo test | gpg --clearsign >/dev/null`).
- Hooks: `core.hooksPath` is unset and `.git/hooks` holds only samples, so `R/.githooks/commit-msg`
  (conventional-subject check) is NOT active. If someone runs `just hooks` (`R/justfile:347-353`)
  during the session, every `fractal commit` and `merge <branch>` commit is rejected. Leave hooks
  off until the tree is torn down.
- Scope: `--scope=dir1,dir2` restricts commits to directories (the wiki is always allowed);
  out-of-scope changes refuse the commit (`commit.py:545-588,677-707`; `NODE.md:134-137`). Repo-root
  project here, so no double nesting (`fractal-nodes.md:64-69` applies to sub-projects only).
  Root-level files (`README.md`, `NOTICE` for a Nord attribution) cannot be scoped as directories:
  leave the foundation node unscoped, or merge it with `--ignore-scope`.
- Child → parent (inside the tree): the parent's PREPARE step merges the parent branch and each
  ready child with `git merge --no-ff -m "merge <child>"` (`F/fractal/_node/steps/00-PREPARE.md:17-31`).
- Node → user branch (operator): `fractal node merge <branch>` from the repo root squash-merges
  into the parent's checkout, strips seeds, checks the footprint against the scope, optionally runs
  `--validate=<script>` (destination-relative, must not change the staged tree), commits
  `merge <branch>` (signed via global config) and advances the node's merge base
  (`merge.sh:9-33,930-968`; `SKILL.md:525-566`). A conflict restores the target and leaves a manual
  `git merge --squash` + `fractal node merge <branch> --continue`.
- `fractal node delete` force-deletes branches regardless of merge state (`SKILL.md:556-566`). Do
  not chain `--delete` until the PR branch has been exported and pushed.

## 7. Forbidding pushes

- `_push` runs `git push origin <branch>` after every node commit unless `local`
  (`commit.py:422-426,1415-1430`). Here `origin` is `git@github.com:penso/herdr-gpui.git`, the
  upstream HANDOFF §0 forbids pushing to.
- `--local` is inherited and immutable: a local parent forces `local` on children, and
  `--no-local` under a local parent raises (`node.py:1532-1544`). The user node has no `local`
  key, so pass `--local` on every top-level `node init`. `fractal commit --init` never pushes
  (`commit.py:499-501`).
- The agent can still run `git push`/`gh` itself under `danger-full-access`. NODE.md already says
  "Don't switch branches or push manually" (`NODE.md:176-178`). Add an explicit rule, and
  (authorization needed, repo-wide) a git-side guard:
  - `git -C R config remote.origin.pushurl no-push://penso-is-read-only`. Upstream is never a push
    target per HANDOFF, so this costs nothing.
  - `R/.git/hooks/pre-push` (active because hooksPath is unset) that rejects
    `refs/heads/ipad_ui*` and any push whose cwd is under `R/.worktrees/`. `--no-verify` bypasses
    it; this is defence in depth only.
- [推測] Launching with `env -u GITHUB_TOKEN` keeps the token out of the tmux global env, but `gh`
  may still authenticate from the keychain. Rely on the NODE.md rule.

## 8. Cost caps and gpt-6.1-sol pricing

- Pricing source: the LiteLLM table, cached at `~/.fractal/pricing.json` (`pricing.py:17-21`).
  Lookup chain: exact id → `openrouter/<id>` → author-stripped (`codex.py:640-652`). Cost =
  uncached-in × input + cached × cache_read + out × output (`codex.py:655-688`).
- `gpt-6.1-sol` is priced in the table fetched 2026-11-01: input 2e-06, cache_read 1e-07, output
  1e-05 USD/token ($2 / $0.10 / $10 per 1M); `chatgpt/gpt-6.1-sol` also exists.
- 1.3.0 refreshes pricing for any token-priced agent whether or not a model is set
  (`loop.py:663-695`; `F/CHANGELOG.md:363-368`), so `fractal-nodes.md:47-57` ("pricing never runs
  without --model") is stale for 1.3.0. The conclusion still holds: a cost cap over a model-less
  token-priced agent is refused at step launch (`CHANGELOG.md:373-374`). Pass
  `--model=gpt-6.1-sol` explicitly. It is needed anyway, because the node's `CODEX_HOME` names no
  model (§4).
- Model and effort do NOT pass to children unless `--inherit=config` or explicit flags
  (`node.py:1584-1593`); budget keys never inherit. Leaf-only trees (`--max-depth=0`) avoid this.
- Codex has no per-step cost cap (`enforces_budget = False`, `codex.py:190`). Overshoot is bounded
  only by `--step-timeout` (`SKILL.md:355-368`); `--max-step-cost` is warn-only. Caps are soft and
  checked between steps (`SKILL.md:172-178`).
- Sync is a billed step once per numbered step, so an iteration with 5 steps ≈ 10 invocations
  (`SKILL.md:385-395`). `--no-sync` halves that for leaves.
- [推測] Sizing: a Codex step reading ~1-3M input tokens (≈90 % cached) and writing 20-50k output
  costs ≈ $0.4-1.2. A 5-step leaf iteration without sync is then ≈ $2-6. Start with
  `--max-cost=30 --max-iter-cost=10 --max-iters=4` per PR unit and re-price after one iteration
  from `fractal node activity` (`fractal-nodes.md:82-97` measured $4-5 per parent iteration).
- Effort: fractal passes `-c model_reasoning_effort="<e>"` unvalidated (`codex.py:289-291`); the
  seed default is `high`. `--effort=ultra` matches the user's interactive setting at higher token
  cost.
- After `fractal node update`, re-read `<node_dir>/config.json` (`fractal-nodes.md:71-80`).

## 9. Concurrency

- Defaults: `max-children`, `max-depth`, `max-descendants` unlimited; `0` disables spawning
  (`SKILL.md:332-341`). There is no machine-wide node cap. tmux session names are
  `herdr-gpui (<branch with . → ->)`, and a duplicate name refuses (`start.sh:116-158`).
- Practical limits here: disk (§10 risks), Cargo's lock on the shared target dir (builds
  serialize [推測]), and ChatGPT plan quota (`auth_mode=chatgpt`). Recommend one building node at a
  time, at most two concurrent nodes with disjoint `--scope`, and leaf-only nodes
  (`--max-depth=0`) with Claude as the manager. PR units 01→03 depend on foundation, and 02/06 and
  07/08 touch shared sidebar/status-bar code (HANDOFF §4), so the dependency graph is mostly serial.
- `NODE.md` compute etiquette: bound parallel workers, `nice -n 15` (`NODE.md:150-153`). Add
  `CARGO_BUILD_JOBS=4` to the shell policy.

## 10. Authoring NODE.md, steps, scripts

- After `fractal node init`, the node dir is
  `R/.worktrees/ipad_ui.<name>/.fractal/ipad_ui.<name>/`. Edit `NODE.md` sections
  `## Instructions`, `## Completion Requirements`, `## Rules` (template:
  `F/fractal/_node/NODE.md:46-185`), then commit the seed from the node worktree with
  `fractal commit "configure <name>" --init` and `fractal node start` (`SKILL.md:428-436`).
  NODE.md is re-read every step, so it is the steering surface (`SKILL.md:463-468`).
- Steps: `steps/00-PREPARE … 04-COMMIT` (`F/fractal/_node/steps/`). Frontmatter keys: `agent`,
  `provider`, `model`, `effort`, `timeout`, `detached`, `requires_approval` (`SKILL.md:395-403`).
  `requires_approval: true` on `03-REVIEW.md` gives the operator a gate before COMMIT
  (`fractal node pending` / `approve`).
- Scripts: `setup.sh` runs each iteration from the worktree root; 3 consecutive failures end the run
  before any agent cost (`loop.py:60-63,1441-1457,1558-1562`). `lint.sh` runs before each commit;
  `test.sh` is run by the agent (`SKILL.md:405-412`).
- Seed rules that work against implementation: "Always make changes. Every iteration produces
  edits" (`NODE.md:182-184`). An empty Completion Requirements section means it never
  self-completes (`NODE.md:56-61`). Write concrete completion requirements and cap `--max-iters`.
- Nodes cannot read the design canvas (HANDOFF §1: the artifact and `*.dc.html` are not on this
  Mac). Inline the screen spec (HANDOFF §2 plus the screen's §3 block) in each NODE.md, or point at
  `git show fork/design/ipad-ui:<path>` once that branch exists.

Leaf NODE.md skeleton (operator fills the screen block):

```markdown
## Instructions
Implement HANDOFF screen(s) <NN> in this worktree. Spec: <paste §2 + §3 block>.
iPad-only behaviour stays behind crate::ipad_ui::active() or cfg(target_os = "ios"); desktop
output, snapshots and tests must not change. Follow AGENTS.md (1,000-line limit, Fluent strings
in crates/herdr-gpui/locales/en-US/*.ftl, tests in tests/ topic files, thiserror, no unwrap).
Files you own: <list>. Do not touch: <list owned by other nodes>.

## Completion Requirements
- bash $NODE_DIR/scripts/test.sh exits 0 (fmt check, file-size, sorted, i18n, iOS clippy via
  scripts/ios/lint.sh, focused cargo tests for the touched modules).
- Every changed function is listed with file:line in your final outbox post
  (`fractal radio post`), together with the exact commands run and their results; anything
  decided without evidence is tagged [推測].

## Rules
- Never run git push, gh, or any command that contacts GitHub. Never switch branches.
- Never write to the shared wiki/; private notes go to memory only.
- CARGO_TARGET_DIR must equal /Users/shin1ohno/ManagedProjects/herdr-gpui/target/shared; check
  with `echo $CARGO_TARGET_DIR` before the first cargo command. Never run cargo clean, --release
  builds, `just ci`, `cargo test --workspace`, or simulator bundles.
- `just` is not installed: call the underlying commands (see test.sh).
```

`test.sh` body (no `just`; commands from `R/justfile:41-56,227-240,409-410`):

```bash
[[ "${CARGO_TARGET_DIR:-}" == /Users/shin1ohno/ManagedProjects/herdr-gpui/target/shared ]] || { echo "CARGO_TARGET_DIR not shared" >&2; exit 3; }
cargo fmt --all -- --check
bash scripts/check-file-size.sh
python3 scripts/check-sorted.py
python3 scripts/check-i18n.py
bash scripts/ios/lint.sh
cargo test --locked -p herdr-gpui <module filter>
```

`setup.sh` guard (runs in loop env, so check disk only): fail when `df -g` free < 3 GiB.

## 11. Runbook (operator; each mutation needs the user's go-ahead where marked)

1. Free disk to at least 8 GiB before the first node (user decision: candidates are
   `R/target/shared/aarch64-apple-ios` 657M, stale `R/.claude/worktrees/wf_69a73f45-e56-*`
   ≈45M each, and unrelated data outside the repo).
2. Install: `uv tool install 'plasma-fractal==1.3.0'` and `uv tool install 'plasma-wiki>=1,<2'`.
3. Prime GPG: `gpg-connect-agent reloadagent /bye && echo test | gpg --clearsign >/dev/null`.
4. Root branch in the main checkout (main checkout leaves `feat/ipados-foundation`; same commit,
   so no file changes): `git -C R switch -c ipad_ui feat/ipados-foundation`.
5. Init, outside Claude's sandbox: `fractal init /Users/shin1ohno/ManagedProjects/herdr-gpui
   --agent=codex`, then from `R/`: `fractal commit "configure ipad_ui" --init` (commits `wiki/`
   and `.gitattributes` on `ipad_ui`).
6. Push guards (authorization): `git -C R config remote.origin.pushurl no-push://penso-is-read-only`;
   optional `R/.git/hooks/pre-push` rejecting `refs/heads/ipad_ui*`.
7. Per PR unit:

   ```zsh
   fractal node init sidebar --path=/Users/shin1ohno/ManagedProjects/herdr-gpui \
     --agent=codex --model=gpt-6.1-sol --effort=high --local --no-sync \
     --max-depth=0 --max-children=0 --max-descendants=0 \
     --max-iters=4 --max-cost=30 --max-iter-cost=10 \
     --step-timeout=45m --timeout=6h \
     --scope=crates/herdr-gpui,crates/herdr-commands
   ```

   Edit `NODE.md`, `scripts/test.sh`, `scripts/setup.sh`, and `.codex/config.toml`
   (`[shell_environment_policy]`), then re-read `config.json` to confirm `local: true`,
   `model`, and `max_cost`.
8. Launch, sandbox-disabled or from the user's terminal:

   ```zsh
   cd /Users/shin1ohno/ManagedProjects/herdr-gpui/.worktrees/ipad_ui.sidebar
   fractal commit "configure sidebar" --init
   export CARGO_TARGET_DIR=/Users/shin1ohno/ManagedProjects/herdr-gpui/target/shared CARGO_BUILD_JOBS=4
   fractal node start
   ```

9. Watch every 5-10 min: `fractal node list`, `fractal node status ipad_ui.sidebar`,
   `fractal node activity ipad_ui.sidebar`, `fractal node cost spent ipad_ui.sidebar`, the tail of
   `<node_dir>/codex.err`, and `df -h`. Flat spend with an active step is normal
   (`fractal-nodes.md:172-179`). Steer by editing NODE.md; stop with `fractal node finish|stop|kill`
   (`SKILL.md:496-504`).
10. Merge into `ipad_ui` from `R/`: `fractal node merge ipad_ui.sidebar` (optional
    `--validate=.fractal/ipad_ui/validate.sh`, an untracked script inside the self-ignored user
    dir). Then run the full gates once on the operator side: the `just ci` equivalents,
    `bash scripts/ios/lint.sh`, and the test-ios commands from `R/justfile:448-454`, run from the
    main checkout so `../herdr-uikit` resolves.
11. Export the PR: create `feat/ipad-ui-sidebar` from `feat/ipados-foundation` (or the previous PR
    branch) in a linked worktree, apply the `git diff … ':(exclude)wiki' ':(exclude).gitattributes'`
    transplant, commit with a conventional signed subject (no AI trailers, per AGENTS.md "Git And
    Handoff"), `git push fork feat/ipad-ui-sidebar`, and open the PR on shin1ohno/herdr-gpui.
12. After all PRs merge: `fractal reset` (refuses while nodes run; removes worktrees and branches,
    keeps wiki and history; `SKILL.md:568-573`), then `fractal destroy ipad_ui` to strip the
    exclude block, and switch the main checkout back.

## Screens

None — execution-tooling area.

## Risks

- Disk: 5.0 GiB free (falling from 5.4 GiB during this session); `target/shared` is 6.4 GiB
  (debug 2.7, aarch64-apple-ios-sim 3.1, aarch64-apple-ios 0.66). A full `just ci` (clippy and
  tests in two feature sets) or a release/simulator bundle in a node can exhaust it. Each node
  worktree costs ≈53 MB of checkout (measured working tree) plus Codex rollouts in `.codex/sessions`.
- Push to upstream: without `--local`, every node commit pushes to `origin` = penso/herdr-gpui
  (`commit.py:1428`).
- Unbounded agent: `danger-full-access` gives nodes the user's credentials (gh, ssh, gpg) and
  network.
- Sandbox inheritance [推測]: starting the tmux server from Claude's sandboxed Bash confines every
  node.
- `CARGO_TARGET_DIR` silently missing on a warm tmux server makes `scripts/ios/lint.sh` and
  test-ios fall back to `<worktree>/target`, a fresh multi-GiB build per worktree.
- Model drift: without `--model`, nodes run Codex's built-in default instead of gpt-6.1-sol, and a
  cost cap is refused.
- Signing: a cold gpg cache with `pinentry-tty` fails node commits [推測].
- Hooks: running `just hooks` mid-session makes every fractal commit fail on the
  conventional-subject check.
- Exclude block side effect: new files under `.claude/` (and `tmp/`, `.venv`, `.db`) become
  invisible to git in every worktree until destroy.
- `just` is absent, so HANDOFF gates written as `just …` fail verbatim in nodes and for the
  operator.
- test-ios / ios-sim from a node worktree point at a missing `../herdr-uikit`.
- ChatGPT plan quota exhaustion fails steps. Step retries default to 1 with 10 s backoff
  (`SKILL.md:88-89`), so a quota wall can burn iterations without progress.
- Node commit subjects are non-conventional; PR export must re-commit (squash transplant).
- `fractal node delete` force-deletes unmerged branches.

## Unknowns

- fractal 1.4.0 (PyPI latest) changes vs 1.3.0 were not read. The runbook pins 1.3.0.
- Whether gpt-6.1-sol passes the preflight under this ChatGPT plan: the user's interactive config
  uses it, but no probe was run (no install).
- Exact `[shell_environment_policy] set` merge behaviour in codex 0.166 (key presence confirmed
  from binary strings only).
- Whether Cargo's hashing shares workspace-crate artifacts across worktrees (disk) or duplicates
  them [推測: shared].
- Real per-iteration cost of gpt-6.1-sol on this repo. Measure after one iteration.
- Whether a tmux server started outside the sandbox but from a Claude Bash call outlives the Claude
  session [推測: yes].
- Where `just` comes from on the user's other machines (flake.nix exists; not on PATH here).
- Design sources (`*.dc.html`) are not on this Mac yet (HANDOFF:15-16); nodes depend on the
  inlined spec until `fork/design/ipad-ui` exists.
