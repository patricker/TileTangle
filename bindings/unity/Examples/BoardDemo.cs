using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using TileTangle;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UI;

namespace TileTangle.Examples
{
    public class BoardDemo : MonoBehaviour
    {
        [Header("Board Settings")]
        public int width = 5;
        public int height = 5; // per-layer height
        public bool use3D = false;
        public int depth = 1;
        public int zLayer = 0;
        public enum AdjacencyMode { Orthogonal, Diagonal, Hex }
        public AdjacencyMode adjacency = AdjacencyMode.Orthogonal;
        public enum ShapeMask { Rect, Diamond }
        public ShapeMask shape = ShapeMask.Rect;
        public bool freeWordMode = true;
        public bool rtl = false;
        public bool stackingEnabled = false;
        public bool sumStackScoring = false;
        public bool forbidSameOverlay = true;

        [Header("UI Settings")]
        public Vector2 cellSize = new Vector2(48, 48);
        public Vector2 cellSpacing = new Vector2(6, 6);

        private Engine engine = new Engine();
        private GridLayoutGroup grid;
        private readonly List<Button> cells = new();
        private HorizontalLayoutGroup rackBar;
        private readonly List<Button> rackButtons = new();
        private string? selectedKindId;
        private Text statusLabel;
        private string scoreSummaryLine = "Scores unavailable";
        private string statusDetail = string.Empty;
        private readonly System.Collections.Generic.List<(int x, int y, string kindId)> staged = new();
        private System.Collections.Generic.List<Vector2Int> hlMain = new();
        private System.Collections.Generic.List<System.Collections.Generic.List<Vector2Int>> hlCross = new();
        private Button cpuButton;
        private Button cpuDifficultyButton;
        private Toggle cpuAutoToggle;
        private bool cpuAutoEnabled;
        private Coroutine cpuAutoRoutine;
        private bool cpuBusy;
        private string cpuDifficulty = "medium";
        private Button undoButton;
        private Button redoButton;
        private readonly List<string> history = new();
        private int historyIndex = -1;

        void Start()
        {
            EnsureUiRoot();
            BuildOrRebuildEngine();
            BuildGrid();
            RefreshBoard();
            RefreshRack();
            SyncScoresAndMaybeTriggerCpu();
        }

        void OnDestroy()
        {
            engine.Dispose();
        }

        public void ToggleFreeWordMode(bool on)
        {
            freeWordMode = on;
            engine.SetFreeWordMode(on);
        }

        private void BuildOrRebuildEngine()
        {
            var cfg = BuildConfigJson();
            if (!engine.NewGame(cfg, 2))
            {
                Debug.LogError($"Failed to create game: {Engine.LastError()}");
            }
            engine.SetFreeWordMode(freeWordMode);
            engine.SetReadingDirection(rtl);
            engine.SetStacking(stackingEnabled, 7, forbidSameOverlay, sumStackScoring);
            ApplyAutoBonuses();
            ResetHistory();
        }

