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

            SpawnCells();
        }

        private void RebuildGrid()
        {
            foreach (var b in cells) Destroy(b.gameObject);
            cells.Clear();
            grid.constraintCount = width;
            SpawnCells();
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
            var json = JsonSerializer.Serialize(placements);
            var res = engine.PlayMove(json);
            if (res == null)
                Debug.LogError($"play_move error: {Engine.LastError()}");
            RefreshBoard();
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
            }
            catch (Exception e)
            {
                Debug.LogError(e);
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
    }
}

