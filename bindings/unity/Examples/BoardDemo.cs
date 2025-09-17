using System;
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
        public int height = 5;
        public bool useHexGeometry = true;
        public bool freeWordMode = true;

        [Header("UI Settings")]
        public Vector2 cellSize = new Vector2(48, 48);
        public Vector2 cellSpacing = new Vector2(6, 6);

        private Engine engine = new Engine();
        private GridLayoutGroup grid;
        private readonly List<Button> cells = new();
        private HorizontalLayoutGroup rackBar;
        private readonly List<Button> rackButtons = new();
        private string? selectedKindId;
        private Text scoreLabel;
        private readonly System.Collections.Generic.List<(int x, int y, string kindId)> staged = new();
        private System.Collections.Generic.List<Vector2Int> hlMain = new();
        private System.Collections.Generic.List<System.Collections.Generic.List<Vector2Int>> hlCross = new();
        private Button cpuButton;
        private Button cpuDifficultyButton;
        private bool cpuBusy;
        private string cpuDifficulty = "medium";

        void Start()
        {
            EnsureUiRoot();
            BuildOrRebuildEngine();
            BuildGrid();
            RefreshBoard();
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

        public void ToggleGeometry(bool hex)
        {
            useHexGeometry = hex;
            BuildOrRebuildEngine();
            RebuildGrid();
            RefreshBoard();
        }

        private void BuildOrRebuildEngine()
        {
            var cfg = BuildConfigJson();
            if (!engine.NewGame(cfg, 2))
            {
                Debug.LogError($"Failed to create game: {Engine.LastError()}");
            }
            engine.SetFreeWordMode(freeWordMode);
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

            var freeToggle = CreateToggle(bar.transform, "Free Word Mode", freeWordMode);
            freeToggle.onValueChanged.AddListener(ToggleFreeWordMode);
            var geomToggle = CreateToggle(bar.transform, "Hex Geometry", useHexGeometry);
            geomToggle.onValueChanged.AddListener(ToggleGeometry);

            // Score label
            var scoreGo = new GameObject("Score", typeof(RectTransform), typeof(Text));
            scoreGo.transform.SetParent(bar.transform, false);
            var t = scoreGo.GetComponent<Text>();
            t.font = Resources.GetBuiltinResource<Font>("Arial.ttf");
            t.alignment = TextAnchor.MiddleLeft;
            t.color = Color.white;
            t.text = "";
            scoreLabel = t;

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

            // Commit / Cancel buttons
            var commit = CreateButton(bar.transform, "Commit Move");
            commit.onClick.AddListener(CommitStaged);
            var cancel = CreateButton(bar.transform, "Cancel");
            cancel.onClick.AddListener(ClearStaged);

            cpuDifficultyButton = CreateButton(bar.transform, "Difficulty: Medium");
            cpuDifficultyButton.onClick.AddListener(CycleDifficulty);

            cpuButton = CreateButton(bar.transform, "CPU Move (Medium)");
            cpuButton.onClick.AddListener(CpuMove);
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
            var placements = new[] { new { x, y, kind_id = "A" } };
            if (!string.IsNullOrEmpty(selectedKindId))
                placements = new[] { new { x, y, kind_id = selectedKindId } };
            var json = JsonSerializer.Serialize(placements);
            var res = engine.PlayMove(json);
            if (res == null)
                Debug.LogError($"play_move error: {Engine.LastError()}");
            else UpdateScoreOverlay(res);
            RefreshBoard();
            RefreshRack();
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
                for (int y = 0; y < height; y++)
                for (int x = 0; x < width; x++)
                {
                    var sym = rows[y][x].GetString() ?? string.Empty;
                    SetCellText(cells[i++], sym);
                }
                // Overlay preview/committed highlights
                foreach (var v in hlMain)
                {
                    int idx = v.y * width + v.x;
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
                        int idx = v.y * width + v.x;
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
                scoreLabel.text = $"Last: {mainWord} total={total} (main={mainScore} +cross={crossSum})";
            }
            catch (Exception e)
            {
                Debug.LogError(e);
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

        private void CpuMove()
        {
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
                    return;
                }
                if (best == "null")
                {
                    scoreLabel.text = "CPU: no legal move";
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
                    return;
                }
                var placementsJson = JsonSerializer.Serialize(placements);
                var preview = engine.PreviewMoveJson(placementsJson);
                if (!string.IsNullOrEmpty(preview))
                {
                    ApplyPreviewHighlights(preview);
                }
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
            }
            catch (Exception e)
            {
                Debug.LogError($"CPU move failed: {e}");
            }
            finally
            {
                cpuBusy = false;
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
            if (useHexGeometry)
            {
                var nodes = new List<object>();
                for (int y = 0; y < height; y++)
                    for (int x = 0; x < width; x++)
                        nodes.Add(new { x, y });
                int Idx(int x, int y) => y * width + x;
                var edges = new List<object>();
                void TryEdge(int x1, int y1, int x2, int y2, string dir)
                {
                    if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
                    edges.Add(new { a = Idx(x1, y1), b = Idx(x2, y2), dir });
                }
                for (int y = 0; y < height; y++)
                for (int x = 0; x < width; x++)
                {
                    bool even = (y % 2) == 0;
                    TryEdge(x, y, x + 1, y, "E");
                    TryEdge(x, y, x + (even ? 0 : 1), y - 1, "NE");
                    TryEdge(x, y, x + (even ? 0 : 1), y + 1, "SE");
                }
                baseCfg["board_layout"] = new { width, height, type = "graph", nodes, edges };
            }
            else
            {
                baseCfg["board_layout"] = new { width, height };
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
            var placements = staged.ConvertAll(p => new { x = p.x, y = p.y, kind_id = p.kindId });
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
        }

        private void ClearStaged()
        {
            staged.Clear();
            hlMain.Clear(); hlCross.Clear();
            RefreshBoard();
        }

        private void UpdatePreviewOverlay()
        {
            if (staged.Count == 0)
            {
                hlMain.Clear(); hlCross.Clear();
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
                scoreLabel.text = valid ? $"Preview: {mainWord} total={total} (main={mainScore} +cross={crossSum})" : "Preview: invalid";
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
            catch { }
        }
    }
}