        private void BuildGrid()
        {
            if (grid != null) return;
            var canvas = EnsureCanvas();
            var gridGo = new GameObject("BoardGrid", typeof(RectTransform), typeof(GridLayoutGroup));
            gridGo.transform.SetParent(canvas.transform, false);
            var rt = gridGo.GetComponent<RectTransform>();
            rt.anchorMin = Vector2.zero;
            rt.anchorMax = Vector2.one;
            rt.offsetMin = new Vector2(16, 16);
            rt.offsetMax = new Vector2(-16, -60);
            grid = gridGo.GetComponent<GridLayoutGroup>();
            grid.cellSize = cellSize;
            grid.spacing = cellSpacing;
            grid.constraint = GridLayoutGroup.Constraint.FixedColumnCount;
            grid.constraintCount = width;

            // Top bar with toggles
            var bar = new GameObject("TopBar", typeof(RectTransform), typeof(HorizontalLayoutGroup));
            bar.transform.SetParent(canvas.transform, false);
            var barRt = bar.GetComponent<RectTransform>();
            barRt.anchorMin = new Vector2(0, 1);
            barRt.anchorMax = new Vector2(1, 1);
            barRt.pivot = new Vector2(0.5f, 1f);
            barRt.sizeDelta = new Vector2(0, 44);
            barRt.anchoredPosition = new Vector2(0, 0);
            var layout = bar.GetComponent<HorizontalLayoutGroup>();
            layout.padding = new RectOffset(12, 12, 6, 6);
            layout.spacing = 12;

            var freeToggle = CreateToggle(bar.transform, "Free Word", freeWordMode);
            freeToggle.onValueChanged.AddListener(ToggleFreeWordMode);
            var rtlToggle = CreateToggle(bar.transform, "RTL", rtl);
            rtlToggle.onValueChanged.AddListener(on => { rtl = on; engine.SetReadingDirection(on); });
            var stackToggle = CreateToggle(bar.transform, "Stack", stackingEnabled);
            stackToggle.onValueChanged.AddListener(on => { stackingEnabled = on; engine.SetStacking(stackingEnabled, 7, forbidSameOverlay, sumStackScoring); });
            var adjBtn = CreateButton(bar.transform, $"Adj: {adjacency}");
            adjBtn.onClick.AddListener(() => { CycleAdjacency(adjBtn); });
            var shapeBtn = CreateButton(bar.transform, $"Shape: {shape}");
            shapeBtn.onClick.AddListener(() => { CycleShape(shapeBtn); });
            var dimToggle = CreateToggle(bar.transform, "3D", use3D);
            dimToggle.onValueChanged.AddListener(on => { use3D = on; if (!use3D) { zLayer = 0; depth = 1; } BuildOrRebuildEngine(); RebuildGrid(); RefreshBoard(); });
            var zDec = CreateButton(bar.transform, "Z-");
            zDec.onClick.AddListener(() => { SetZLayer(zLayer - 1); });
            var zInc = CreateButton(bar.transform, "Z+");
            zInc.onClick.AddListener(() => { SetZLayer(zLayer + 1); });

            // Score label
            var scoreGo = new GameObject("Score", typeof(RectTransform), typeof(Text));
            scoreGo.transform.SetParent(bar.transform, false);
            var t = scoreGo.GetComponent<Text>();
            t.font = Resources.GetBuiltinResource<Font>("Arial.ttf");
            t.alignment = TextAnchor.MiddleLeft;
            t.color = Color.white;
            statusLabel = t;
            RenderStatus();

            SpawnCells();

            // Rack bar at bottom
            var rackGo = new GameObject("Rack", typeof(RectTransform), typeof(HorizontalLayoutGroup));
            rackGo.transform.SetParent(canvas.transform, false);
            var rackRt = rackGo.GetComponent<RectTransform>();
            rackRt.anchorMin = new Vector2(0, 0);
            rackRt.anchorMax = new Vector2(1, 0);
            rackRt.pivot = new Vector2(0.5f, 0f);
            rackRt.sizeDelta = new Vector2(0, 60);
            rackRt.anchoredPosition = new Vector2(0, 0);
            rackBar = rackGo.GetComponent<HorizontalLayoutGroup>();
            rackBar.padding = new RectOffset(12, 12, 10, 10);
            rackBar.spacing = 8;

            // Right rail for moves
            var rightGo = new GameObject("MovesPanel", typeof(RectTransform), typeof(Image), typeof(VerticalLayoutGroup));
            rightGo.transform.SetParent(canvas.transform, false);
            var rightRt = rightGo.GetComponent<RectTransform>();
            rightRt.anchorMin = new Vector2(1f, 0f);
            rightRt.anchorMax = new Vector2(1f, 1f);
            rightRt.pivot = new Vector2(1f, 0.5f);
            rightRt.sizeDelta = new Vector2(220, -60);
            rightRt.anchoredPosition = new Vector2(-8, 30);
            var rightBg = rightGo.GetComponent<Image>();
            rightBg.color = new Color(0.1f, 0.1f, 0.12f, 0.6f);
            movesPanel = rightGo.GetComponent<VerticalLayoutGroup>();
            movesPanel.padding = new RectOffset(8, 8, 8, 8);
            movesPanel.spacing = 6;

            // Commit / Cancel buttons
            var commit = CreateButton(bar.transform, "Commit Move");
            commit.onClick.AddListener(CommitStaged);
            var cancel = CreateButton(bar.transform, "Cancel");
            cancel.onClick.AddListener(ClearStaged);

            cpuDifficultyButton = CreateButton(bar.transform, "Difficulty: Medium");
            cpuDifficultyButton.onClick.AddListener(CycleDifficulty);

            cpuButton = CreateButton(bar.transform, "CPU Move (Medium)");
            cpuButton.onClick.AddListener(CpuMove);

            cpuAutoToggle = CreateToggle(bar.transform, "CPU Opponent", cpuAutoEnabled);
            cpuAutoToggle.onValueChanged.AddListener(SetCpuAuto);

            undoButton = CreateButton(bar.transform, "Undo");
            undoButton.onClick.AddListener(Undo);
            redoButton = CreateButton(bar.transform, "Redo");
            redoButton.onClick.AddListener(Redo);

            movesButton = CreateButton(bar.transform, "Moves");
            movesButton.onClick.AddListener(FetchMoves);
        }

        private void RebuildGrid()
        {
            foreach (var b in cells) Destroy(b.gameObject);
            cells.Clear();
            grid.constraintCount = width;
            SpawnCells();
            RefreshRack();
            // Preview initial staging (none)
            UpdatePreviewOverlay();
            SyncScoresAndMaybeTriggerCpu();
        }

        private void SpawnCells()
        {
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                var cell = CreateCellButton(grid.transform);
                var cx = x; var cy = y;
                cell.onClick.AddListener(() => OnCellClicked(cx, cy));
                cells.Add(cell);
            }
        }

        private void OnCellClicked(int x, int y)
        {
            int gy = GlobalY(y);
            var placements = new[] { new { x, y = gy, kind_id = "A" } };
            if (!string.IsNullOrEmpty(selectedKindId))
                placements = new[] { new { x, y = gy, kind_id = selectedKindId } };
            var json = JsonSerializer.Serialize(placements);
            PushSnapshot();
            var res = engine.PlayMove(json);
            if (res == null)
                Debug.LogError($"play_move error: {Engine.LastError()}");
            else UpdateScoreOverlay(res);
            RefreshBoard();
            RefreshRack();
            SyncScoresAndMaybeTriggerCpu();
        }

