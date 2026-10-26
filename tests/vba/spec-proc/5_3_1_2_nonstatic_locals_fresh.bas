' INV: vbal.5.3.1.2
' SOURCE: spec ms-vbal 5.3.1.2 variables of a non-static procedure have procedure extent
' EXPECT: 1
' EXPECT: 1
Function Tick() As Long
    Dim n As Long
    n = n + 1
    Tick = n
End Function
Public Sub Main()
    Debug.Print CStr(Tick())
    Debug.Print CStr(Tick())
End Sub
