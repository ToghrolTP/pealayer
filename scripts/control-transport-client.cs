// Passive IPC client for the isolated fixture: no mouse/keyboard automation.
using System;
using System.Collections.Concurrent;
using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public sealed class PealayerCopyDataClient : IDisposable {
    const uint CopyData=0x004a;
    const ulong Request=0x504c4a51, Response=0x504c4a52;
    [StructLayout(LayoutKind.Sequential)] struct Copy {public UIntPtr tag;public uint count;public IntPtr data;}
    delegate IntPtr WindowProc(IntPtr window,uint message,IntPtr wparam,IntPtr lparam);
    [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct Class {
        public uint style;public WindowProc procedure;public int extraClass,extraWindow;
        public IntPtr instance,icon,cursor,background;public string menu,name;
    }
    [StructLayout(LayoutKind.Sequential)] struct Message {public IntPtr window;public uint message;public UIntPtr wparam;public IntPtr lparam;public uint time;public int x,y;public uint reserved;}
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern ushort RegisterClassW(ref Class value);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CreateWindowExW(uint extended,string cls,string name,uint style,int x,int y,int width,int height,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr parameter);
    [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern IntPtr GetModuleHandleW(string name);
    [DllImport("user32.dll")] static extern int GetMessageW(out Message message,IntPtr window,uint min,uint max);
    [DllImport("user32.dll")] static extern IntPtr DispatchMessageW(ref Message message);
    [DllImport("user32.dll")] static extern IntPtr DefWindowProcW(IntPtr window,uint message,IntPtr wp,IntPtr lp);
    [DllImport("user32.dll")] static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll")] static extern void PostQuitMessage(int code);
    [DllImport("user32.dll")] static extern bool PostMessageW(IntPtr window,uint message,IntPtr wp,IntPtr lp);
    [DllImport("user32.dll",SetLastError=true)] static extern IntPtr SendMessageTimeoutW(IntPtr window,uint message,IntPtr wp,ref Copy data,uint flags,uint timeout,out IntPtr result);
    readonly ConcurrentQueue<string> replies=new ConcurrentQueue<string>();
    readonly AutoResetEvent ready=new AutoResetEvent(false), arrived=new AutoResetEvent(false);
    readonly Thread thread;
    readonly WindowProc callback;
    IntPtr receiver;
    public PealayerCopyDataClient() {
        callback=OnMessage;
        thread=new Thread(()=>{
            string name="PealayerControlFixture-"+Guid.NewGuid().ToString("N");
            var cls=new Class{procedure=callback,name=name,instance=GetModuleHandleW(null)};
            if(RegisterClassW(ref cls)!=0) receiver=CreateWindowExW(0,name,"",0,0,0,0,0,new IntPtr(-3),IntPtr.Zero,cls.instance,IntPtr.Zero);
            ready.Set();
            if(receiver!=IntPtr.Zero){Message message;while(GetMessageW(out message,IntPtr.Zero,0,0)>0)DispatchMessageW(ref message);}
        });
        thread.IsBackground=true;thread.Start();
        if(!ready.WaitOne(3000)||receiver==IntPtr.Zero)throw new InvalidOperationException("Reply window did not start");
    }
    IntPtr OnMessage(IntPtr window,uint message,IntPtr wp,IntPtr lp) {
        if(message==CopyData) {
            var data=Marshal.PtrToStructure<Copy>(lp);
            if(data.tag.ToUInt64()==Response && data.count<=1048576){var bytes=new byte[data.count];Marshal.Copy(data.data,bytes,0,bytes.Length);replies.Enqueue(Encoding.UTF8.GetString(bytes));arrived.Set();return new IntPtr(1);}
        }
        if(message==0x0010){DestroyWindow(window);return IntPtr.Zero;}
        if(message==0x0002){PostQuitMessage(0);return IntPtr.Zero;}
        return DefWindowProcW(window,message,wp,lp);
    }
    public string Call(long target,string json) {
        string previous;while(replies.TryDequeue(out previous)){} arrived.Reset();
        byte[] bytes=Encoding.UTF8.GetBytes(json);
        var pin=GCHandle.Alloc(bytes,GCHandleType.Pinned);
        try {
            var copy=new Copy{tag=new UIntPtr(Request),count=(uint)bytes.Length,data=pin.AddrOfPinnedObject()};IntPtr accepted;
            if(SendMessageTimeoutW(new IntPtr(target),CopyData,receiver,ref copy,3,1500,out accepted)==IntPtr.Zero||accepted!=new IntPtr(1))throw new IOException("WM_COPYDATA request was not queued: "+Marshal.GetLastWin32Error());
        } finally {pin.Free();}
        if(!arrived.WaitOne(5000)||!replies.TryDequeue(out previous))throw new TimeoutException("WM_COPYDATA JSON response timed out");
        return previous;
    }
    public static string Pipe(string path,string json) {
        string name=path.Substring(path.LastIndexOf('\\')+1);
        using(var pipe=new NamedPipeClientStream(".",name,PipeDirection.InOut,PipeOptions.Asynchronous)) {
            pipe.Connect(3000);
            byte[] bytes=Encoding.UTF8.GetBytes(json+"\n");pipe.Write(bytes,0,bytes.Length);pipe.Flush();
            using(var reader=new StreamReader(pipe,Encoding.UTF8)){
                var read=reader.ReadLineAsync();if(!read.Wait(5000))throw new TimeoutException("Native pipe response timed out");return read.Result;
            }
        }
    }
    public void Dispose(){PostMessageW(receiver,0x0010,IntPtr.Zero,IntPtr.Zero);thread.Join(2000);}
}