        private void RefreshBoard()
        {
            var boardJson = engine.GetBoardJson();
            if (boardJson == null) return;
            try
            {
                var doc = JsonDocument.Parse(boardJson);
                var rows = doc.RootElement.GetProperty("rows");
                int i = 0;
                int startY = GlobalY(0);
                for (int y = 0; y < height; y++)
                for (int x = 0; x < width; x++)
                {
                    var sym = rows[startY + y][x].GetString() ?? string.Empty;
                    SetCellText(cells[i++], sym);
                }
                // Overlay preview/committed highlights (map to local slice)
                foreach (var v in hlMain)
                {
                    int ly = v.y - startY;
                    if (ly < 0 || ly >= height) continue;
                    int idx = ly * width + v.x;
                    if (idx >= 0 && idx < cells.Count)
                    {
                        var img = cells[idx].GetComponent<Image>();
                        if (img != null) img.color = new Color(1.0f, 0.95f, 0.7f, 1f);
                    }
                }
                foreach (var list in hlCross)
                {
                    foreach (var v in list)
                    {
                        int ly = v.y - startY;
                        if (ly < 0 || ly >= height) continue;
                        int idx = ly * width + v.x;
                        if (idx >= 0 && idx < cells.Count)
                        {
                            var img = cells[idx].GetComponent<Image>();
                            if (img != null) img.color = new Color(0.95f, 1.0f, 0.8f, 1f);
                        }
                    }
                }
                // Draw staged tile glyphs on top of board text
                foreach (var s in staged)
                {
                    int idx = s.y * width + s.x;
                    if (idx >= 0 && idx < cells.Count)
                    {
                        SetCellText(cells[idx], s.kindId);
                    }
                }
            }
            catch (Exception e)
            {
                Debug.LogError(e);
            }
        }

        private void RefreshRack()
        {
            foreach (var b in rackButtons) Destroy(b.gameObject);
            rackButtons.Clear();
            selectedKindId = null;
            var json = engine.GetRackJson();
            if (json == null) return;
            try
            {
                var doc = JsonDocument.Parse(json);
                foreach (var el in doc.RootElement.EnumerateArray())
                {
                    var kindId = el.GetProperty("kind_id").GetString() ?? "";
                    var symbol = el.GetProperty("symbol").GetString() ?? kindId;
                    var score = el.GetProperty("score").GetInt16();
                    var btn = CreateRackButton(rackBar.transform, symbol, score);
                    var drag = btn.gameObject.AddComponent<RackTileDraggable>();
                    drag.KindId = kindId; drag.Symbol = symbol; drag.Score = score;
                    btn.onClick.AddListener(() => { selectedKindId = kindId; HighlightSelected(btn); });
                    rackButtons.Add(btn);
                }
            }
            catch (Exception e)
            {
                Debug.LogError(e);
            }
        }

        private void UpdateScoreOverlay(string scoreJson)
        {
            try
            {
                var doc = JsonDocument.Parse(scoreJson);
                var total = doc.RootElement.GetProperty("total").GetInt32();
                var mainWord = doc.RootElement.GetProperty("main_word").GetString() ?? "";
                var mainScore = doc.RootElement.GetProperty("main_score").GetInt32();
                var cross = doc.RootElement.GetProperty("cross_words");
                var crossSum = 0;
                foreach (var cw in cross.EnumerateArray()) crossSum += cw[1].GetInt32();
                statusDetail = $"Last: {mainWord} total={total} (main={mainScore} +cross={crossSum})";
                RenderStatus();
            }
            catch (Exception e)
            {
                Debug.LogError(e);
                statusDetail = "";
                RenderStatus();
            }
        }

        private void CycleDifficulty()
        {
            cpuDifficulty = cpuDifficulty switch
            {
                "easy" => "medium",
                "medium" => "hard",
                _ => "easy",
            };
            string human = cpuDifficulty switch
            {
                "easy" => "Easy",
                "hard" => "Hard",
                _ => "Medium",
            };
            if (cpuDifficultyButton != null)
            {
                cpuDifficultyButton.GetComponentInChildren<Text>().text = $"Difficulty: {human}";
            }
            if (cpuButton != null)
            {
                cpuButton.GetComponentInChildren<Text>().text = $"CPU Move ({human})";
            }
        }

        private void ApplyPreviewHighlights(string previewJson)
        {
            try
            {
                using var doc = JsonDocument.Parse(previewJson);
                var mainCells = doc.RootElement.GetProperty("main_cells");
                var crossCells = doc.RootElement.GetProperty("cross_cells");
                hlMain = new System.Collections.Generic.List<Vector2Int>();
                foreach (var c in mainCells.EnumerateArray())
                {
                    hlMain.Add(new Vector2Int(c[0].GetInt32(), c[1].GetInt32()));
                }
                hlCross = new System.Collections.Generic.List<System.Collections.Generic.List<Vector2Int>>();
                foreach (var arr in crossCells.EnumerateArray())
                {
                    var list = new System.Collections.Generic.List<Vector2Int>();
                    foreach (var c in arr.EnumerateArray())
                    {
                        list.Add(new Vector2Int(c[0].GetInt32(), c[1].GetInt32()));
                    }
                    hlCross.Add(list);
                }
            }
            catch (Exception e)
            {
                Debug.LogWarning($"Failed to parse preview JSON: {e.Message}");
            }
        }

        private void RenderStatus()
        {
            if (statusLabel == null) return;
            if (string.IsNullOrEmpty(scoreSummaryLine))
            {
                statusLabel.text = statusDetail;
            }
            else if (string.IsNullOrEmpty(statusDetail))
            {
                statusLabel.text = scoreSummaryLine;
            }
            else
            {
                statusLabel.text = $"{scoreSummaryLine}\n{statusDetail}";
            }
        }

