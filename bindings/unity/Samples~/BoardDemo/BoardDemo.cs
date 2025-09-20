// Copy of Examples/BoardDemo.cs for Unity UPM Samples~
// See original under bindings/unity/Examples/BoardDemo.cs
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
        // This is a sample entry point that mirrors the example in the repo.
        // For full source and latest updates, see the Examples folder.
        [Header("Board Settings")]
        public int width = 5;
        public int height = 5;
        public bool freeWordMode = true;

        private Engine engine = new Engine();
        private GridLayoutGroup grid;
        private readonly List<Button> cells = new();

        void Start()
        {
            EnsureUi();
            var cfg = new {
                tileset = new { tile_kinds = new object[] {
                    new { id = "A", symbol = "A", score = 1 },
                    new { id = "B", symbol = "B", score = 3 }
                }},
                rack_size = 7,
                board_layout = new { width, height },
                ruleset_id = "cross",
                dictionary_id = "en",
                rng_seed = 42,
                tile_counts = new { A = 10, B = 10 },
                free_word_mode = freeWordMode,
            };
            string cfgJson = JsonSerializer.Serialize(cfg);
            if (!engine.NewGame(cfgJson, 2))
            {
                Debug.LogError($"Failed to create game: {Engine.LastError()}");
                return;
            }
            engine.SetFreeWordMode(freeWordMode);
            SpawnCells();
            RefreshBoard();
        }

        void OnDestroy() { engine.Dispose(); }

        private void EnsureUi()
        {
            var canvas = FindObjectOfType<Canvas>();
            if (canvas == null)
            {
                var canGo = new GameObject("Canvas", typeof(RectTransform), typeof(Canvas), typeof(CanvasScaler), typeof(GraphicRaycaster));
                var c = canGo.GetComponent<Canvas>(); c.renderMode = RenderMode.ScreenSpaceOverlay;
                var scale = canGo.GetComponent<CanvasScaler>(); scale.uiScaleMode = CanvasScaler.ScaleMode.ScaleWithScreenSize;
                if (FindObjectOfType<EventSystem>() == null)
                    new GameObject("EventSystem", typeof(EventSystem), typeof(StandaloneInputModule));
                canvas = c;
            }
            var gridGo = new GameObject("BoardGrid", typeof(RectTransform), typeof(GridLayoutGroup));
            gridGo.transform.SetParent(canvas.transform, false);
            grid = gridGo.GetComponent<GridLayoutGroup>();
            grid.constraint = GridLayoutGroup.Constraint.FixedColumnCount; grid.constraintCount = width;
            grid.cellSize = new Vector2(48,48); grid.spacing = new Vector2(6,6);
        }

        private void SpawnCells()
        {
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                var btn = CreateCellButton(grid.transform);
                int cx = x, cy = y;
                btn.onClick.AddListener(() => OnCellClicked(cx, cy));
                cells.Add(btn);
            }
        }

        private void OnCellClicked(int x, int y)
        {
            var move = $"[{\"x\":{x},\"y\":{y},\"kind_id\":\"A\"}]";
            var res = engine.PlayMove(move);
            if (res == null) Debug.LogError($"play_move error: {Engine.LastError()}");
            RefreshBoard();
        }

        private void RefreshBoard()
        {
            var boardJson = engine.GetBoardJson(); if (string.IsNullOrEmpty(boardJson)) return;
            using var doc = JsonDocument.Parse(boardJson);
            var rows = doc.RootElement.GetProperty("rows");
            int i = 0;
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                var sym = rows[y][x].GetString() ?? string.Empty;
                SetCellText(cells[i++], sym);
            }
        }

        private Button CreateCellButton(Transform parent)
        {
            var go = new GameObject("Cell", typeof(RectTransform), typeof(Image), typeof(Button));
            go.transform.SetParent(parent, false);
            var img = go.GetComponent<Image>(); img.color = new Color(0.92f, 0.92f, 0.92f, 1f);
            var txtGo = new GameObject("Text", typeof(RectTransform), typeof(Text));
            txtGo.transform.SetParent(go.transform, false);
            var txt = txtGo.GetComponent<Text>(); txt.font = Resources.GetBuiltinResource<Font>("Arial.ttf");
            txt.alignment = TextAnchor.MiddleCenter; txt.color = Color.black; txt.text = "";
            return go.GetComponent<Button>();
        }

        private void SetCellText(Button b, string s)
        {
            var t = b.GetComponentInChildren<Text>(); if (t != null) t.text = s;
        }
    }
}

