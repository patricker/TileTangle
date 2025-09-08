#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../include/engine.h"

static const char* cfg_json =
    "{\n"
    "  \"tileset\": {\"tile_kinds\": ["
    "{\"id\": \"A\", \"symbol\": \"A\", \"score\": 1},"
    "{\"id\": \"B\", \"symbol\": \"B\", \"score\": 3}"
    "]},\n"
    "  \"rack_size\": 7,\n"
    "  \"board_layout\": {\"width\": 5, \"height\": 5},\n"
    "  \"ruleset_id\": \"cross\",\n"
    "  \"dictionary_id\": \"en\",\n"
    "  \"rng_seed\": 42,\n"
    "  \"tile_counts\": {\"A\": 10, \"B\": 10},\n"
    "  \"free_word_mode\": true\n"
    "}";

int main(void) {
    GameHandle* g = tt_new_game(cfg_json, 2);
    if (!g) {
        const char* err = tt_last_error_message();
        fprintf(stderr, "tt_new_game failed: %s\n", err ? err : "(null)");
        return 1;
    }
    char* b = tt_get_board(g);
    if (!b) { fprintf(stderr, "get_board null\n"); return 1; }
    tt_string_free(b);
    const char* placements = "[{\"x\":2,\"y\":2,\"kind_id\":\"A\"},{\"x\":3,\"y\":2,\"kind_id\":\"B\"}]";
    char* score = tt_play_move(g, placements);
    if (!score) {
        const char* err = tt_last_error_message();
        fprintf(stderr, "tt_play_move failed: %s\n", err ? err : "(null)");
        tt_free_game(g);
        return 1;
    }
    tt_string_free(score);
    tt_free_game(g);
    puts("OK");
    return 0;
}