        private int UpdateScoreSummary()
        {
            var json = engine.GetScoresJson();
            if (string.IsNullOrEmpty(json))
            {
                scoreSummaryLine = "Scores unavailable";
                RenderStatus();
                return 0;
            }
            try
            {
                using var doc = JsonDocument.Parse(json);
                int you = 0;
                int cpu = 0;
                int idx = 0;
                foreach (var player in doc.RootElement.GetProperty("players").EnumerateArray())
                {
                    int score = 0;
                    if (player.TryGetProperty("score", out var scoreEl) && scoreEl.ValueKind == JsonValueKind.Number)
                    {
                        score = scoreEl.GetInt32();
                    }
                    if (idx == 0) you = score;
                    else if (idx == 1) cpu = score;
                    idx++;
                }
                int toMove = 0;
                if (doc.RootElement.TryGetProperty("to_move", out var toMoveEl) && toMoveEl.ValueKind == JsonValueKind.Number)
                {
                    toMove = toMoveEl.GetInt32();
                }
                var turnLabel = toMove == 0 ? "Your turn" : "CPU thinking";
                scoreSummaryLine = $"You: {you} | CPU: {cpu} - {turnLabel}";
                RenderStatus();
                return toMove;
            }
            catch (Exception e)
            {
                Debug.LogWarning($"Failed to parse score JSON: {e.Message}");
                scoreSummaryLine = "Scores unavailable";
                RenderStatus();
                return 0;
            }
        }

        private void SyncScoresAndMaybeTriggerCpu()
        {
            var toMove = UpdateScoreSummary();
            MaybeTriggerCpuTurnInternal(toMove);
        }

        private void MaybeTriggerCpuTurnInternal(int toMove)
        {
            if (!cpuAutoEnabled || cpuBusy) return;
            if (staged.Count > 0) return;
            if (toMove != 1) return;
            if (cpuAutoRoutine != null)
            {
                StopCoroutine(cpuAutoRoutine);
            }
            cpuAutoRoutine = StartCoroutine(CpuAutoRoutine());
        }

        private IEnumerator CpuAutoRoutine()
        {
            yield return null;
            cpuAutoRoutine = null;
            if (!cpuAutoEnabled || cpuBusy) yield break;
            if (staged.Count > 0) yield break;
            CpuMove();
        }

        private void SetCpuAuto(bool on)
        {
            cpuAutoEnabled = on;
            if (!on && cpuAutoRoutine != null)
            {
                StopCoroutine(cpuAutoRoutine);
                cpuAutoRoutine = null;
            }
            SyncScoresAndMaybeTriggerCpu();
        }

        private void CpuMove()
        {
            if (cpuAutoRoutine != null)
            {
                StopCoroutine(cpuAutoRoutine);
                cpuAutoRoutine = null;
            }
            if (cpuBusy)
            {
                Debug.LogWarning("CPU move already in progress");
                return;
            }
            cpuBusy = true;
            try
            {
                var best = engine.BestMove(cpuDifficulty, 42);
                if (string.IsNullOrEmpty(best))
                {
                    Debug.LogWarning($"best_move error: {Engine.LastError()}");
                    statusDetail = "CPU move failed";
                    RenderStatus();
                    return;
                }
                if (best == "null")
                {
                    statusDetail = "CPU: no legal move";
                    RenderStatus();
                    return;
                }
                using var doc = JsonDocument.Parse(best);
                var placementsEl = doc.RootElement.GetProperty("placements");
                var placements = new List<Dictionary<string, object?>>();
                foreach (var el in placementsEl.EnumerateArray())
                {
                    var entry = new Dictionary<string, object?>
                    {
                        ["x"] = el.GetProperty("x").GetInt32(),
                        ["y"] = el.GetProperty("y").GetInt32(),
                        ["kind_id"] = el.GetProperty("kind_id").GetString() ?? string.Empty,
                    };
                    if (el.TryGetProperty("mark", out var markEl) && markEl.ValueKind == JsonValueKind.String)
                    {
                        entry["mark"] = markEl.GetString();
                    }
                    placements.Add(entry);
                }
                if (placements.Count == 0)
                {
                    Debug.LogWarning("CPU move returned no placements");
                    statusDetail = "CPU move had no placements";
                    RenderStatus();
                    return;
                }
                var placementsJson = JsonSerializer.Serialize(placements);
                var preview = engine.PreviewMoveJson(placementsJson);
                if (!string.IsNullOrEmpty(preview))
                {
                    ApplyPreviewHighlights(preview);
                }
                PushSnapshot();
                var res = engine.PlayMove(placementsJson);
                if (res == null)
                {
                    Debug.LogError($"play_move error: {Engine.LastError()}");
                    statusDetail = "CPU move failed";
                    RenderStatus();
                }
                else
                {
                    UpdateScoreOverlay(res);
                }
                staged.Clear();
                RefreshBoard();
                RefreshRack();
            }
            catch (Exception e)
            {
                Debug.LogError($"CPU move failed: {e}");
                statusDetail = "CPU move failed";
                RenderStatus();
            }
            finally
            {
                cpuBusy = false;
                SyncScoresAndMaybeTriggerCpu();
            }
        }

