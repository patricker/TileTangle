using NUnit.Framework;
using TileTangle;

public class EngineSmokeTest
{
    [Test]
    public void CanCreateGameAndGetBoard()
    {
        var cfg = new {
            tileset = new { tile_kinds = new object[] {
                new { id = "A", symbol = "A", score = 1 },
                new { id = "B", symbol = "B", score = 3 }
            }},
            rack_size = 7,
            board_layout = new { width = 5, height = 5 },
            ruleset_id = "cross",
            dictionary_id = "en",
            rng_seed = 1,
            tile_counts = new { A = 10, B = 10 },
            free_word_mode = true,
        };
        string cfgJson = System.Text.Json.JsonSerializer.Serialize(cfg);
        using var eng = new Engine();
        Assert.IsTrue(eng.NewGame(cfgJson, 2), Engine.LastError());
        var board = eng.GetBoardJson();
        Assert.IsNotNull(board, Engine.LastError());
    }
}

