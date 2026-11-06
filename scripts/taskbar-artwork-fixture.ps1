# Read-only shell artwork checks for the isolated native-window fixture. Does
# not activate windows, drive input, restart Explorer, or alter shell settings.
Add-Type -AssemblyName System.Drawing
$drawingReferences=@([Drawing.Bitmap].Assembly.Location,[Drawing.Point].Assembly.Location,'System.Runtime','System.Collections','System.Runtime.InteropServices')
# .NET 10 separates native drawing interfaces into these assemblies.
$drawingReferences+=@(Get-ChildItem -LiteralPath $PSHOME -Filter 'System.Private.Windows.*.dll' | ForEach-Object FullName)
Add-Type -ReferencedAssemblies $drawingReferences -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
public static class PealayerTaskbarArtwork {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int L,T,R,B; }
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindowW(string c,string n);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindowExW(IntPtr p,IntPtr a,string c,string n);
 [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
 public static Bitmap Capture() {
  SetProcessDPIAware();
  var bar=FindWindowW("Shell_TrayWnd",null); Rect r;
  if(bar==IntPtr.Zero || !GetWindowRect(bar,out r) || r.R-r.L<100 || r.B-r.T<20 || r.B-r.T>200)
   throw new Exception("A visible horizontal primary taskbar is required; no visual pass inferred.");
  var tray=FindWindowExW(bar,IntPtr.Zero,"TrayNotifyWnd",null); Rect t;
  if(tray!=IntPtr.Zero && GetWindowRect(tray,out t) && t.L>r.L) r.R=Math.Min(r.R,t.L);
  var b=new Bitmap(r.R-r.L,r.B-r.T,PixelFormat.Format32bppArgb);
  using(var g=Graphics.FromImage(b))g.CopyFromScreen(r.L,r.T,0,0,b.Size);
  return b;
 }
 public static double[] Match(Bitmap capture,string iconPath,int[] fixedBox) {
  var data=capture.LockBits(new Rectangle(0,0,capture.Width,capture.Height),ImageLockMode.ReadOnly,PixelFormat.Format32bppArgb);
  byte[] pixels=new byte[data.Stride*capture.Height]; Marshal.Copy(data.Scan0,pixels,0,pixels.Length);
  capture.UnlockBits(data);
  double best=Double.MaxValue; int bx=0,by=0,bs=0;
  using(var icon=new Icon(iconPath,256,256))using(var source=icon.ToBitmap()) {
   for(int side=24;side<=Math.Min(64,capture.Height);side+=2) {
    if(fixedBox!=null && side!=fixedBox[2])continue;
    using(var sample=new Bitmap(side,side,PixelFormat.Format32bppArgb)) {
     using(var g=Graphics.FromImage(sample)){g.InterpolationMode=InterpolationMode.HighQualityBicubic;g.DrawImage(source,0,0,side,side);}
     var points=new System.Collections.Generic.List<int[]>();
     for(int y=2;y<side-2;y+=3)for(int x=2;x<side-2;x+=3){var c=sample.GetPixel(x,y);if(c.A>245)points.Add(new int[]{x,y,c.B,c.G,c.R});}
     if(points.Count<20)continue;
     int x0=fixedBox==null?0:fixedBox[0],x1=fixedBox==null?capture.Width-side:fixedBox[0];
     int y0=fixedBox==null?0:fixedBox[1],y1=fixedBox==null?capture.Height-side:fixedBox[1];
     for(int y=y0;y<=y1;y++)for(int x=x0;x<=x1;x++) {
      if(x<0||y<0||x+side>capture.Width||y+side>capture.Height)continue;
      double sum=0; int count=0;
      // Evenly distributed probes reject unrelated buttons cheaply.
      for(int n=0;n<points.Count+8;n++) {
       int j=n<8?n*(points.Count-1)/7:n-8;
       var p=points[j];int offset=(y+p[1])*data.Stride+(x+p[0])*4;
       for(int c=0;c<3;c++){int d=pixels[offset+c]-p[c+2];sum+=d*d;count++;}
       if(fixedBox==null && n==7 && sum/count>2500){sum=Double.MaxValue;break;}
      }
      double score=sum/count;
      if(score<best){best=score;bx=x;by=y;bs=side;}
     }
    }
   }
  }
  return new double[]{Math.Sqrt(best),bx,by,bs};
 }
}
'@

function Assert-TaskbarArtwork {
    param([string]$IconPath,[string]$OtherIconPath,[int[]]$Box,[string]$OutputPath)
    $capture=[PealayerTaskbarArtwork]::Capture()
    try {
        $match=[PealayerTaskbarArtwork]::Match($capture,$IconPath,$Box)
        $targetBox=@([int]$match[1],[int]$match[2],[int]$match[3])
        $other=[PealayerTaskbarArtwork]::Match($capture,$OtherIconPath,$targetBox)
        $crop=$capture.Clone([Drawing.Rectangle]::new($targetBox[0],$targetBox[1],$targetBox[2],$targetBox[2]),$capture.PixelFormat)
        try{$crop.Save($OutputPath,[Drawing.Imaging.ImageFormat]::Png)}finally{$crop.Dispose()}
        # Explorer can select the ICO's size-tuned native frame rather than our
        # resampled 256px master. Permit that difference, but require a clear
        # margin over the other design in this same, fixed taskbar slot.
        if($match[0] -gt 30 -or $other[0] -lt ($match[0]+12)) {
            throw "Taskbar artwork mismatch: expected RMSE=$($match[0]), other=$($other[0]); HWND/property readback is not visible acceptance."
        }
        Write-Host "Displayed taskbar artwork matched (RMSE=$([Math]::Round($match[0],2))); alternate rejected ($([Math]::Round($other[0],2)))."
        return ,$targetBox
    } finally {$capture.Dispose()}
}