        private string BuildConfigJson()
        {
            var baseCfg = new Dictionary<string, object?>
            {
                ["tileset"] = new { tile_kinds = new object[] {
                    new { id = "A", symbol = "A", score = 1 },
                    new { id = "B", symbol = "B", score = 3 },
                }},
                ["rack_size"] = 7,
                ["ruleset_id"] = "cross",
                ["dictionary_id"] = "en",
                ["rng_seed"] = 42,
                ["tile_counts"] = new { A = 10, B = 10 },
                ["free_word_mode"] = freeWordMode,
            };
            if (use3D)
            {
                baseCfg["board_layout"] = new { type = "3d", width, height, depth = Mathf.Max(1, depth) };
            }
            else
            {
                bool shapeMask = shape == ShapeMask.Diamond;
                var nodes = new List<object>();
                var coordToIndex = new Dictionary<string, int>();
                for (int y = 0; y < height; y++)
                for (int x = 0; x < width; x++)
                {
                    if (shapeMask && !IsActiveInMask(x, y)) continue;
                    coordToIndex[$"{x},{y}"] = nodes.Count;
                    nodes.Add(new { x, y });
                }
                var edges = new List<object>();
                void AddEdge(int x1, int y1, int x2, int y2, string dir)
                {
                    if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
                    if (shapeMask && (!IsActiveInMask(x1, y1) || !IsActiveInMask(x2, y2))) return;
                    if (!coordToIndex.TryGetValue($"{x1},{y1}", out var a)) return;
                    if (!coordToIndex.TryGetValue($"{x2},{y2}", out var b)) return;
                    edges.Add(new { a, b, dir });
                }
                for (int y = 0; y < height; y++)
                for (int x = 0; x < width; x++)
                {
                    if (shapeMask && !IsActiveInMask(x, y)) continue;
                    if (adjacency == AdjacencyMode.Hex)
                    {
                        bool even = (y % 2) == 0;
                        int eastShift = even ? 0 : 1;
                        int westShift = even ? -1 : 0;
                        AddEdge(x, y, x + 1, y, "E");
                        AddEdge(x, y, x - 1, y, "W");
                        AddEdge(x, y, x + eastShift, y - 1, "NE");
                        AddEdge(x, y, x + westShift, y - 1, "NW");
                        AddEdge(x, y, x + eastShift, y + 1, "SE");
                        AddEdge(x, y, x + westShift, y + 1, "SW");
                    }
                    else if (adjacency == AdjacencyMode.Diagonal)
                    {
                        AddEdge(x, y, x + 1, y, "E");
                        AddEdge(x, y, x - 1, y, "W");
                        AddEdge(x, y, x, y - 1, "N");
                        AddEdge(x, y, x, y + 1, "S");
                        AddEdge(x, y, x + 1, y - 1, "NE");
                        AddEdge(x, y, x - 1, y - 1, "NW");
                        AddEdge(x, y, x + 1, y + 1, "SE");
                        AddEdge(x, y, x - 1, y + 1, "SW");
                    }
                    else
                    {
                        AddEdge(x, y, x + 1, y, "E");
                        AddEdge(x, y, x - 1, y, "W");
                        AddEdge(x, y, x, y - 1, "N");
                        AddEdge(x, y, x, y + 1, "S");
                    }
                }
                if (adjacency == AdjacencyMode.Orthogonal && !shapeMask)
                {
                    baseCfg["board_layout"] = new { width, height };
                }
                else
                {
                    baseCfg["board_layout"] = new { width, height, type = "graph", nodes, edges };
                }
            }
            return JsonSerializer.Serialize(baseCfg);
        }

        // UI helpers
        private Canvas EnsureCanvas()
        {
            var canvas = FindObjectOfType<Canvas>();
            if (canvas != null) return canvas;
            var canGo = new GameObject("Canvas", typeof(RectTransform), typeof(Canvas), typeof(CanvasScaler), typeof(GraphicRaycaster));
            var c = canGo.GetComponent<Canvas>();
            c.renderMode = RenderMode.ScreenSpaceOverlay;
            var scale = canGo.GetComponent<CanvasScaler>();
            scale.uiScaleMode = CanvasScaler.ScaleMode.ScaleWithScreenSize;
            if (FindObjectOfType<EventSystem>() == null)
            {
                var es = new GameObject("EventSystem", typeof(EventSystem), typeof(StandaloneInputModule));
                es.transform.SetParent(transform, false);
            }
            return c;
        }

        private void EnsureUiRoot()
        {
            // Intentionally empty: BuildGrid ensures Canvas exists
        }

        private Button CreateCellButton(Transform parent)
        {
            var go = new GameObject("Cell", typeof(RectTransform), typeof(Image), typeof(Button));
            go.transform.SetParent(parent, false);
            var img = go.GetComponent<Image>();
            img.color = new Color(0.92f, 0.92f, 0.92f, 1f);
            var btn = go.GetComponent<Button>();
            var drop = go.AddComponent<BoardCellDropTarget>();
            drop.Board = this;
            // label
            var textGo = new GameObject("Text", typeof(RectTransform), typeof(Text));
            textGo.transform.SetParent(go.transform, false);
            var rt = textGo.GetComponent<RectTransform>();
            rt.anchorMin = Vector2.zero; rt.anchorMax = Vector2.one; rt.offsetMin = Vector2.zero; rt.offsetMax = Vector2.zero;
            var txt = textGo.GetComponent<Text>();
            txt.text = "";
            txt.alignment = TextAnchor.MiddleCenter;
            txt.color = Color.black;
            txt.font = Resources.GetBuiltinResource<Font>("Arial.ttf");
            return btn;
        }

        private void SetCellText(Button btn, string s)
        {
            var txt = btn.GetComponentInChildren<Text>();
            if (txt != null) txt.text = s ?? string.Empty;
        }

