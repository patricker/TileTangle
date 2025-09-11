using System;
using TileTangle;

class ConsoleSmoke
{
    static void Main()
    {
        var cfg = new {
            tileset = new {
                tile_kinds = new object[] {
                    new { id = "A", symbol = "A", score = 1 },
                    new { id = "B", symbol = "B", score = 3 }
                }
            },
            rack_size = 7,
            board_layout = new { width = 5, height = 5 },
            ruleset_id = "cross",
            dictionary_id = "en",
            rng_seed = 42,
            tile_counts = new { A = 9, B = 2 },
            free_word_mode = true,
        };
        string cfgJson = System.Text.Json.JsonSerializer.Serialize(cfg);

        using var eng = new Engine();
        if (!eng.NewGame(cfgJson, 2))
        {
            Console.Error.WriteLine($"Failed to create game: {Engine.LastError()}");
            Environment.Exit(1);
        }
        var board = eng.GetBoardJson();
        Console.WriteLine($"Board: {board}");
        var move = "[{\"x\":2,\"y\":2,\"kind_id\":\"A\"},{\"x\":3,\"y\":2,\"kind_id\":\"B\"}]";
        var res = eng.PlayMove(move);
        if (res == null) Console.Error.WriteLine($"play_move error: {Engine.LastError()}");
        else Console.WriteLine($"Score: {res}");
    }
}

