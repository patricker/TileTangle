using System;
using System.Runtime.InteropServices;

namespace TileTangle
{
    internal static class Native
    {
        const string LIB = "tiletangle_ffi"; // Resolves to libtiletangle_ffi.(so|dylib)/tiletangle_ffi.dll

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_new_game(IntPtr configJson, uint players);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern void tt_free_game(IntPtr game);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_play_move(IntPtr game, IntPtr placementsJson);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_get_board(IntPtr game);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_get_rack(IntPtr game);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_get_scores(IntPtr game);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_preview_move(IntPtr game, IntPtr placementsJson);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_best_move(IntPtr game, IntPtr difficulty, ulong seed, uint seedIsSome);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint tt_set_bonuses(IntPtr game, IntPtr bonusesJson);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern void tt_string_free(IntPtr ptr);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_last_error_message();

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern void tt_set_free_word_mode(IntPtr game, uint on);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern void tt_set_reading_direction(IntPtr game, uint rtl);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern void tt_set_stacking(IntPtr game, uint enabled, uint maxHeight, uint forbidSame, uint sumScoring);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_snapshot_state_json(IntPtr game);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint tt_restore_state_json(IntPtr game, IntPtr snapshotJson);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint tt_set_dictionary_from_text(IntPtr game, IntPtr kind, IntPtr text, uint caseFold);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint tt_set_dictionary_from_fst_bytes(IntPtr game, IntPtr bytes, UIntPtr len, uint caseFold);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_generate_moves(IntPtr game, uint maxLen, uint limit);
    }

    public sealed class Engine : IDisposable
    {
        private IntPtr handle = IntPtr.Zero;

        public bool NewGame(string configJson, uint players)
        {
            if (handle != IntPtr.Zero)
            {
                Dispose();
            }
            var cfgPtr = StringToUtf8(configJson);
            try
            {
                handle = Native.tt_new_game(cfgPtr, players);
                return handle != IntPtr.Zero;
            }
            finally
            {
                Marshal.FreeHGlobal(cfgPtr);
            }
        }

        public string? PlayMove(string placementsJson)
        {
            EnsureHandle();
            var pPtr = StringToUtf8(placementsJson);
            try
            {
                var res = Native.tt_play_move(handle, pPtr);
                return TakeString(res);
            }
            finally
            {
                Marshal.FreeHGlobal(pPtr);
            }
        }

        public string? GetBoardJson()
        {
            EnsureHandle();
            var ptr = Native.tt_get_board(handle);
            return TakeString(ptr);
        }

        public string? GetRackJson()
        {
            EnsureHandle();
            var ptr = Native.tt_get_rack(handle);
            return TakeString(ptr);
        }

        public string? GetScoresJson()
        {
            EnsureHandle();
            var ptr = Native.tt_get_scores(handle);
            return TakeString(ptr);
        }

        public string? PreviewMoveJson(string placementsJson)
        {
            EnsureHandle();
            var pPtr = StringToUtf8(placementsJson);
            try
            {
                var res = Native.tt_preview_move(handle, pPtr);
                return TakeString(res);
            }
            finally
            {
                Marshal.FreeHGlobal(pPtr);
            }
        }

        public string? BestMove(string difficulty, ulong? seed = null)
        {
            EnsureHandle();
            var diffPtr = StringToUtf8(difficulty);
            try
            {
                var res = Native.tt_best_move(handle, diffPtr, seed ?? 0, seed.HasValue ? 1u : 0u);
                return TakeString(res);
            }
            finally
            {
                Marshal.FreeHGlobal(diffPtr);
            }
        }

        public bool SetBonuses(string bonusesJson)
        {
            EnsureHandle();
            var ptr = StringToUtf8(bonusesJson);
            try
            {
                var ok = Native.tt_set_bonuses(handle, ptr);
                return ok != 0u;
            }
            finally
            {
                Marshal.FreeHGlobal(ptr);
            }
        }

        public static string? LastError()
        {
            var ptr = Native.tt_last_error_message();
            if (ptr == IntPtr.Zero) return null;
            return Marshal.PtrToStringUTF8(ptr);
        }

        public void Dispose()
        {
            if (handle != IntPtr.Zero)
            {
                Native.tt_free_game(handle);
                handle = IntPtr.Zero;
            }
            GC.SuppressFinalize(this);
        }

        public void SetFreeWordMode(bool on)
        {
            EnsureHandle();
            Native.tt_set_free_word_mode(handle, on ? 1u : 0u);
        }

        public void SetReadingDirection(bool rtl)
        {
            EnsureHandle();
            Native.tt_set_reading_direction(handle, rtl ? 1u : 0u);
        }

        public void SetStacking(bool enabled, uint maxHeight = 7, bool forbidSame = true, bool sumStackScoring = false)
        {
            EnsureHandle();
            Native.tt_set_stacking(handle, enabled ? 1u : 0u, maxHeight, forbidSame ? 1u : 0u, sumStackScoring ? 1u : 0u);
        }

        public string? SnapshotStateJson()
        {
            EnsureHandle();
            var ptr = Native.tt_snapshot_state_json(handle);
            return TakeString(ptr);
        }

        public bool RestoreStateJson(string snapshotJson)
        {
            EnsureHandle();
            var sPtr = StringToUtf8(snapshotJson);
            try
            {
                var ok = Native.tt_restore_state_json(handle, sPtr);
                return ok != 0u;
            }
            finally
            {
                Marshal.FreeHGlobal(sPtr);
            }
        }

        public bool SetDictionaryFromText(string kind, string text, bool caseFold = true)
        {
            EnsureHandle();
            var kindPtr = StringToUtf8(kind);
            var textPtr = StringToUtf8(text);
            try
            {
                var ok = Native.tt_set_dictionary_from_text(handle, kindPtr, textPtr, caseFold ? 1u : 0u);
                return ok != 0u;
            }
            finally
            {
                Marshal.FreeHGlobal(kindPtr);
                Marshal.FreeHGlobal(textPtr);
            }
        }

        public bool SetDictionaryFromFstBytes(byte[] bytes, bool caseFold = true)
        {
            EnsureHandle();
            if (bytes == null || bytes.Length == 0) return false;
            var unmanaged = Marshal.AllocHGlobal(bytes.Length);
            try
            {
                Marshal.Copy(bytes, 0, unmanaged, bytes.Length);
                var ok = Native.tt_set_dictionary_from_fst_bytes(handle, unmanaged, (UIntPtr)bytes.Length, caseFold ? 1u : 0u);
                return ok != 0u;
            }
            finally
            {
                Marshal.FreeHGlobal(unmanaged);
            }
        }

        public string? GenerateMoves(uint maxLen, uint limit)
        {
            EnsureHandle();
            var ptr = Native.tt_generate_moves(handle, maxLen, limit);
            return TakeString(ptr);
        }

        private void EnsureHandle()
        {
            if (handle == IntPtr.Zero)
                throw new InvalidOperationException("Engine not initialized. Call NewGame() first.");
        }

        private static IntPtr StringToUtf8(string s)
        {
            // Allocate unmanaged UTF-8 null-terminated buffer
            var bytes = System.Text.Encoding.UTF8.GetBytes(s);
            var mem = Marshal.AllocHGlobal(bytes.Length + 1);
            Marshal.Copy(bytes, 0, mem, bytes.Length);
            Marshal.WriteByte(mem, bytes.Length, 0);
            return mem;
        }

        private static string? TakeString(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return null;
            try { return Marshal.PtrToStringUTF8(ptr); }
            finally { Native.tt_string_free(ptr); }
        }

        ~Engine() { Dispose(); }
    }
}