        private Button CreateRackButton(Transform parent, string label, int score)
        {
            var go = new GameObject("RackTile", typeof(RectTransform), typeof(Image), typeof(Button));
            go.transform.SetParent(parent, false);
            var img = go.GetComponent<Image>();
            img.color = new Color(0.85f, 0.85f, 1f, 1f);
            var btn = go.GetComponent<Button>();
            var textGo = new GameObject("Text", typeof(RectTransform), typeof(Text));
            textGo.transform.SetParent(go.transform, false);
            var rt = textGo.GetComponent<RectTransform>();
            rt.anchorMin = Vector2.zero; rt.anchorMax = Vector2.one; rt.offsetMin = Vector2.zero; rt.offsetMax = Vector2.zero;
            var txt = textGo.GetComponent<Text>();
            txt.text = $"{label}\n{score}";
            txt.alignment = TextAnchor.MiddleCenter;
            txt.color = Color.black;
            txt.font = Resources.GetBuiltinResource<Font>("Arial.ttf");
            return btn;
        }

        private void HighlightSelected(Button selected)
        {
            foreach (var b in rackButtons)
            {
                var img = b.GetComponent<Image>();
                img.color = new Color(0.85f, 0.85f, 1f, 1f);
            }
            var selImg = selected.GetComponent<Image>();
            selImg.color = new Color(0.95f, 0.95f, 0.6f, 1f);
        }

        private Button CreateButton(Transform parent, string label)
        {
            var go = new GameObject(label, typeof(RectTransform), typeof(Image), typeof(Button));
            go.transform.SetParent(parent, false);
            var img = go.GetComponent<Image>();
            img.color = new Color(0.2f, 0.4f, 0.8f, 0.9f);
            var btn = go.GetComponent<Button>();
            var textGo = new GameObject("Text", typeof(RectTransform), typeof(Text));
            textGo.transform.SetParent(go.transform, false);
            var rt = textGo.GetComponent<RectTransform>();
            rt.anchorMin = Vector2.zero; rt.anchorMax = Vector2.one; rt.offsetMin = Vector2.zero; rt.offsetMax = Vector2.zero;
            var txt = textGo.GetComponent<Text>();
            txt.text = label;
            txt.alignment = TextAnchor.MiddleCenter;
            txt.color = Color.white;
            txt.font = Resources.GetBuiltinResource<Font>("Arial.ttf");
            return btn;
        }

        private Toggle CreateToggle(Transform parent, string label, bool initial)
        {
            var go = new GameObject($"{label}Toggle", typeof(RectTransform), typeof(Image), typeof(Toggle));
            go.transform.SetParent(parent, false);
            var rect = go.GetComponent<RectTransform>();
            rect.sizeDelta = new Vector2(140, 28);
            rect.pivot = new Vector2(0f, 0.5f);
            rect.anchorMin = new Vector2(0f, 0.5f);
            rect.anchorMax = new Vector2(0f, 0.5f);

            var background = go.GetComponent<Image>();
            background.color = new Color(0.15f, 0.15f, 0.15f, 0.9f);

            var toggle = go.GetComponent<Toggle>();
            toggle.isOn = initial;
            toggle.targetGraphic = background;

            var checkGo = new GameObject("Checkmark", typeof(RectTransform), typeof(Image));
            checkGo.transform.SetParent(go.transform, false);
            var checkRect = checkGo.GetComponent<RectTransform>();
            checkRect.anchorMin = new Vector2(0f, 0.5f);
            checkRect.anchorMax = new Vector2(0f, 0.5f);
            checkRect.pivot = new Vector2(0f, 0.5f);
            checkRect.anchoredPosition = new Vector2(8f, 0f);
            checkRect.sizeDelta = new Vector2(16f, 16f);
            var checkImg = checkGo.GetComponent<Image>();
            checkImg.color = new Color(0.2f, 0.8f, 0.2f, 1f);
            toggle.graphic = checkImg;

            var textGo = new GameObject("Label", typeof(RectTransform), typeof(Text));
            textGo.transform.SetParent(go.transform, false);
            var textRect = textGo.GetComponent<RectTransform>();
            textRect.anchorMin = new Vector2(0f, 0f);
            textRect.anchorMax = new Vector2(1f, 1f);
            textRect.offsetMin = new Vector2(28f, 0f);
            textRect.offsetMax = new Vector2(0f, 0f);
            var text = textGo.GetComponent<Text>();
            text.text = label;
            text.alignment = TextAnchor.MiddleLeft;
            text.color = Color.white;
            text.font = Resources.GetBuiltinResource<Font>("Arial.ttf");

            return toggle;
        }

        // Staging API
        public void AddStagedPlacement(int x, int y, string kindId)
        {
            // replace if same cell exists
            int idx = staged.FindIndex(p => p.x == x && p.y == y);
            if (idx >= 0) staged[idx] = (x, y, kindId);
            else staged.Add((x, y, kindId));
            UpdatePreviewOverlay();
            RefreshBoard();
        }

        private void CommitStaged()
        {
            if (staged.Count == 0) return;
            var placements = staged.ConvertAll(p => new { x = p.x, y = GlobalY(p.y), kind_id = p.kindId });
            var json = JsonSerializer.Serialize(placements);
            var res = engine.PlayMove(json);
            if (res == null)
            {
                Debug.LogError($"play_move error: {Engine.LastError()}");
            }
            else
            {
                UpdateScoreOverlay(res);
            }
            staged.Clear();
            hlMain.Clear(); hlCross.Clear();
            RefreshBoard();
            RefreshRack();
            SyncScoresAndMaybeTriggerCpu();
        }

