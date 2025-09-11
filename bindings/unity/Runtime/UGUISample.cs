using System;
using System.Text.Json;
using System.Text.Json.Serialization;
using UnityEngine;
using UnityEngine.UI;

namespace TileTangle
{
    public class UGUISample : MonoBehaviour
    {
        [Header("UI wiring")]
        public RectTransform gridRoot;
        public Button cellButtonPrefab;
        public int width = 5;
        public int height = 5;
        public string placeKindId = "A";
        public bool useHex = true;
        public bool freeWordMode = true;

        private Engine engine = new Engine();

        [Serializable]
        private class BoardJson
        {
            public int width;
            public int height;
            public string[][] rows;
        }

        void Start()
        {
            if (gridRoot == null || cellButtonPrefab == null)
            {
                Debug.LogError("UGUISample: Assign gridRoot and cellButtonPrefab in inspector.");
                return;
            }

            var cfg = BuildConfig();
            string cfgJson = JsonSerializer.Serialize(cfg);
            if (!engine.NewGame(cfgJson, 2))
            {
                Debug.LogError($"Failed to create game: {Engine.LastError()}");
                return;
            }

            BuildGrid();
            engine.SetFreeWordMode(freeWordMode);
            RefreshBoard();
        }

        private object BuildConfig()
        {
            var baseCfg = new
            {
                tileset = new
                {
                    tile_kinds = new object[]
                    {
                        new { id = "A", symbol = "A", score = 1 },
                        new { id = "B", symbol = "B", score = 3 },
                    }
                },
                rack_size = 7,
                ruleset_id = "cross",
                dictionary_id = "en",
                rng_seed = 42,
                tile_counts = new { A = 10, B = 10 },
                free_word_mode = true,
            };
            if (!useHex)
            {
                return new { board_layout = new { width = this.width, height = this.height }, baseCfg.tileset, baseCfg.rack_size, baseCfg.ruleset_id, baseCfg.dictionary_id, baseCfg.rng_seed, baseCfg.tile_counts, baseCfg.free_word_mode };
            }
            var nodes = new System.Collections.Generic.List<object>();
            for (int y = 0; y < height; y++) for (int x = 0; x < width; x++) nodes.Add(new { x, y });
            int Index(int x, int y) => y * width + x;
            var edges = new System.Collections.Generic.List<object>();
            void TryEdge(int x1, int y1, int x2, int y2, string dir)
            {
                if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
                edges.Add(new { a = Index(x1, y1), b = Index(x2, y2), dir });
            }
            for (int y = 0; y < height; y++)
            {
                for (int x = 0; x < width; x++)
                {
                    bool even = (y % 2) == 0;
                    TryEdge(x, y, x + 1, y, "E");
                    TryEdge(x, y, x + (even ? 0 : 1), y - 1, "NE");
                    TryEdge(x, y, x + (even ? 0 : 1), y + 1, "SE");
                }
            }
            var board_layout = new { width = this.width, height = this.height, type = "graph", nodes, edges };
            return new { board_layout, baseCfg.tileset, baseCfg.rack_size, baseCfg.ruleset_id, baseCfg.dictionary_id, baseCfg.rng_seed, baseCfg.tile_counts, baseCfg.free_word_mode };
        }

        private void BuildGrid()
        {
            foreach (Transform child in gridRoot)
                Destroy(child.gameObject);

            var grid = gridRoot.GetComponent<GridLayoutGroup>();
            if (grid != null)
            {
                grid.constraint = GridLayoutGroup.Constraint.FixedColumnCount;
                grid.constraintCount = width;
            }

            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                var btn = Instantiate(cellButtonPrefab, gridRoot);
                var label = btn.GetComponentInChildren<TMPro.TMP_Text>();
                if (label != null) label.text = "";
                int cx = x, cy = y;
                btn.onClick.AddListener(() => OnCellClicked(cx, cy));
            }
        }

        private void OnCellClicked(int x, int y)
        {
            var move = $"[{\"x\":{x},\"y\":{y},\"kind_id\":\"{placeKindId}\"}]";
            var res = engine.PlayMove(move);
            if (res == null)
                Debug.LogError($"PlayMove error: {Engine.LastError()}");
            RefreshBoard();
        }

        private void RefreshBoard()
        {
            var json = engine.GetBoardJson();
            if (json == null) return;
            var board = JsonSerializer.Deserialize<BoardJson>(json);
            if (board == null) return;

            int i = 0;
            foreach (Transform child in gridRoot)
            {
                int x = i % width;
                int y = i / width;
                string kind = board.rows[y][x] ?? string.Empty;
                var label = child.GetComponentInChildren<TMPro.TMP_Text>();
                if (label != null) label.text = kind;
                i++;
            }
        }

        // Hook this to a UI Toggle
        public void OnToggleFreeWordMode(bool on)
        {
            freeWordMode = on;
            if (engine != null) engine.SetFreeWordMode(on);
        }
    }
}
