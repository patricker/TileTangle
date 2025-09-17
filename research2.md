Awesome—here are concrete, “drop-in” heuristics you can wire into TileTangle’s iterator/generator. I’ll split them into (A) blanks and (B) left/right traversal + branching order, and give tight citations where the idea comes from or is validated.

---

## A) Blank-tile handling: practical heuristics

1. **Lazy binding (commit blanks as late as possible).**
   Treat a blank on the rack as a wildcard that **doesn’t pick a letter until you step across a GADDAG arc that requires it** *and* the target square’s cross-check allows it. This keeps the branch factor small and avoids “painting yourself into a corner.” The original Scrabble generators push all pruning into dictionary/cross-checks (precompute cross sets; see below) and only commit to letters when necessary. ([nongnu.org][1], [Computer Science Department][2])

2. **Cross-checks first; then child-arcs.**
   When an empty square is next, compute `allowed = crosscheck(square) ∩ children(current_state)`. If the rack contains a blank, enumerate **only** letters in `allowed`. This mirrors Appel–Jacobson’s constant-time pruning via cross-check sets and maps perfectly onto GADDAG arc following. ([Computer Science Department][2], [nongnu.org][1])

3. **Order blank assignments by “promise.”**
   When branching a blank over multiple candidate letters, try them in this order:

* **Board-forced letters** first (adjacent fixed letters that restrict choices tightly).
* **High score potential** next (tile value × word/letter multipliers on this square and any perpendicular word bonus).
* **Rack-leave quality** last (prefer assignments that leave “good” leaves; Quackle/modern engines evaluate leaves downstream).
  This is consistent with classic move-ordering in Scrabble engines (generate legal plays fast; score/evaluate later) and is compatible with Quackle-style pipelines. ([MIT CSAIL][3], [SJSU Computer Science][4])

4. **Prefer blanks for awkward consonants only when they “unlock” anchors.**
   As a tie-breaker, spend blanks on hard-to-use letters (Q, J, X, Z, V) **only if** the local cross-checks support them **and** they enable entering a promising right side (big multiplier, long suffix). Otherwise, conserve blanks for bingo lines. (This is a pragmatic heuristic used in high-level play/analysis; see Quackle notes and competitive AI writeups.) ([MIT CSAIL][3], [UPCommons][5])

5. **Encode blanks compactly; keep semantics in the search.**
   Implementation tip: keep the GADDAG **letter-pure**; handle blanks in the generator. If you’re packing board tiles into integers, you can adopt the common pattern: “blank on board has a high bit set; blank on rack is 0 and expands at runtime.” (Nice bit-packing notes from engine authors.) ([Ziggit][6])

6. **De-duplicate plays with blank case.**
   If your iterator yields canonical move keys (e.g., `(row, col, dir, letters-with-blank-tags)`), normalize blank assignments so the same placement with a different blank expansion isn’t double-emitted. (Quackle and others normalize output; avoids UI confusion.) ([MIT CSAIL][3])

---

## B) GADDAG left/right traversal & branching order

These follow Gordon’s algorithm closely but add move-ordering so you get “good” moves early while still being exhaustive. ([ericsink.com][7])

**Core rules you likely have already:**

* Precompute **anchor squares** and **cross-check sets** (turn the 2-D board into 1-D problems + constant-time pruning). ([Computer Science Department][2])
* For each anchor on a line, **walk left** first using rack tiles (following GADDAG arcs before the delimiter), and at each step also try the **delimiter arc** (`+`) to **switch to right-extension** through board letters and empty squares. Fixed board letters must match arcs exactly. Emit a move when you’re at a **terminal state** and the next square (beyond the placed word) is either out-of-bounds or blocked. ([ericsink.com][7], [users.cs.northwestern.edu][8])

**Traversal/branching heuristics that pay off:**

1. **Early delimiter tests (“eager split”).**
   While extending left, **test the delimiter arc at each depth** before adding another left letter. This finds short, locally valid words early (good for interactive latency) and often leads quickly into long right extensions constrained by board letters. Gordon’s description matches this (“look for ‘+’ while moving left, then start right”). ([ericsink.com][7])

2. **Prefer consuming board letters before rack letters on the right.**
   Once you’ve taken the delimiter, **greedily traverse consecutive board letters** to the right as far as the automaton allows before placing any rack tile. This collapses huge subtrees immediately (board letters are hard constraints). Common in visualized GADDAG traversals and engine writeups. ([seth.rocks][9])

3. **Left-depth cap = contiguous empty run left of anchor.**
   Do **not** branch left beyond the nearest fixed tile or board edge; that’s the classic bound, but also stop if cross-checks on any left square yield `∅`. This prevents pointless delimiter tests beyond legal extent. (Appel–Jacobson’s pruning principle; applied in GADDAG context.) ([Computer Science Department][2])

4. **Child-arc ordering: degree & multiplier potential.**
   When choosing which next letter to try (left or right), sort children by:

   * **Arc tightness** first (fewer descendants / lower “fan-out” first);
   * then **square multipliers** impact;
   * then **tile score**.
     This typically finds high-value, fully constrained plays early and keeps the frontier small. (General branching-factor heuristic; validated in Scrabble engine notes.) ([users.cs.northwestern.edu][8], [MIT CSAIL][3])