        private void ClearStaged()
        {
            staged.Clear();
            hlMain.Clear(); hlCross.Clear();
            RefreshBoard();
            UpdatePreviewOverlay();
        }

        private void UpdatePreviewOverlay()
        {
            if (staged.Count == 0)
            {
                hlMain.Clear(); hlCross.Clear();
                statusDetail = string.Empty;
                RenderStatus();
                return;
            }
            var placements = staged.ConvertAll(p => new { x = p.x, y = p.y, kind_id = p.kindId });
            var json = JsonSerializer.Serialize(placements);
            var res = engine.PreviewMoveJson(json);
            if (res == null) { return; }
            try
            {
                var doc = JsonDocument.Parse(res);
                var valid = doc.RootElement.GetProperty("valid").GetBoolean();
                var total = doc.RootElement.GetProperty("total").GetInt32();
                var mainWord = doc.RootElement.GetProperty("main_word").GetString() ?? "";
                var mainScore = doc.RootElement.GetProperty("main_score").GetInt32();
                var cross = doc.RootElement.GetProperty("cross_words");
                var crossSum = 0; foreach (var cw in cross.EnumerateArray()) crossSum += cw[1].GetInt32();
                statusDetail = valid ? $"Preview: {mainWord} total={total} (main={mainScore} +cross={crossSum})" : "Preview: invalid";
                RenderStatus();
                hlMain = new System.Collections.Generic.List<Vector2Int>();
                hlCross = new System.Collections.Generic.List<System.Collections.Generic.List<Vector2Int>>();
                foreach (var c in doc.RootElement.GetProperty("main_cells").EnumerateArray())
                {
                    hlMain.Add(new Vector2Int(c[0].GetInt32(), c[1].GetInt32()));
                }
                foreach (var arr in doc.RootElement.GetProperty("cross_cells").EnumerateArray())
                {
                    var list = new System.Collections.Generic.List<Vector2Int>();
                    foreach (var c in arr.EnumerateArray()) list.Add(new Vector2Int(c[0].GetInt32(), c[1].GetInt32()));
                    hlCross.Add(list);
                }
            }
            catch
            {
                statusDetail = string.Empty;
                RenderStatus();
            }
        }

        // --- Parity helpers ---
        private int GlobalY(int localY) => use3D ? (zLayer * height + localY) : localY;

        private void SetZLayer(int next)
        {
            if (!use3D) return;
            int maxLayer = Math.Max(0, depth - 1);
            zLayer = Mathf.Clamp(next, 0, maxLayer);
            RefreshBoard();
        }

        private void CycleAdjacency(Button labelBtn)
        {
            adjacency = adjacency switch
            {
                AdjacencyMode.Orthogonal => AdjacencyMode.Diagonal,
                AdjacencyMode.Diagonal => AdjacencyMode.Hex,
                _ => AdjacencyMode.Orthogonal,
            };
            labelBtn.GetComponentInChildren<Text>().text = $"Adj: {adjacency}";
            BuildOrRebuildEngine();
            RebuildGrid();
            RefreshBoard();
        }

        private void CycleShape(Button labelBtn)
        {
            shape = shape == ShapeMask.Rect ? ShapeMask.Diamond : ShapeMask.Rect;
            labelBtn.GetComponentInChildren<Text>().text = $"Shape: {shape}";
            BuildOrRebuildEngine();
            RebuildGrid();
            RefreshBoard();
        }

        private void ResetHistory()
        {
            history.Clear();
            historyIndex = -1;
            PushSnapshot();
        }

        private void PushSnapshot()
        {
            var snap = engine.SnapshotStateJson();
            if (string.IsNullOrEmpty(snap)) return;
            if (historyIndex >= 0 && historyIndex < history.Count - 1)
            {
                history.RemoveRange(historyIndex + 1, history.Count - (historyIndex + 1));
            }
            history.Add(snap);
            historyIndex = history.Count - 1;
        }

        private void Undo()
        {
            if (history.Count == 0) return;
            if (historyIndex <= 0) return;
            historyIndex--;
            if (engine.RestoreStateJson(history[historyIndex]))
            {
                staged.Clear(); hlMain.Clear(); hlCross.Clear();
                RefreshBoard();
                RefreshRack();
                SyncScoresAndMaybeTriggerCpu();
            }
        }

        private void Redo()
        {
            if (history.Count == 0) return;
            if (historyIndex >= history.Count - 1) return;
            historyIndex++;
            if (engine.RestoreStateJson(history[historyIndex]))
            {
                staged.Clear(); hlMain.Clear(); hlCross.Clear();
                RefreshBoard();
                RefreshRack();
                SyncScoresAndMaybeTriggerCpu();
            }
        }

        private void ApplyAutoBonuses()
        {
            try
            {
                var bonuses = ResolveAutoBonuses();
                var json = JsonSerializer.Serialize(bonuses);
                engine.SetBonuses(json);
            }
            catch (Exception e)
            {
                Debug.LogWarning($"Failed to set bonuses: {e.Message}");
            }
        }

        private struct BonusCell { public int x; public int y; public int? letter_mul; public int? word_mul; public string[]? tags; }

