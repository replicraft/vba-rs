' INV: vbal.5.2.1.3
' SOURCE: spec ms-vbal 5.2.1.3 explicit-mode accepts declared variables
' EXPECT: v=5
Option Explicit
Dim m As Long
Public Sub Main()
    Dim x As Long
    x = 2
    m = 3
    Debug.Print "v=" & CStr(x + m)
End Sub