5. **Cross-checks everywhere, not just at anchors.**
   Apply cross-check filtering **on every empty square you attempt**, left and right. With GADDAG you’ll still try only dictionary-consistent arcs, but cross-checks kill branches *before* trying to match automaton children. This gives the “constant-time prune” win from Appel–Jacobson atop GADDAG. ([Computer Science Department][2])

6. **Occupied-anchor walk-through (hook-through).**
   If the anchor cell is occupied (classic “through” play), **start by traversing that fixed letter (and any contiguous fixed letters) in the GADDAG** before you consume rack tiles. This is spelled out in community explanations and matches the GADDAG design (bidirectional around a hook). ([Wikipedia][10], [seth.rocks][9])

7. **Terminal-state checks on the fly.**
   Any time your last placed tile (or last matched board tile) lands on a **terminal GADDAG state** and the next square in the main direction is blocked or empty with no forced continuation, **record the move**. Many published pseudocodes do this to avoid missing shorter words embedded in longer paths. ([users.cs.northwestern.edu][8])

8. **Duplicate-path pruning with memoization.**
   Cache `(gaddag_state, idx_left, idx_right, rack_multiset_signature)` → failure to stop repeating doomed partials—especially helpful with multiple identical rack letters + blanks. (Not specific to one paper; a standard DFS optimization consistent with the literature.) ([users.cs.northwestern.edu][8])

---

## One clean, Rust-friendly outline (pseudo)

* **Data:** read-only, minimized GADDAG (FST or packed array); cross-checks as `[26-bit mask]` per square; rack as small multiset (bitset + counts + blank count). ([Amédée d'Aboville][11])
* **Left phase:** recursive `extend_left(state, pos, rack)`

  * For each `c` in `rack ∩ children(state)` that also passes `crosscheck(pos)` (if pos empty): place `c`, recurse.
  * **Also** if `delimiter` child exists: `extend_right(state_delim, anchor_right, rack_remaining)` now.
* **Right phase:** `extend_right(state, pos, rack)`

  * While `pos` is board-letter `b`: require `b` child; advance.
  * If `pos` empty: enumerate `c` from `allowed = crosscheck(pos) ∩ children(state) ∩ rack`; include blank expansions.
  * If `state.terminal` and end-conditions hold: emit.

This structure exactly matches Gordon’s algorithm; using `fst` means `children(state)` queries are cheap and cache-friendly. ([ericsink.com][7], [Amédée d'Aboville][11])

---

## Quick reference list (good for commit msgs / docstrings)

* **Appel & Jacobson (1988):** anchors + **precomputed cross-checks** ⇒ constant-time pruning. Use everywhere, not just at start. ([Computer Science Department][2], [nongnu.org][1])
* **Gordon (1994):** **GADDAG** + “try delimiter while going left; then go right” to avoid nondeterministic prefix generation. ([ericsink.com][7])
* **Quackle docs:** production move generator uses a **GADDAG**; coordinates with simulation/evaluation (good place to mirror move ordering). ([MIT CSAIL][3])
* **Amédée (2020):** practical **Rust FST-backed GADDAG**; easy to embed/mmap; great for TileTangle’s read-only dictionary. ([Amédée d'Aboville][11])
* **Seth Hoffert:** traversal visualizations; confirms walk-through of fixed letters and delimiter usage patterns. ([seth.rocks][9])
* **Implementation bits:** compact blank encoding and board/tile packing tricks from engine authors (good for hot loops). ([Ziggit][6])

If you want, I can adapt these into a tiny Rust trait for your iterator (with a `BlankPolicy` enum and a `BranchOrder` strategy), plus a couple of property tests to guarantee we don’t double-emit moves with different blank expansions.

[1]: https://www.nongnu.org/eliot/download/aj.pdf?utm_source=chatgpt.com "The World's Fastest"
[2]: https://www.cs.upc.edu/prop/data/uploads/fastestscrabble.pdf?utm_source=chatgpt.com "The World's Fastest"
[3]: https://people.csail.mit.edu/jasonkb/quackle/doc/how_quackle_plays_scrabble.html?utm_source=chatgpt.com "How Quackle Plays Scrabble"
[4]: https://www.cs.sjsu.edu/faculty/pollett/masters/Semesters/Spring17/priyatha/CS298Report.pdf?utm_source=chatgpt.com "A Scrabble Artificial Intelligence Game A Project Presented to ..."
[5]: https://upcommons.upc.edu/bitstream/handle/2117/367333/TAGR1de1.pdf?sequence=1&utm_source=chatgpt.com "A novel computer Scrabble engine based on probability ..."
[6]: https://ziggit.dev/t/gaddag-optimization/7343?utm_source=chatgpt.com "Gaddag optimization - Brainstorming"
[7]: https://ericsink.com/downloads/faster-scrabble-gordon.pdf?utm_source=chatgpt.com "A Faster Scrabble Move Generation Algorithm"
[8]: https://users.cs.northwestern.edu/~robby/uc-courses/22001-2008-winter/scrabble-on-the-tetrascale-wickman.pdf?utm_source=chatgpt.com "Scrabble on the TeraScale"
[9]: https://seth.rocks/projects/scrabble/?utm_source=chatgpt.com "Scrabble solver"
[10]: https://en.wikipedia.org/wiki/GADDAG?utm_source=chatgpt.com "GADDAG"
[11]: https://amedee.me/2020/11/04/fst-gaddag/?utm_source=chatgpt.com "The 2 Data structures you need in a Scrabble AI"