        private BonusCell[] ResolveAutoBonuses()
        {
            var list = new List<BonusCell>();
            int W = width; int H = height;
            int centerX = (W - 1) / 2;
            int centerY = (H - 1) / 2;
            for (int y = 0; y < H; y++)
            {
                for (int x = 0; x < W; x++)
                {
                    if (!IsActiveInMask(x, y)) continue;
                    int edgeDist = Math.Min(Math.Min(x, W - 1 - x), Math.Min(y, H - 1 - y));
                    if (edgeDist == 0)
                    {
                        list.Add(new BonusCell { x = x, y = GlobalY(y), word_mul = 3 });
                        continue;
                    }
                    if (edgeDist == 1)
                    {
                        list.Add(new BonusCell { x = x, y = GlobalY(y), word_mul = 2 });
                        continue;
                    }
                    int manhattanCenter = Math.Abs(x - centerX) + Math.Abs(y - centerY);
                    if (manhattanCenter == 0)
                    {
                        continue;
                    }
                    if (manhattanCenter % 4 == 0)
                    {
                        list.Add(new BonusCell { x = x, y = GlobalY(y), letter_mul = 3 });
                    }
                    else if (((x + y) % 3) == 0)
                    {
                        list.Add(new BonusCell { x = x, y = GlobalY(y), letter_mul = 2 });
                    }
                }
            }
            int mx = Mathf.RoundToInt(centerX);
            int my = Mathf.RoundToInt(centerY);
            if (mx >= 0 && mx < W && my >= 0 && my < H)
            {
                list.Add(new BonusCell { x = mx, y = GlobalY(my), word_mul = 2, tags = new[] { "center" } });
            }
            if (adjacency == AdjacencyMode.Hex || shape == ShapeMask.Diamond)
            {
                for (int y = 0; y < H; y++)
                {
                    for (int x = 0; x < W; x++)
                    {
                        if (!IsActiveInMask(x, y)) continue;
                        float axialQ = x - centerX;
                        float axialR = y - centerY;
                        float axialS = -axialQ - axialR;
                        float radius = Math.Max(Math.Max(Math.Abs(axialQ), Math.Abs(axialR)), Math.Abs(axialS));
                        if (Math.Abs(radius - 2f) < 1e-6)
                        {
                            list.Add(new BonusCell { x = x, y = GlobalY(y), letter_mul = 3, tags = new[] { "hex" } });
                        }
                        else if (Math.Abs(radius - 3f) < 1e-6)
                        {
                            list.Add(new BonusCell { x = x, y = GlobalY(y), letter_mul = 2, tags = new[] { "hex" } });
                        }
                    }
                }
            }
            return list.ToArray();
        }

        private bool IsActiveInMask(int x, int y)
        {
            if (shape == ShapeMask.Rect) return true;
            int cx = (width - 1) / 2; int cy = (height - 1) / 2;
            int r = Math.Min(cx, cy);
            return Math.Abs(x - cx) + Math.Abs(y - cy) <= r;
        }

        private void FetchMoves()
        {
            if (use3D)
            {
                statusDetail = "3D move generation not available";
                RenderStatus();
                return;
            }
            try
            {
                var json = engine.GenerateMoves(7, 30);
                foreach (var b in moveButtons) Destroy(b.gameObject);
                moveButtons.Clear();
                if (string.IsNullOrEmpty(json)) return;
                using var doc = JsonDocument.Parse(json);
                foreach (var el in doc.RootElement.EnumerateArray())
                {
                    var word = el.TryGetProperty("word", out var wEl) && wEl.ValueKind == JsonValueKind.String ? wEl.GetString() ?? "" : "";
                    var score = el.TryGetProperty("score", out var sEl) && sEl.ValueKind == JsonValueKind.Number ? sEl.GetInt32() : 0;
                    var btn = CreateButton(movesPanel.transform, $"{word} (+{score})");
                    // Capture element by value
                    var placements = el.GetProperty("placements").Clone();
                    btn.onClick.AddListener(() => ApplyGeneratedMove(placements));
                    moveButtons.Add(btn);
                }
            }
            catch (Exception e)
            {
                Debug.LogWarning($"generate_moves failed: {e.Message}");
            }
        }

        private void ApplyGeneratedMove(JsonElement placements)
        {
            try
            {
                var list = new List<Dictionary<string, object?>>();
                foreach (var p in placements.EnumerateArray())
                {
                    var entry = new Dictionary<string, object?>
                    {
                        ["x"] = p.GetProperty("x").GetInt32(),
                        ["y"] = p.GetProperty("y").GetInt32(),
                        ["kind_id"] = p.GetProperty("kind_id").GetString() ?? string.Empty,
                    };
                    if (p.TryGetProperty("mark", out var mEl) && mEl.ValueKind == JsonValueKind.String)
                    {
                        entry["mark"] = mEl.GetString();
                    }
                    list.Add(entry);
                }
                var placementsJson = JsonSerializer.Serialize(list);
                PushSnapshot();
                var preview = engine.PreviewMoveJson(placementsJson);
                if (!string.IsNullOrEmpty(preview)) ApplyPreviewHighlights(preview);
                var res = engine.PlayMove(placementsJson);
                if (res == null)
                {
                    Debug.LogError($"play_move error: {Engine.LastError()}");
                }
                else
                {
                    UpdateScoreOverlay(res);
                }
                staged.Clear();
                RefreshBoard();
                RefreshRack();
                SyncScoresAndMaybeTriggerCpu();
            }
            catch (Exception e)
            {
                Debug.LogWarning($"ApplyGeneratedMove failed: {e.Message}");
            }
        }
    }
}
