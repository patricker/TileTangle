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
        public static extern void tt_string_free(IntPtr ptr);

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr tt_last_error_message();

        [DllImport(LIB, CallingConvention = CallingConvention.Cdecl)]
        public static extern void tt_set_free_word_mode(IntPtr game, uint on);
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
